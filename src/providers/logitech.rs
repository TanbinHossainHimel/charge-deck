//! Logitech HID++ 2.0 (Lightspeed, Bolt, Unifying receivers and direct USB).
//! UNTESTED: please report.
//!
//! Protocol facts come from Logitech's public HID++ 2.0 feature descriptions
//! and the MIT-licensed libratbag docs. Solaar is GPL, so only facts were
//! learned from it and no code was copied.
//!
//! A request is a "long" report (20 bytes):
//!   [0x11, device index, feature index, (function << 4) | software id, params...]
//! The ROOT feature (index 0) function 1 answers "which index has feature X?".
//! Then we call the battery feature of that index.
//!
//! Safety: only VID 0x046D, only the HID++ vendor collections (rule 4), and
//! only "get" functions: feature lookup, battery status, device name/type.

use super::Provider;
use crate::hid_util::{flush, open_if_allowed, read_until, write_report};
use crate::model::{DeviceBattery, DeviceKind, Reading, Status};
use hidapi::{DeviceInfo, HidApi, HidDevice};

const LOGI_VID: u16 = 0x046D;
const SOURCE: &str = "Logitech";
const LONG: u8 = 0x11;
const SHORT: u8 = 0x10;
/// Software id: any value 1..=15 (0 is used by device notifications).
const SW_ID: u8 = 0x0A;
const TIMEOUT_MS: u64 = 300;

const FEATURE_UNIFIED_BATTERY: u16 = 0x1004;
const FEATURE_BATTERY_STATUS: u16 = 0x1000;
const FEATURE_BATTERY_VOLTAGE: u16 = 0x1001;
const FEATURE_DEVICE_NAME: u16 = 0x0005;

/// Build a long request.
pub fn build_request(device_index: u8, feature_index: u8, function: u8, params: &[u8]) -> [u8; 20] {
    let mut r = [0u8; 20];
    r[0] = LONG;
    r[1] = device_index;
    r[2] = feature_index;
    r[3] = (function << 4) | SW_ID;
    r[4..4 + params.len()].copy_from_slice(params);
    r
}

/// Answer to one of our requests.
#[derive(Debug, PartialEq)]
pub enum Reply {
    /// Parameters (bytes after the 4-byte header).
    Ok(Vec<u8>),
    /// The device or receiver answered with an error.
    Error,
}

/// Is this report the answer to (device_index, feature_index, function)?
/// None = unrelated report (keep listening).
pub fn parse_reply(r: &[u8], device_index: u8, feature_index: u8, function: u8) -> Option<Reply> {
    if r.len() < 7 || (r[0] != SHORT && r[0] != LONG) || r[1] != device_index {
        return None;
    }
    // HID++ 2.0 error: [id, idx, 0xFF, feature index, function|swid, error]
    if r[2] == 0xFF && r[3] == feature_index && r[4] == ((function << 4) | SW_ID) {
        return Some(Reply::Error);
    }
    // HID++ 1.0 error (receiver says "no device there"): [id, idx, 0x8F, ...]
    if r[2] == 0x8F {
        return Some(Reply::Error);
    }
    if r[2] == feature_index && r[3] == ((function << 4) | SW_ID) {
        return Some(Reply::Ok(r[4..].to_vec()));
    }
    None
}

/// Parsed battery numbers before turning them into a `Reading`.
#[derive(Debug, PartialEq)]
pub struct Battery {
    pub percent: u8,
    pub estimated: bool,
    pub voltage_mv: Option<u32>,
    pub charging: bool,
}

/// 0x1004 Unified Battery, function 1 (GetStatus):
/// params = [percentage, level flags, charging status, external power]
pub fn parse_unified(p: &[u8]) -> Option<Battery> {
    let (pct, flags, status) = (*p.first()?, *p.get(1)?, *p.get(2)?);
    let charging = matches!(status, 1 | 2);
    if pct > 0 {
        return Some(Battery { percent: pct.min(100), estimated: false, voltage_mv: None, charging });
    }
    // Some devices only report coarse levels: critical 1, low 2, good 4, full 8.
    let percent = match flags {
        f if f & 8 != 0 => 100,
        f if f & 4 != 0 => 60,
        f if f & 2 != 0 => 20,
        f if f & 1 != 0 => 5,
        _ => return None,
    };
    Some(Battery { percent, estimated: true, voltage_mv: None, charging })
}

/// 0x1000 Battery Status, function 0 (GetBatteryLevelStatus):
/// params = [level %, next level %, status]
pub fn parse_battery_status(p: &[u8]) -> Option<Battery> {
    let (level, status) = (*p.first()?, *p.get(2)?);
    Some(Battery {
        percent: level.min(100),
        estimated: false,
        voltage_mv: None,
        charging: matches!(status, 1 | 2 | 4),
    })
}

/// 0x1001 Battery Voltage, function 0: params = [mV high, mV low, flags]
pub fn parse_voltage(p: &[u8]) -> Option<Battery> {
    let mv = (*p.first()? as u32) << 8 | *p.get(1)? as u32;
    let flags = *p.get(2)?;
    if !(2500..=4500).contains(&mv) {
        return None;
    }
    Some(Battery {
        percent: voltage_to_percent(mv),
        estimated: true,
        voltage_mv: Some(mv),
        charging: flags & 0x80 != 0,
    })
}

/// Rough Li-ion curve (our own approximation, so shown with "≈").
pub fn voltage_to_percent(mv: u32) -> u8 {
    const CURVE: [(u32, u32); 8] =
        [(3300, 0), (3600, 10), (3700, 25), (3800, 50), (3900, 70), (4000, 85), (4100, 95), (4200, 100)];
    if mv <= CURVE[0].0 {
        return 0;
    }
    for w in CURVE.windows(2) {
        let ((v0, p0), (v1, p1)) = (w[0], w[1]);
        if mv < v1 {
            return (p0 + (mv - v0) * (p1 - p0) / (v1 - v0)) as u8;
        }
    }
    100
}

/// Device type from feature 0x0005 function 2.
pub fn kind_from_type(t: u8) -> DeviceKind {
    match t {
        0 | 2 => DeviceKind::Keyboard,
        3..=5 => DeviceKind::Mouse,
        8 => DeviceKind::Headset,
        11 | 12 => DeviceKind::Controller,
        _ => DeviceKind::Other,
    }
}

struct Link<'a> {
    dev: &'a HidDevice,
}

impl Link<'_> {
    /// One request/answer round trip. None = no answer in time.
    fn call(&self, idx: u8, feature_index: u8, function: u8, params: &[u8]) -> Option<Reply> {
        flush(self.dev);
        // READ-ONLY: only "get" functions (feature lookup, battery, name, type).
        if !write_report(self.dev, &build_request(idx, feature_index, function, params)) {
            return None;
        }
        read_until(self.dev, TIMEOUT_MS, |r| parse_reply(r, idx, feature_index, function))
    }

    /// Root feature, function 1: where is `feature_id`? Some(0) = unsupported.
    fn feature_index(&self, idx: u8, feature_id: u16) -> Option<Option<u8>> {
        match self.call(idx, 0x00, 1, &feature_id.to_be_bytes())? {
            Reply::Ok(p) => Some(p.first().copied().filter(|&i| i != 0)),
            Reply::Error => Some(None),
        }
    }

    fn ok(&self, idx: u8, fi: u8, function: u8, params: &[u8]) -> Option<Vec<u8>> {
        match self.call(idx, fi, function, params)? {
            Reply::Ok(p) => Some(p),
            Reply::Error => None,
        }
    }

    fn battery(&self, idx: u8) -> Option<Battery> {
        // Newest first; fall back to older battery features.
        match self.feature_index(idx, FEATURE_UNIFIED_BATTERY)? {
            Some(fi) => return parse_unified(&self.ok(idx, fi, 1, &[])?),
            None => {}
        }
        if let Some(fi) = self.feature_index(idx, FEATURE_BATTERY_STATUS)? {
            return parse_battery_status(&self.ok(idx, fi, 0, &[])?);
        }
        let fi = self.feature_index(idx, FEATURE_BATTERY_VOLTAGE)??;
        parse_voltage(&self.ok(idx, fi, 0, &[])?)
    }

    /// Device name and kind from feature 0x0005 (both optional).
    fn name_and_kind(&self, idx: u8) -> (Option<String>, DeviceKind) {
        let Some(Some(fi)) = self.feature_index(idx, FEATURE_DEVICE_NAME) else {
            return (None, DeviceKind::Other);
        };
        let kind = self.ok(idx, fi, 2, &[]).and_then(|p| p.first().copied()).map(kind_from_type).unwrap_or(DeviceKind::Other);
        let count = self.ok(idx, fi, 0, &[]).and_then(|p| p.first().copied()).unwrap_or(0) as usize;
        let mut name = Vec::new();
        while name.len() < count.min(48) {
            let Some(p) = self.ok(idx, fi, 1, &[name.len() as u8]) else { break };
            let chunk: Vec<u8> = p.into_iter().take_while(|&b| b != 0).collect();
            if chunk.is_empty() {
                break;
            }
            name.extend(chunk);
        }
        let name = String::from_utf8_lossy(&name).trim().to_string();
        ((!name.is_empty()).then_some(name), kind)
    }
}

pub struct Logitech;

impl Logitech {
    /// HID++ collections: usage page 0xFF00 usage 1 (short) / 2 (long),
    /// or 0xFF43 usage 0x0202 (direct / Bluetooth LE devices).
    fn is_ours(info: &DeviceInfo) -> bool {
        info.vendor_id() == LOGI_VID
            && matches!(
                (info.usage_page(), info.usage()),
                (0xFF00, 1) | (0xFF00, 2) | (0xFF43, 0x0202)
            )
    }

    fn is_receiver(info: &DeviceInfo) -> bool {
        info.product_string().is_some_and(|p| p.to_lowercase().contains("receiver"))
    }
}

impl Provider for Logitech {
    fn name(&self) -> &'static str {
        SOURCE
    }

    fn claims(&self, info: &DeviceInfo) -> bool {
        Logitech::is_ours(info)
    }

    fn scan(&mut self, hid: &HidApi) -> Vec<DeviceBattery> {
        // Use only the long collection per physical device (our requests are long reports).
        let mut chosen: Vec<&DeviceInfo> = Vec::new();
        for info in hid.device_list().filter(|i| Logitech::is_ours(i)) {
            let is_long = (info.usage_page(), info.usage()) != (0xFF00, 1);
            if !is_long {
                continue;
            }
            let key = (info.product_id(), info.serial_number().unwrap_or(""));
            if !chosen.iter().any(|c| (c.product_id(), c.serial_number().unwrap_or("")) == key) {
                chosen.push(info);
            }
        }

        let mut out = Vec::new();
        for info in chosen {
            let Some(dev) = open_if_allowed(hid, info) else { continue };
            let link = Link { dev: &dev };
            // Receivers hold up to 6 devices (index 1..=6); direct devices use 0xFF.
            let indices: Vec<u8> = if Logitech::is_receiver(info) { (1..=6).collect() } else { vec![0xFF] };
            for idx in indices {
                let Some(b) = link.battery(idx) else { continue };
                let (name, kind) = link.name_and_kind(idx);
                let fallback = info.product_string().unwrap_or("Logitech device").to_string();
                out.push(DeviceBattery {
                    id: format!("logi:{:04x}:{}:{idx}", info.product_id(), info.serial_number().unwrap_or("")),
                    name: name.unwrap_or(fallback),
                    kind,
                    source: SOURCE,
                    status: Status::Ok(Reading {
                        percent: b.percent,
                        estimated: b.estimated,
                        voltage_mv: b.voltage_mv,
                        charging: b.charging,
                    }),
                });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_feature_lookup() {
        // Ask root (index 0) function 1 about feature 0x1004 on device 1.
        let r = build_request(1, 0x00, 1, &[0x10, 0x04]);
        assert_eq!(&r[..6], &[0x11, 0x01, 0x00, 0x1A, 0x10, 0x04]);
        assert_eq!(r.len(), 20);
    }

    #[test]
    fn parses_lookup_reply_and_errors() {
        let ok = [0x11, 1, 0x00, 0x1A, 0x07, 0x00, 0x02, 0, 0, 0];
        assert_eq!(parse_reply(&ok, 1, 0x00, 1), Some(Reply::Ok(vec![0x07, 0, 2, 0, 0, 0])));
        let err = [0x11, 1, 0xFF, 0x00, 0x1A, 0x02, 0, 0, 0, 0];
        assert_eq!(parse_reply(&err, 1, 0x00, 1), Some(Reply::Error));
        let no_dev = [0x10, 1, 0x8F, 0x00, 0x1A, 0x09, 0];
        assert_eq!(parse_reply(&no_dev, 1, 0x00, 1), Some(Reply::Error));
        // other device index / other feature: ignored
        assert_eq!(parse_reply(&ok, 2, 0x00, 1), None);
        assert_eq!(parse_reply(&ok, 1, 0x05, 1), None);
    }

    #[test]
    fn unified_battery() {
        let b = parse_unified(&[87, 4, 0, 0]).unwrap();
        assert_eq!((b.percent, b.estimated, b.charging), (87, false, false));
        let b = parse_unified(&[55, 4, 1, 1]).unwrap();
        assert!(b.charging);
        // percentage 0 but "good" flag: approximate
        let b = parse_unified(&[0, 4, 0, 0]).unwrap();
        assert_eq!((b.percent, b.estimated), (60, true));
        assert_eq!(parse_unified(&[0, 0, 0, 0]), None);
    }

    #[test]
    fn battery_status_and_voltage() {
        let b = parse_battery_status(&[40, 30, 1]).unwrap();
        assert_eq!((b.percent, b.charging), (40, true));
        // 4000 mV = 0x0FA0, not charging
        let b = parse_voltage(&[0x0F, 0xA0, 0x00]).unwrap();
        assert_eq!((b.percent, b.voltage_mv, b.estimated, b.charging), (85, Some(4000), true, false));
        assert_eq!(parse_voltage(&[0x00, 0x10, 0]), None); // nonsense voltage
        assert_eq!(voltage_to_percent(3300), 0);
        assert_eq!(voltage_to_percent(4300), 100);
    }

    #[test]
    fn device_types() {
        assert_eq!(kind_from_type(3), DeviceKind::Mouse);
        assert_eq!(kind_from_type(0), DeviceKind::Keyboard);
        assert_eq!(kind_from_type(8), DeviceKind::Headset);
        assert_eq!(kind_from_type(99), DeviceKind::Other);
    }
}
