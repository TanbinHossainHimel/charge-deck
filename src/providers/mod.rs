//! Provider trait + registry.
//!
//! Adding a brand = add one file in this folder + one line in `registry()`
//! (like registering a route in Gin).

use crate::model::DeviceBattery;
use hidapi::HidApi;
use std::panic::{catch_unwind, AssertUnwindSafe};

pub mod asus;
pub mod razer_barracuda;

/// One source of battery information (a brand, or "Bluetooth", ...).
pub trait Provider {
    /// Short name shown on cards, e.g. "ASUS receiver".
    fn name(&self) -> &'static str;

    /// Find this provider's devices and read their batteries.
    /// Must finish in a few seconds and never panic.
    fn scan(&mut self, hid: &HidApi) -> Vec<DeviceBattery>;

    /// Does this provider handle this HID interface? Only used by the probe
    /// tool to label its list. It must NOT open or talk to the device.
    fn claims(&self, _info: &hidapi::DeviceInfo) -> bool {
        false
    }
}

/// All providers, in the order they run. Add your provider here.
pub fn registry() -> Vec<Box<dyn Provider>> {
    vec![Box::new(asus::Asus), Box::new(razer_barracuda::RazerBarracuda::default())]
}

/// Run one provider and swallow any panic, so one broken provider can never
/// stop the others (or the window).
pub fn run_guarded(provider: &mut dyn Provider, hid: &HidApi) -> Vec<DeviceBattery> {
    catch_unwind(AssertUnwindSafe(|| provider.scan(hid))).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Boom;
    impl Provider for Boom {
        fn name(&self) -> &'static str {
            "Boom"
        }
        fn scan(&mut self, _hid: &HidApi) -> Vec<DeviceBattery> {
            panic!("provider bug");
        }
    }

    #[test]
    fn a_panicking_provider_is_contained() {
        // HidApi::new can fail on odd machines; skip quietly if so.
        if let Ok(api) = HidApi::new() {
            assert!(run_guarded(&mut Boom, &api).is_empty());
        }
    }
}
