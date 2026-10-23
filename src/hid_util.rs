//! Small helpers shared by every HID-based provider and the probe tool.
//!
//! SAFETY (rule 4): every provider opens devices through `open_if_allowed`,
//! which refuses input collections (usage page 0x01 = keyboard/mouse/gamepad).
//! That way no provider can ever read keypresses or mouse movement.

use hidapi::{DeviceInfo, HidApi, HidDevice};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

static VERBOSE: AtomicBool = AtomicBool::new(false);

/// Turn on printing of every byte sent/received (used by the probe tool).
pub fn set_verbose(on: bool) {
    VERBOSE.store(on, Ordering::Relaxed);
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ")
}

/// Print "> bytes" for data we send.
pub fn trace_tx(bytes: &[u8]) {
    if VERBOSE.load(Ordering::Relaxed) {
        println!("    > {}", hex(bytes));
    }
}

/// Print "< bytes" for data we receive.
pub fn trace_rx(bytes: &[u8]) {
    if VERBOSE.load(Ordering::Relaxed) {
        println!("    < {}", hex(bytes));
    }
}

/// Rule 4: only vendor collections (0xFF00 and up) and battery/power
/// collections (0x06 Generic Device, 0x84 Power Device, 0x85 Battery System)
/// may ever be opened. 0x01 (Generic Desktop) is never allowed.
pub fn is_allowed_usage_page(usage_page: u16) -> bool {
    usage_page >= 0xFF00 || matches!(usage_page, 0x06 | 0x84 | 0x85)
}

/// Open one specific collection by path, but only if rule 4 allows it.
/// We open by *path* (not by VID/PID) because VID/PID would open whichever
/// collection comes first, which could be the keyboard itself.
pub fn open_if_allowed(api: &HidApi, info: &DeviceInfo) -> Option<HidDevice> {
    if !is_allowed_usage_page(info.usage_page()) {
        return None;
    }
    api.open_path(info.path()).ok()
}

/// Throw away reports that were already waiting (old answers).
pub fn flush(dev: &HidDevice) {
    let mut buf = [0u8; 256];
    for _ in 0..64 {
        match dev.read_timeout(&mut buf, 0) {
            Ok(n) if n > 0 => continue,
            _ => break,
        }
    }
}

/// Read reports until `accept` returns Some, or `timeout_ms` has passed.
/// Reports that `accept` rejects are ignored (e.g. wrong echo).
pub fn read_until<T>(
    dev: &HidDevice,
    timeout_ms: u64,
    mut accept: impl FnMut(&[u8]) -> Option<T>,
) -> Option<T> {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let mut buf = [0u8; 256];
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return None;
        }
        match dev.read_timeout(&mut buf, left.as_millis().max(1) as i32) {
            Ok(n) if n > 0 => {
                trace_rx(&buf[..n]);
                if let Some(v) = accept(&buf[..n]) {
                    return Some(v);
                }
            }
            Ok(_) => {}
            Err(_) => return None,
        }
    }
}

/// Send one output report. READ-ONLY: callers only pass battery/status queries.
pub fn write_report(dev: &HidDevice, data: &[u8]) -> bool {
    trace_tx(data);
    // READ-ONLY: this only sends a battery/status query chosen by the provider.
    dev.write(data).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_collections_are_refused() {
        assert!(!is_allowed_usage_page(0x01)); // keyboard / mouse / gamepad
        assert!(!is_allowed_usage_page(0x0C)); // consumer control
        assert!(!is_allowed_usage_page(0x00));
    }

    #[test]
    fn vendor_and_battery_pages_are_allowed() {
        for p in [0xFF00, 0xFF01, 0xFFC0, 0x06, 0x84, 0x85] {
            assert!(is_allowed_usage_page(p), "{p:#x}");
        }
    }

    #[test]
    fn hex_formats_bytes() {
        assert_eq!(hex(&[0x02, 0x12, 0xff]), "02 12 ff");
    }
}
