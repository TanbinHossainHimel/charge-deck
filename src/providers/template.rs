//! TEMPLATE for a new brand. Copy this file, rename it (e.g. `steelseries.rs`),
//! then follow CONTRIBUTING.md. This example is NOT registered, so it never runs.
//!
//! The three jobs of a provider:
//!   1. Recognise YOUR devices (vendor id + product id + usage page).
//!   2. Ask them for the battery (read-only!).
//!   3. Turn the answer into a `DeviceBattery`.
//!
//! Keep the parsing in small pure functions (bytes in, numbers out) so you can
//! unit test it with real captured bytes, without owning the hardware.

use super::Provider;
use crate::hid_util::{flush, open_if_allowed, read_until, write_report};
use crate::model::{DeviceBattery, DeviceKind, Reading, Status};
use hidapi::{DeviceInfo, HidApi};

/// Example numbers. Replace with your brand's (see Device Manager -> Hardware IDs).
const EXAMPLE_VID: u16 = 0xFFFF;
const EXAMPLE_PID: u16 = 0x0001;
const SOURCE: &str = "Example dongle";

/// The question we send. First byte = report id (0 if the device has none).
/// MANDATORY: this must be a read-only battery/status query (rules 5 and 6).
const BATTERY_QUERY: [u8; 3] = [0x00, 0x01, 0x02];

/// PURE function: bytes in, battery out. Test this with real captures.
/// Returns None when the answer is not for our question (wrong echo).
fn parse_answer(answer: &[u8]) -> Option<Status> {
    if answer.len() < 4 || answer[0] != 0x01 {
        return None; // not our answer: keep listening
    }
    match answer[2] {
        0 => Some(Status::Asleep),
        percent => Some(Status::Ok(Reading::exact(percent, answer[3] == 1))),
    }
}

pub struct Example;

impl Example {
    /// Rule 6: match by vendor id + product id + usage page. Never "try"
    /// commands on devices you don't explicitly know.
    fn is_ours(info: &DeviceInfo) -> bool {
        info.vendor_id() == EXAMPLE_VID
            && info.product_id() == EXAMPLE_PID
            && info.usage_page() == 0xFF00 // vendor collection only (rule 4)
    }
}

impl Provider for Example {
    /// Must be unique across all providers.
    fn name(&self) -> &'static str {
        SOURCE
    }

    /// Lets the probe tool label "claimed by". Must not talk to the device.
    fn claims(&self, info: &DeviceInfo) -> bool {
        Example::is_ours(info)
    }

    fn scan(&mut self, hid: &HidApi) -> Vec<DeviceBattery> {
        let mut out = Vec::new();
        for info in hid.device_list().filter(|i| Example::is_ours(i)) {
            // `open_if_allowed` refuses input collections (rule 4).
            let Some(dev) = open_if_allowed(hid, info) else { continue };
            flush(&dev); // throw away old answers
            // READ-ONLY: battery query only.
            if !write_report(&dev, &BATTERY_QUERY) {
                continue; // wrong collection: try the next one
            }
            // Wait up to 600 ms for an answer that parses.
            if let Some(status) = read_until(&dev, 600, parse_answer) {
                out.push(DeviceBattery {
                    id: format!("example:{:04x}", info.product_id()),
                    name: info.product_string().unwrap_or("Example device").to_string(),
                    kind: DeviceKind::Mouse,
                    source: SOURCE,
                    status,
                });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Put REAL captured bytes here (from `charge-deck-probe --verbose`).
    #[test]
    fn parses_a_captured_answer() {
        let answer = [0x01, 0x00, 77, 0x00];
        assert_eq!(parse_answer(&answer), Some(Status::Ok(Reading::exact(77, false))));
    }

    #[test]
    fn ignores_other_reports() {
        assert_eq!(parse_answer(&[0x7F, 0, 50, 0]), None);
    }
}
