//! Razer mice using the standard 90-byte feature report (UNTESTED: please report).
//!
//! Protocol facts (layout, command numbers, product IDs) were learned from the
//! public OpenRazer documentation/headers. OpenRazer is GPL, so NO code was
//! copied; everything below is our own.
//!
//! The report (90 bytes):
//!   0      status (0 when we send; 2 = "success" in the answer)
//!   1      transaction id (0x1F, 0xFF or 0x3F depending on the device)
//!   2-3    remaining packets (0)
//!   4      protocol type (0)
//!   5      data size
//!   6      command class
//!   7      command id
//!   8-87   arguments
//!   88     checksum = XOR of bytes 2..=87
//!   89     reserved
//! Commands used (class 0x07): 0x80 = battery level (0-255), 0x84 = charging.
//!
//! Safety: only VID 0x1532 + a PID in `KNOWN` below, only collections that
//! rule 4 allows. If a mouse offers this report ONLY on its normal mouse
//! collection (usage page 0x01), we do not open it - by design.

use super::Provider;
use crate::hid_util::{flush, is_allowed_usage_page, open_if_allowed, trace_rx, trace_tx};
use crate::model::{DeviceBattery, DeviceKind, Reading, Status};
use hidapi::{DeviceInfo, HidApi, HidDevice};
use std::thread::sleep;
use std::time::Duration;

const RAZER_VID: u16 = 0x1532;
const SOURCE: &str = "Razer dongle";
const REPORT_LEN: usize = 90;

const CLASS_POWER: u8 = 0x07;
const CMD_BATTERY_LEVEL: u8 = 0x80;
const CMD_CHARGING: u8 = 0x84;

/// Transaction ids seen on different Razer generations. They only tell the
/// device which "slot" the answer belongs to; a wrong one just gets no answer.
const TRANSACTION_IDS: [u8; 3] = [0x1F, 0xFF, 0x3F];

/// Wireless-capable Razer mice and their receivers.
/// (product id, name). Add more here once someone confirms them.
const KNOWN: &[(u16, &str)] = &[
    (0x0045, "Razer Mamba Wireless"),
    (0x0072, "Razer Mamba Wireless"),
    (0x0073, "Razer Mamba Wireless"),
    (0x005A, "Razer Lancehead Wireless"),
    (0x006F, "Razer Lancehead Wireless"),
    (0x0070, "Razer Lancehead Wireless"),
    (0x0077, "Razer Pro Click"),
    (0x0080, "Razer Pro Click"),
    (0x007A, "Razer Viper Ultimate"),
    (0x007B, "Razer Viper Ultimate"),
    (0x007C, "Razer DeathAdder V2 Pro"),
    (0x007D, "Razer DeathAdder V2 Pro"),
    (0x0083, "Razer Basilisk X HyperSpeed"),
    (0x0086, "Razer Basilisk Ultimate"),
    (0x0088, "Razer Basilisk Ultimate"),
    (0x008F, "Razer Naga Pro"),
    (0x0090, "Razer Naga Pro"),
    (0x0094, "Razer Orochi V2"),
    (0x009A, "Razer Pro Click Mini"),
    (0x009C, "Razer DeathAdder V2 X HyperSpeed"),
    (0x009E, "Razer Viper Mini SE"),
    (0x009F, "Razer Viper Mini SE"),
    (0x00A5, "Razer Viper V2 Pro"),
    (0x00A6, "Razer Viper V2 Pro"),
    (0x00A7, "Razer Naga V2 Pro"),
    (0x00A8, "Razer Naga V2 Pro"),
    (0x00AA, "Razer Basilisk V3 Pro"),
    (0x00AB, "Razer Basilisk V3 Pro"),
    (0x00AF, "Razer Cobra Pro"),
    (0x00B0, "Razer Cobra Pro"),
    (0x00B3, "Razer HyperPolling dongle"),
    (0x00B4, "Razer Naga V2 HyperSpeed"),
    (0x00B6, "Razer DeathAdder V3 Pro"),
    (0x00B7, "Razer DeathAdder V3 Pro"),
    (0x00B8, "Razer Viper V3 HyperSpeed"),
    (0x00B9, "Razer Basilisk V3 X HyperSpeed"),
    (0x00BE, "Razer DeathAdder V4 Pro"),
    (0x00BF, "Razer DeathAdder V4 Pro"),
    (0x00C0, "Razer Viper V3 Pro"),
    (0x00C1, "Razer Viper V3 Pro"),
    (0x00C2, "Razer DeathAdder V3 Pro"),
];

fn known_name(pid: u16) -> Option<&'static str> {
    KNOWN.iter().find(|(p, _)| *p == pid).map(|(_, n)| *n)
}

/// XOR of bytes 2..=87.
pub fn checksum(report: &[u8; REPORT_LEN]) -> u8 {
    report[2..88].iter().fold(0, |a, b| a ^ b)
}

/// Build a request with `size` argument bytes (all zero: we only ask).
pub fn build_request(transaction_id: u8, class: u8, command: u8, size: u8) -> [u8; REPORT_LEN] {
    let mut r = [0u8; REPORT_LEN];
    r[1] = transaction_id;
    r[5] = size;
    r[6] = class;
    r[7] = command;
    r[88] = checksum(&r);
    r
}

/// Check an answer and return argument byte 1 (the value) when it is a
/// successful answer to (class, command) with a correct checksum.
pub fn parse_response(resp: &[u8], class: u8, command: u8) -> Option<u8> {
    // hidapi puts the (zero) report id in front on some Windows setups.
    let body = if resp.len() == REPORT_LEN + 1 { &resp[1..] } else { resp };
    let r: &[u8; REPORT_LEN] = body.get(..REPORT_LEN)?.try_into().ok()?;
    if r[0] != 0x02 || r[6] != class || r[7] != command || r[88] != checksum(r) {
        return None;
    }
    Some(r[9])
}

fn level_to_percent(level: u8) -> u8 {
    ((level as u32 * 100 + 127) / 255) as u8
}

/// Send one read-only query and read the answer.
fn query(dev: &HidDevice, tx: u8, command: u8) -> Option<u8> {
    flush(dev);
    let req = build_request(tx, CLASS_POWER, command, 0x02);
    let mut out = [0u8; REPORT_LEN + 1]; // byte 0 = report id 0
    out[1..].copy_from_slice(&req);
    trace_tx(&out);
    // READ-ONLY: class 0x07 battery level / charging status query.
    dev.send_feature_report(&out).ok()?;
    sleep(Duration::from_millis(60));
    let mut buf = [0u8; REPORT_LEN + 1];
    let n = dev.get_feature_report(&mut buf).ok()?;
    trace_rx(&buf[..n]);
    parse_response(&buf[..n], CLASS_POWER, command)
}

pub struct RazerMouse;

impl RazerMouse {
    fn is_ours(info: &DeviceInfo) -> bool {
        info.vendor_id() == RAZER_VID
            && known_name(info.product_id()).is_some()
            && is_allowed_usage_page(info.usage_page())
    }
}

impl Provider for RazerMouse {
    /// Unique per provider (the deck tracks "device went missing" by this name).
    fn name(&self) -> &'static str {
        "Razer mouse"
    }

    fn claims(&self, info: &DeviceInfo) -> bool {
        RazerMouse::is_ours(info)
    }

    fn scan(&mut self, hid: &HidApi) -> Vec<DeviceBattery> {
        let mut out: Vec<DeviceBattery> = Vec::new();
        for info in hid.device_list().filter(|i| RazerMouse::is_ours(i)) {
            let pid = info.product_id();
            let id = format!("razer-mouse:{pid:04x}");
            if out.iter().any(|d| d.id == id) {
                continue;
            }
            let Some(dev) = open_if_allowed(hid, info) else { continue };
            for tx in TRANSACTION_IDS {
                let Some(level) = query(&dev, tx, CMD_BATTERY_LEVEL) else { continue };
                let charging = query(&dev, tx, CMD_CHARGING) == Some(1);
                let status = if level == 0 {
                    Status::Asleep
                } else {
                    Status::Ok(Reading::exact(level_to_percent(level), charging))
                };
                out.push(DeviceBattery {
                    id,
                    name: known_name(pid).unwrap_or("Razer mouse").to_string(),
                    kind: DeviceKind::Mouse,
                    source: SOURCE,
                    status,
                });
                break;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_layout_and_crc() {
        let r = build_request(0x1F, 0x07, 0x80, 0x02);
        assert_eq!(&r[..8], &[0x00, 0x1F, 0, 0, 0, 0x02, 0x07, 0x80]);
        // CRC = XOR of bytes 2..=87 = 0x02 ^ 0x07 ^ 0x80 = 0x85
        assert_eq!(r[88], 0x85);
        assert_eq!(r[89], 0);
    }

    fn answer(level: u8, status: u8) -> [u8; REPORT_LEN] {
        let mut r = build_request(0x1F, 0x07, 0x80, 0x02);
        r[0] = status;
        r[9] = level;
        r[88] = checksum(&r);
        r
    }

    #[test]
    fn parses_good_answer() {
        assert_eq!(parse_response(&answer(255, 0x02), 0x07, 0x80), Some(255));
        assert_eq!(level_to_percent(255), 100);
        assert_eq!(level_to_percent(128), 50);
        assert_eq!(level_to_percent(0), 0);
    }

    #[test]
    fn accepts_leading_report_id_byte() {
        let mut v = vec![0u8];
        v.extend_from_slice(&answer(100, 0x02));
        assert_eq!(parse_response(&v, 0x07, 0x80), Some(100));
    }

    #[test]
    fn rejects_bad_status_crc_or_echo() {
        assert_eq!(parse_response(&answer(100, 0x05), 0x07, 0x80), None); // not supported
        let mut bad = answer(100, 0x02);
        bad[88] ^= 1;
        assert_eq!(parse_response(&bad, 0x07, 0x80), None);
        assert_eq!(parse_response(&answer(100, 0x02), 0x07, 0x84), None); // other command
        assert_eq!(parse_response(&[0u8; 10], 0x07, 0x80), None); // too short
    }

    #[test]
    fn barracuda_is_not_in_the_mouse_table() {
        assert!(known_name(0x0550).is_none());
        assert!(known_name(0x0552).is_none());
    }
}
