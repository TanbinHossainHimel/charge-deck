//! Game controllers (Xbox, PlayStation, many 2.4 GHz pads).
//!
//! Layer 1: WinRT `RawGameController::TryGetBatteryReport()` gives real
//! capacity numbers. Layer 2 (fallback): `XInputGetBatteryInformation` gives
//! only four levels, shown as approximate %.
//!
//! Safety (rule 3): we ONLY ask for the battery. We never call
//! `GetCurrentReading`, `XInputGetState` or anything else that would read
//! buttons or sticks.

use super::Provider;
use crate::model::{DeviceBattery, DeviceKind, Reading, Status};
use hidapi::HidApi;
use std::thread::sleep;
use std::time::Duration;
use windows::Gaming::Input::RawGameController;
use windows::System::Power::BatteryStatus;
use windows::Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize};
use windows::Win32::UI::Input::XboxController::{
    BATTERY_DEVTYPE_GAMEPAD, BATTERY_LEVEL_EMPTY, BATTERY_LEVEL_FULL, BATTERY_LEVEL_LOW, BATTERY_LEVEL_MEDIUM,
    BATTERY_TYPE_DISCONNECTED, BATTERY_TYPE_WIRED, XINPUT_BATTERY_INFORMATION, XInputGetBatteryInformation,
};

const SOURCE: &str = "Game controller";

/// XInput only has four levels. These are rough middles of each band.
fn xinput_level_to_percent(level: u8) -> u8 {
    match level {
        x if x == BATTERY_LEVEL_EMPTY.0 as u8 => 5,
        x if x == BATTERY_LEVEL_LOW.0 as u8 => 25,
        x if x == BATTERY_LEVEL_MEDIUM.0 as u8 => 60,
        x if x == BATTERY_LEVEL_FULL.0 as u8 => 100,
        _ => 0,
    }
}

/// remaining / full capacity -> percent (None when the numbers are unusable,
/// e.g. wired pads report 0 / 0).
fn capacity_to_percent(remaining: i32, full: i32) -> Option<u8> {
    if full <= 0 || remaining < 0 {
        return None;
    }
    Some((remaining as i64 * 100 / full as i64).clamp(0, 100) as u8)
}

#[derive(Default)]
pub struct Controllers {
    ro_initialised: bool,
}

impl Controllers {
    fn winrt_controllers(&mut self) -> Vec<DeviceBattery> {
        if !self.ro_initialised {
            // SAFETY: plain COM/WinRT init for this thread. An "already
            // initialised" answer is fine, so the result is ignored.
            let _ = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
            self.ro_initialised = true;
        }
        let mut list = RawGameController::RawGameControllers().ok();
        // Windows fills this list asynchronously: the first call can be empty.
        if list.as_ref().is_none_or(|l| l.Size().unwrap_or(0) == 0) {
            sleep(Duration::from_millis(300));
            list = RawGameController::RawGameControllers().ok();
        }
        let mut out = Vec::new();
        for c in list.into_iter().flatten() {
            let name = c.DisplayName().map(|s| s.to_string()).unwrap_or_else(|_| "Game controller".into());
            let id = c.NonRoamableId().map(|s| s.to_string()).unwrap_or_else(|_| name.clone());
            let Ok(report) = c.TryGetBatteryReport() else { continue };
            let remaining = report.RemainingCapacityInMilliwattHours().and_then(|v| v.Value());
            let full = report.FullChargeCapacityInMilliwattHours().and_then(|v| v.Value());
            let (Ok(remaining), Ok(full)) = (remaining, full) else { continue };
            let Some(pct) = capacity_to_percent(remaining, full) else { continue };
            let charging = report.Status().is_ok_and(|s| s == BatteryStatus::Charging);
            out.push(DeviceBattery {
                id: format!("pad:{id}"),
                name,
                kind: DeviceKind::Controller,
                source: SOURCE,
                status: Status::Ok(Reading::exact(pct, charging)),
            });
        }
        out
    }

    fn xinput_controllers(&self) -> Vec<DeviceBattery> {
        let mut out = Vec::new();
        for slot in 0..4u32 {
            let mut info = XINPUT_BATTERY_INFORMATION::default();
            // SAFETY: read-only battery query for one gamepad slot.
            let rc = unsafe { XInputGetBatteryInformation(slot, BATTERY_DEVTYPE_GAMEPAD, &mut info) };
            if rc != 0 || info.BatteryType == BATTERY_TYPE_DISCONNECTED || info.BatteryType == BATTERY_TYPE_WIRED {
                continue;
            }
            let pct = xinput_level_to_percent(info.BatteryLevel.0 as u8);
            out.push(DeviceBattery {
                id: format!("xinput:{slot}"),
                name: format!("Xbox-compatible controller {}", slot + 1),
                kind: DeviceKind::Controller,
                source: SOURCE,
                status: Status::Ok(Reading::estimated(pct, false)),
            });
        }
        out
    }
}

impl Provider for Controllers {
    fn name(&self) -> &'static str {
        SOURCE
    }

    fn scan(&mut self, _hid: &HidApi) -> Vec<DeviceBattery> {
        let found = self.winrt_controllers();
        // Skip the XInput fallback when WinRT already saw the controllers,
        // so one pad never shows up twice.
        if found.is_empty() { self.xinput_controllers() } else { found }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacity_math() {
        assert_eq!(capacity_to_percent(50, 100), Some(50));
        assert_eq!(capacity_to_percent(150, 100), Some(100));
        assert_eq!(capacity_to_percent(0, 0), None);
    }

    #[test]
    fn xinput_levels() {
        assert_eq!(xinput_level_to_percent(0), 5);
        assert_eq!(xinput_level_to_percent(1), 25);
        assert_eq!(xinput_level_to_percent(2), 60);
        assert_eq!(xinput_level_to_percent(3), 100);
    }
}
