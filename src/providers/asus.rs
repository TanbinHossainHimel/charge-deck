//! ASUS ROG keyboards and mice (2.4 GHz receivers or cable).
//!
//! Protocol facts (byte meanings) come from public descriptions of the ASUS
//! vendor protocol; the code below is our own. All three queries are the same
//! READ-ONLY "get battery" command (0x12) and change no setting.
//!
//! Safety: we only touch devices whose VID is ASUS (0x0B05) AND whose product
//! name contains "ROG" AND only vendor collections (usage page 0xFF00+).
//! That keeps us away from e.g. the "AURA LED Controller" on the motherboard.

use super::Provider;
use crate::hid_util::{flush, open_if_allowed, read_until, write_report};
use crate::model::{DeviceBattery, DeviceKind, Reading, Status};
use hidapi::{DeviceInfo, HidApi, HidDevice};

const ASUS_VID: u16 = 0x0B05;
const SOURCE: &str = "ASUS receiver";

#[derive(Clone, Copy, Debug, PartialEq)]
enum Query {
    /// Keyboard behind an Omni receiver: usage page 0xFF00, report ID 0x02.
    Keyboard,
    /// Mouse behind an Omni receiver: usage page 0xFF01, report ID 0x03.
    OmniMouse,
    /// Mouse on a classic receiver or cable: no report ID.
    ClassicMouse,
}

/// The raw numbers inside an answer.
#[derive(Debug, PartialEq)]
struct Raw {
    value: u8,
    charging: bool,
}

impl Query {
    /// The bytes we send (first byte = report ID, 0 when there is none).
    fn request(self) -> &'static [u8] {
        match self {
            Query::Keyboard => &[0x02, 0x12, 0x01],
            Query::OmniMouse => &[0x03, 0x12, 0x07],
            Query::ClassicMouse => &[0x00, 0x12, 0x07],
        }
    }

    fn kind(self) -> DeviceKind {
        match self {
            Query::Keyboard => DeviceKind::Keyboard,
            _ => DeviceKind::Mouse,
        }
    }

    /// Read the answer. Returns None when the echo doesn't match
    /// (some other report, e.g. from a different command) so the caller
    /// keeps listening.
    fn parse(self, r: &[u8]) -> Option<Raw> {
        // (echo bytes, index of the value, index of the charging flag)
        let (echo, val_i, chg_i): (&[u8], usize, usize) = match self {
            Query::Keyboard => (&[0x02, 0x12, 0x01], 6, 9),
            Query::OmniMouse => (&[0x03, 0x12, 0x07], 5, 10),
            // Report ID 0 is stripped by hidapi, so the answer starts at 0x12.
            Query::ClassicMouse => (&[0x12, 0x07], 4, 9),
        };
        // Seen on a real Omni receiver when the device behind it is asleep or
        // not reachable: `<report id> ff aa 00 00 ...`. Treat as "asleep".
        let id = echo[0];
        if id != 0x12 && r.len() > 2 && r[0] == id && r[1] == 0xFF && r[2] == 0xAA {
            return Some(Raw { value: 0, charging: false });
        }
        if !r.starts_with(echo) || r.len() <= val_i {
            return None;
        }
        Some(Raw { value: r[val_i], charging: r.get(chg_i) == Some(&1) })
    }
}

/// Older mice report a level 0..=4 instead of a percentage.
fn uses_level_scale(product: &str) -> bool {
    let p = product.to_uppercase();
    p.contains("CHAKRAM") || (p.contains("KERIS") && p.contains("WIRELESS") && !p.contains("AIMPOINT"))
}

/// Turn raw numbers into a card status. Value 0 means the receiver answered
/// but the device is asleep or off.
fn to_status(raw: Raw, level_scale: bool) -> Status {
    if raw.value == 0 {
        return Status::Asleep;
    }
    if level_scale {
        // 1..=4 levels -> 25..100 %, shown as approximate.
        Status::Ok(Reading::estimated((raw.value.min(4)) * 25, raw.charging))
    } else {
        Status::Ok(Reading::exact(raw.value, raw.charging))
    }
}

pub struct Asus;

impl Asus {
    fn is_ours(info: &DeviceInfo) -> bool {
        info.vendor_id() == ASUS_VID
            && info.usage_page() >= 0xFF00
            && info.product_string().is_some_and(|p| p.to_uppercase().contains("ROG"))
    }
}

impl Provider for Asus {
    fn name(&self) -> &'static str {
        SOURCE
    }

    fn claims(&self, info: &DeviceInfo) -> bool {
        Asus::is_ours(info)
    }

    fn scan(&mut self, hid: &HidApi) -> Vec<DeviceBattery> {
        let mut out: Vec<DeviceBattery> = Vec::new();
        for info in hid.device_list().filter(|i| Asus::is_ours(i)) {
            // Rule 6: an Omni receiver gets ONLY its two documented queries, on
            // its two documented collections. The classic (no report ID) query
            // is only for other ROG devices, never for the Omni's other pages.
            let is_omni = info.product_string().is_some_and(|p| p.to_uppercase().contains("OMNI"));
            let queries: &[Query] = match (is_omni, info.usage_page()) {
                (true, 0xFF00) => &[Query::Keyboard],
                (true, 0xFF01) => &[Query::OmniMouse],
                (true, _) => &[],
                (false, _) => &[Query::ClassicMouse],
            };
            if queries.is_empty() {
                continue;
            }
            let Some(dev) = open_if_allowed(hid, info) else { continue };
            for q in queries {
                if let Some(card) = ask(&dev, info, *q) {
                    if !out.iter().any(|d| d.id == card.id) {
                        out.push(card);
                    }
                    break;
                }
            }
        }
        out
    }
}

/// Ask one collection one question. Opens nothing itself; the caller opened it.
fn ask(dev: &HidDevice, info: &DeviceInfo, q: Query) -> Option<DeviceBattery> {
    flush(dev);
    // READ-ONLY: `request()` is only the battery query (command 0x12).
    if !write_report(dev, q.request()) {
        return None; // wrong collection for this query: try the next one
    }
    let raw = read_until(dev, 600, |r| q.parse(r))?;

    let product = info.product_string().unwrap_or("ROG device");
    let is_receiver = product.to_uppercase().contains("RECEIVER");
    let role = if q.kind() == DeviceKind::Keyboard { "keyboard" } else { "mouse" };
    // Behind a receiver we can't see the real model, so name it by its role.
    let name = if is_receiver {
        if q.kind() == DeviceKind::Keyboard { "ROG Keyboard".to_string() } else { "ROG Mouse".to_string() }
    } else {
        product.to_string()
    };
    let serial = info.serial_number().filter(|s| !s.is_empty()).unwrap_or("");
    Some(DeviceBattery {
        id: format!("asus:{:04x}:{:04x}:{serial}:{role}", info.vendor_id(), info.product_id()),
        name,
        kind: q.kind(),
        source: SOURCE,
        status: to_status(raw, uses_level_scale(product)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyboard_answer() {
        let r = [0x02, 0x12, 0x01, 0, 0, 0, 77, 0, 0, 1, 0];
        let raw = Query::Keyboard.parse(&r).unwrap();
        assert_eq!(raw, Raw { value: 77, charging: true });
        assert_eq!(to_status(raw, false), Status::Ok(Reading::exact(77, true)));
    }

    #[test]
    fn omni_mouse_answer() {
        let r = [0x03, 0x12, 0x07, 0, 0, 55, 0, 0, 0, 0, 0];
        assert_eq!(Query::OmniMouse.parse(&r), Some(Raw { value: 55, charging: false }));
    }

    #[test]
    fn classic_mouse_real_capture() {
        let r = [0x12, 0x07, 0x00, 0x00, 0x58, 0x00, 0x14, 0xd4, 0x0f, 0x00];
        let raw = Query::ClassicMouse.parse(&r).unwrap();
        assert_eq!(raw.value, 88); // 0x58
        assert!(!raw.charging);
    }

    #[test]
    fn wrong_echo_is_ignored() {
        let r = [0x02, 0x99, 0x01, 0, 0, 0, 50, 0, 0, 0];
        assert_eq!(Query::Keyboard.parse(&r), None);
        let r = [0x03, 0x12, 0x01, 0, 0, 50, 0, 0, 0, 0, 0]; // keyboard echo on mouse query
        assert_eq!(Query::OmniMouse.parse(&r), None);
    }

    #[test]
    fn receiver_ff_aa_reply_means_asleep() {
        // Real capture from an Omni receiver while the mouse was idle.
        let r = [0x03, 0xff, 0xaa, 0, 0, 0, 0, 0, 0, 0, 0];
        let raw = Query::OmniMouse.parse(&r).unwrap();
        assert_eq!(to_status(raw, false), Status::Asleep);
    }

    #[test]
    fn zero_percent_means_asleep() {
        let r = [0x02, 0x12, 0x01, 0, 0, 0, 0, 0, 0, 0];
        let raw = Query::Keyboard.parse(&r).unwrap();
        assert_eq!(to_status(raw, false), Status::Asleep);
    }

    #[test]
    fn old_models_report_levels() {
        assert!(uses_level_scale("ROG Chakram"));
        assert!(uses_level_scale("ROG Keris Wireless"));
        assert!(!uses_level_scale("ROG Keris II Ace"));
        let st = to_status(Raw { value: 3, charging: false }, true);
        assert_eq!(st, Status::Ok(Reading::estimated(75, false)));
    }
}
