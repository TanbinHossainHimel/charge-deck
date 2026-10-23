//! Any Bluetooth device whose battery Windows already knows.
//!
//! Windows stores the battery level of many connected Bluetooth devices as a
//! device property: {104EA319-6EE2-4701-BD47-8DDBF425BBE5}, property id 2 (one
//! byte, 0-100). We only READ that property from the Windows device list.
//! Nothing is sent to any device and no Bluetooth connection is opened.

use super::Provider;
use crate::model::{DeviceBattery, DeviceKind, Reading, Status};
use hidapi::HidApi;
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    DIGCF_ALLCLASSES, DIGCF_PRESENT, HDEVINFO, SP_DEVINFO_DATA, SetupDiDestroyDeviceInfoList,
    SetupDiEnumDeviceInfo, SetupDiGetClassDevsW, SetupDiGetDeviceInstanceIdW, SetupDiGetDevicePropertyW,
};
use windows::Win32::Devices::Properties::{DEVPROPTYPE, DEVPROP_TYPE_BYTE};
use windows::Win32::Foundation::DEVPROPKEY;
use windows::core::{GUID, PCWSTR};

const SOURCE: &str = "Bluetooth";

/// {104EA319-6EE2-4701-BD47-8DDBF425BBE5}, 2 = "battery percent"
const BATTERY_KEY: DEVPROPKEY =
    DEVPROPKEY { fmtid: GUID::from_u128(0x104EA319_6EE2_4701_BD47_8DDBF425BBE5), pid: 2 };
/// DEVPKEY_Device_FriendlyName
const FRIENDLY_NAME_KEY: DEVPROPKEY =
    DEVPROPKEY { fmtid: GUID::from_u128(0xA45C254E_DF1C_4EFD_8020_67D146A850E0), pid: 14 };
/// DEVPKEY_Device_DeviceDesc
const DESC_KEY: DEVPROPKEY =
    DEVPROPKEY { fmtid: GUID::from_u128(0xA45C254E_DF1C_4EFD_8020_67D146A850E0), pid: 2 };

/// One Windows device node and what Windows knows about its battery.
#[derive(Debug, Clone)]
pub struct WinDevice {
    pub instance_id: String,
    pub name: String,
    pub battery: Option<u8>,
}

/// UTF-16 bytes (little endian, NUL terminated) -> String.
fn utf16_bytes_to_string(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .take_while(|&u| u != 0)
        .collect();
    String::from_utf16_lossy(&units)
}

/// Guess the icon from the name (only cosmetic).
fn guess_kind(name: &str) -> DeviceKind {
    let n = name.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| n.contains(w));
    if has(&["hands-free", "handsfree"]) {
        // Only audio devices (headsets, earbuds) have a Hands-Free profile.
        DeviceKind::Headset
    } else if has(&["keyboard", "keys"]) {
        DeviceKind::Keyboard
    } else if has(&["mouse", "trackpad", "mx master", "mx anywhere"]) {
        DeviceKind::Mouse
    } else if has(&["headset", "headphone", "earbud", "buds", "airpods", "wh-1000", "wf-1000", "soundcore", "jabra"]) {
        DeviceKind::Headset
    } else if has(&["controller", "gamepad", "dualsense", "dualshock", "xbox", "joy-con", "pro controller"]) {
        DeviceKind::Controller
    } else {
        DeviceKind::Other
    }
}

/// Windows names one gadget per Bluetooth profile ("MI BT 18I Hands-Free AG").
/// Show just the gadget name.
fn clean_name(name: &str) -> String {
    let mut s = name.trim();
    for suffix in [" Hands-Free AG", " Hands-Free", " Avrcp Transport", " Stereo"] {
        if let Some(rest) = s.strip_suffix(suffix) {
            s = rest;
        }
    }
    s.to_string()
}

/// Enumerate present Windows devices. `enumerator` can narrow the list
/// (e.g. "BTHENUM"); None = every present device.
pub fn enumerate(enumerator: Option<&str>) -> Vec<WinDevice> {
    let mut out = Vec::new();
    let wide: Vec<u16>;
    let enumerator_ptr = match enumerator {
        Some(e) => {
            wide = e.encode_utf16().chain(std::iter::once(0)).collect();
            PCWSTR(wide.as_ptr())
        }
        None => PCWSTR::null(),
    };
    // SAFETY: plain read-only SetupAPI calls; the buffers outlive each call and
    // the device list is destroyed at the end.
    unsafe {
        let Ok(set) = SetupDiGetClassDevsW(None, enumerator_ptr, None, DIGCF_PRESENT | DIGCF_ALLCLASSES) else {
            return out;
        };
        let mut index = 0;
        loop {
            let mut data = SP_DEVINFO_DATA { cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32, ..Default::default() };
            if SetupDiEnumDeviceInfo(set, index, &mut data).is_err() {
                break;
            }
            index += 1;

            let battery = read_byte(set, &data, &BATTERY_KEY);
            let name = read_string(set, &data, &FRIENDLY_NAME_KEY)
                .or_else(|| read_string(set, &data, &DESC_KEY))
                .unwrap_or_default();
            let instance_id = instance_id(set, &data);
            out.push(WinDevice { instance_id, name, battery });
        }
        let _ = SetupDiDestroyDeviceInfoList(set);
    }
    out
}

unsafe fn read_byte(set: HDEVINFO, data: &SP_DEVINFO_DATA, key: &DEVPROPKEY) -> Option<u8> {
    let mut ty = DEVPROPTYPE(0);
    let mut buf = [0u8; 4];
    // SAFETY: the caller guarantees `set` and `data` are valid.
    unsafe { SetupDiGetDevicePropertyW(set, data, key, &mut ty, Some(&mut buf), None, 0).ok()? };
    (ty == DEVPROP_TYPE_BYTE && buf[0] <= 100).then_some(buf[0])
}

unsafe fn read_string(set: HDEVINFO, data: &SP_DEVINFO_DATA, key: &DEVPROPKEY) -> Option<String> {
    let mut ty = DEVPROPTYPE(0);
    let mut buf = [0u8; 512];
    // SAFETY: the caller guarantees `set` and `data` are valid.
    unsafe { SetupDiGetDevicePropertyW(set, data, key, &mut ty, Some(&mut buf), None, 0).ok()? };
    let s = utf16_bytes_to_string(&buf);
    (!s.is_empty()).then_some(s)
}

unsafe fn instance_id(set: HDEVINFO, data: &SP_DEVINFO_DATA) -> String {
    let mut buf = [0u16; 400];
    // SAFETY: the caller guarantees `set` and `data` are valid.
    let ok = unsafe { SetupDiGetDeviceInstanceIdW(set, data, Some(&mut buf), None).is_ok() };
    if !ok {
        return String::new();
    }
    let len = buf.iter().position(|&u| u == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}

pub struct Bluetooth;

impl Provider for Bluetooth {
    fn name(&self) -> &'static str {
        SOURCE
    }

    fn scan(&mut self, _hid: &HidApi) -> Vec<DeviceBattery> {
        let mut out: Vec<DeviceBattery> = Vec::new();
        for d in enumerate(None) {
            let Some(pct) = d.battery else { continue };
            if d.name.is_empty() {
                continue;
            }
            let kind = guess_kind(&d.name);
            let name = clean_name(&d.name);
            // The same gadget can appear as several device nodes: keep one per name.
            if out.iter().any(|o| o.name == name) {
                continue;
            }
            out.push(DeviceBattery {
                id: format!("bt:{name}"),
                kind,
                name,
                source: SOURCE,
                status: Status::Ok(Reading::exact(pct, false)),
            });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_utf16() {
        let bytes: Vec<u8> = "MX Keys".encode_utf16().chain([0]).flat_map(|u| u.to_le_bytes()).collect();
        assert_eq!(utf16_bytes_to_string(&bytes), "MX Keys");
    }

    #[test]
    fn cleans_profile_suffix() {
        assert_eq!(clean_name("MI BT 18I Hands-Free AG"), "MI BT 18I");
        assert_eq!(guess_kind("MI BT 18I Hands-Free AG"), DeviceKind::Headset);
        assert_eq!(clean_name("MX Keys"), "MX Keys");
    }

    #[test]
    fn guesses_icons() {
        assert_eq!(guess_kind("Logitech MX Keys"), DeviceKind::Keyboard);
        assert_eq!(guess_kind("Sony WH-1000XM5"), DeviceKind::Headset);
        assert_eq!(guess_kind("Xbox Wireless Controller"), DeviceKind::Controller);
        assert_eq!(guess_kind("Thing"), DeviceKind::Other);
    }
}
