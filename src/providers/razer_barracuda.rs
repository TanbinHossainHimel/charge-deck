//! Razer Barracuda X (2022) headset on its 2.4 GHz USB-C dongle.
//!
//! Ported from `src/Protocol.cs` of github.com/mehdibouchami/razer-barracuda-x-battery
//! (MIT, see THIRD_PARTY_NOTICES.md); the 0x0552 "Macronix" framing comes from
//! github.com/KhromotozzDevOut/razer-barracuda-battery-tray (MIT).
//!
//! How it works, in simple words: the headset has an Airoha chip. The dongle
//! passes "RACE" commands to it. We ask "what is your battery voltage?" a few
//! times, then turn the voltage into a percentage with a lookup curve.
//! The headset reports no percentage and no charging flag on 2.4 GHz, so
//! "charging" is guessed from how much the voltage wobbles between samples.
//!
//! Safety: only VID 0x1532 with PID 0x0550 / 0x0552, only the vendor collection
//! (usage page 0xFF00). Commands: switch the dongle to "remote mode" (so it
//! forwards to the headset), read the voltage, switch remote mode back off.
//! Remote mode is not a user setting and is always switched off again.

use super::Provider;
use crate::hid_util::{flush, open_if_allowed, read_until, trace_rx, write_report};
use crate::model::{DeviceBattery, DeviceKind, Reading, Status};
use hidapi::{DeviceInfo, HidApi, HidDevice};
use std::thread::sleep;
use std::time::Duration;

const RAZER_VID: u16 = 0x1532;
const PIDS: [u16; 2] = [0x0550, 0x0552];
const SOURCE: &str = "Razer dongle";

const RACE_GET_BATTERY: u8 = 0x31;
const CHARGE_SAMPLES: usize = 7;
const CHARGE_JITTER_MV: u32 = 12;
const CHARGE_VOLTAGE_MV: u32 = 4180;
const TIMEOUT_MS: u64 = 800;

/// Battery voltage (mV) for 0 %, 10 %, ... 100 %.
const CURVE: [u32; 11] = [3300, 3680, 3750, 3790, 3830, 3870, 3910, 3960, 4020, 4080, 4150];

/// Linear interpolation on the discharge curve.
pub fn mv_to_percent(mv: u32) -> u8 {
    if mv <= CURVE[0] {
        return 0;
    }
    for i in 1..CURVE.len() {
        if mv < CURVE[i] {
            return ((i as u32 - 1) * 10 + (mv - CURVE[i - 1]) * 10 / (CURVE[i] - CURVE[i - 1])) as u8;
        }
    }
    100
}

/// CRC-16/XMODEM: polynomial 0x1021, start value 0.
pub fn crc16_xmodem(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &b in data {
        crc ^= (b as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x1021 } else { crc << 1 };
        }
    }
    crc
}

/// Walk all RACE frames inside one report and return the first one `matches`.
/// A report can hold several frames back to back (the dongle also sends
/// unrelated status events), so the answer is often not the first frame.
/// Frame = `50 49 ..` + 10-byte header, bytes 8-9 = payload length (LE).
pub fn find_frame(report: &[u8], offset: usize, length: usize, matches: impl Fn(&[u8]) -> bool) -> Option<Vec<u8>> {
    const HEADER: usize = 10;
    let end = (offset + length).min(report.len());
    let mut pos = offset;
    while pos + HEADER <= end {
        if report[pos] != 0x50 || report[pos + 1] != 0x49 {
            return None;
        }
        let payload = report[pos + 8] as usize | (report[pos + 9] as usize) << 8;
        let frame_end = pos + HEADER + payload;
        if frame_end > end {
            return None;
        }
        let frame = &report[pos..frame_end];
        if matches(frame) {
            return Some(frame.to_vec());
        }
        pos = frame_end;
    }
    None
}

/// `50 41 <group> <seq> <body...>`
pub fn build_race(group: u8, seq: u8, body: &[u8]) -> Vec<u8> {
    let mut v = vec![0x50, 0x41, group, seq];
    v.extend_from_slice(body);
    v
}

/// 0x0550 (YS-Tech) 64-byte frame: `02 80 len crcLo crcHi race...`, zero padded.
/// The CRC covers all 64 bytes with the two CRC bytes set to 0.
pub fn build_ystech_frame(race: &[u8]) -> [u8; 64] {
    let mut f = [0u8; 64];
    f[0] = 0x02;
    f[1] = 0x80;
    f[2] = race.len() as u8;
    f[5..5 + race.len()].copy_from_slice(race);
    let crc = crc16_xmodem(&f);
    f[3] = crc as u8;
    f[4] = (crc >> 8) as u8;
    f
}

/// Lowest sample is the voltage; big wobble or very high voltage = charging.
pub fn analyse_samples(samples: &[u32]) -> Option<(u32, bool)> {
    let min = *samples.iter().min()?;
    let max = *samples.iter().max()?;
    Some((min, max - min >= CHARGE_JITTER_MV || min >= CHARGE_VOLTAGE_MV))
}

#[derive(Clone, Copy, PartialEq)]
enum Bridge {
    /// PID 0x0550, report ID 0x02
    YsTech,
    /// PID 0x0552, report ID 0x01
    Macronix,
}

struct Link<'a> {
    dev: &'a HidDevice,
    bridge: Bridge,
    seq: &'a mut u8,
}

impl Link<'_> {
    fn report_id(&self) -> u8 {
        if self.bridge == Bridge::YsTech { 0x02 } else { 0x01 }
    }

    /// Send one RACE command and return the matching reply (full frame).
    fn race(&mut self, group: u8, body: &[u8]) -> Option<Vec<u8>> {
        *self.seq = *self.seq % 127 + 1; // cycles 1..=127
        let seq = *self.seq;
        let race = build_race(group, seq, body);
        let matches = move |p: &[u8]| p.len() >= 13 && p[0] == 0x50 && p[1] == 0x49 && p[10] == group && p[11] == (0x80 | seq);
        match self.bridge {
            Bridge::YsTech => self.send_ystech(&race, matches),
            Bridge::Macronix => self.send_macronix(&race, matches),
        }
    }

    fn send_ystech(&mut self, race: &[u8], matches: impl Fn(&[u8]) -> bool) -> Option<Vec<u8>> {
        let frame = build_ystech_frame(race);
        let mut accepted = false;
        for attempt in 0..5 {
            if attempt > 0 {
                sleep(Duration::from_millis(50));
            }
            // READ-ONLY: a RACE command (remote mode / get battery voltage).
            if !write_report(self.dev, &frame) {
                return None;
            }
            let mut status = [0u8; 64];
            status[0] = 0x02;
            let n = self.dev.get_input_report(&mut status).ok()?;
            if n < 2 {
                return None;
            }
            trace_rx(&status[..n.min(64)]);
            match status[1] {
                b'O' => {
                    accepted = true;
                    break;
                }
                b'F' => continue, // busy: wait and retry
                _ => return None,
            }
        }
        if !accepted {
            return None;
        }
        read_until(self.dev, TIMEOUT_MS, |r| {
            if r.len() < 2 || r[0] != 0x02 || r[1] as usize > r.len() - 2 {
                return None;
            }
            find_frame(r, 2, r[1] as usize, &matches)
        })
    }

    fn enter_macronix_app_mode(&mut self) -> bool {
        let id = self.report_id();
        // READ-ONLY: the "app mode" handshake the official tool also uses.
        if !write_report(self.dev, &[id, 0x40]) {
            return false;
        }
        read_until(self.dev, TIMEOUT_MS, |r| {
            if r.len() >= 4 && r[0] == id && r[1] == 0x40 { Some(r.to_vec()) } else { None }
        })
        .is_some_and(|m| m[2] == 0x01 && m[3] == 0x01)
    }

    fn send_macronix(&mut self, race: &[u8], matches: impl Fn(&[u8]) -> bool) -> Option<Vec<u8>> {
        let id = self.report_id();
        let mut frame = vec![id, 0x80, race.len() as u8];
        frame.extend_from_slice(race);
        // READ-ONLY: a RACE command (remote mode / get battery voltage).
        if !write_report(self.dev, &frame) {
            return None;
        }
        read_until(self.dev, TIMEOUT_MS, |r| {
            if r.len() < 3 || r[0] != id || r[1] != 0x80 || r[2] as usize > r.len() - 3 {
                return None;
            }
            find_frame(r, 3, r[2] as usize, &matches)
        })
    }

    /// One full reading. Returns (mV, charging) or None if the headset is away.
    fn query(&mut self) -> Option<(u32, bool)> {
        flush(self.dev);
        if self.bridge == Bridge::Macronix && !self.enter_macronix_app_mode() {
            return None;
        }
        // Remote mode ON: the dongle then forwards our commands to the headset.
        let reply = self.race(0x0E, &[0x02, 0xE1, 0x01])?;
        if reply[12] != 0 {
            return None;
        }
        let mut samples: Vec<u32> = Vec::new();
        for i in 0..CHARGE_SAMPLES {
            if i > 0 {
                sleep(Duration::from_millis(350));
            }
            if let Some(r) = self.race(0x06, &[0x01, 0x00, RACE_GET_BATTERY]) {
                if r.len() >= 15 && r[12] == 0 {
                    let mv = r[13] as u32 | (r[14] as u32) << 8;
                    if (2500..=4500).contains(&mv) {
                        samples.push(mv);
                    }
                }
            }
        }
        // Always switch remote mode OFF again, even if sampling failed.
        let _ = self.race(0x0E, &[0x02, 0xE1, 0x00]);
        analyse_samples(&samples)
    }
}

#[derive(Default)]
pub struct RazerBarracuda {
    seq: u8,
    /// Last % measured while NOT charging (shown while charging).
    last_percent: Option<u8>,
}

impl RazerBarracuda {
    fn is_ours(info: &DeviceInfo) -> bool {
        info.vendor_id() == RAZER_VID && PIDS.contains(&info.product_id()) && info.usage_page() == 0xFF00
    }
}

impl Provider for RazerBarracuda {
    fn name(&self) -> &'static str {
        SOURCE
    }

    fn claims(&self, info: &DeviceInfo) -> bool {
        RazerBarracuda::is_ours(info)
    }

    fn scan(&mut self, hid: &HidApi) -> Vec<DeviceBattery> {
        // Some dongles expose several vendor collections; writing to the wrong
        // one fails, so try each candidate until one answers.
        let mut found_dongle: Option<u16> = None;
        for info in hid.device_list().filter(|i| RazerBarracuda::is_ours(i)) {
            found_dongle = Some(info.product_id());
            let Some(dev) = open_if_allowed(hid, info) else { continue };
            let bridge = if info.product_id() == 0x0550 { Bridge::YsTech } else { Bridge::Macronix };
            let mut link = Link { dev: &dev, bridge, seq: &mut self.seq };
            if let Some((mv, charging)) = link.query() {
                let pct = mv_to_percent(mv);
                let reading = if charging {
                    // Voltage is inflated by the charger, so keep the old %.
                    match self.last_percent {
                        Some(p) => Reading { percent: p, estimated: true, voltage_mv: Some(mv), charging: true },
                        None => Reading { percent: pct, estimated: true, voltage_mv: Some(mv), charging: true },
                    }
                } else {
                    self.last_percent = Some(pct);
                    Reading { percent: pct, estimated: true, voltage_mv: Some(mv), charging: false }
                };
                return vec![card(info.product_id(), Status::Ok(reading))];
            }
        }
        // Dongle present but headset silent (off or out of range).
        match found_dongle {
            Some(pid) => vec![card(pid, Status::Asleep)],
            None => vec![],
        }
    }
}

fn card(pid: u16, status: Status) -> DeviceBattery {
    DeviceBattery {
        id: format!("razer-barracuda:{pid:04x}"),
        name: "Razer Barracuda X".to_string(),
        kind: DeviceKind::Headset,
        source: SOURCE,
        status,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curve_points() {
        assert_eq!(mv_to_percent(3300), 0);
        assert_eq!(mv_to_percent(3000), 0);
        assert_eq!(mv_to_percent(3975), 72);
        assert_eq!(mv_to_percent(4150), 100);
        assert_eq!(mv_to_percent(4300), 100);
    }

    #[test]
    fn crc_check_value() {
        assert_eq!(crc16_xmodem(b"123456789"), 0x31C3);
    }

    #[test]
    fn ystech_frame_layout_and_crc() {
        let race = build_race(0x06, 1, &[0x01, 0x00, 0x31]);
        let f = build_ystech_frame(&race);
        assert_eq!(&f[..5], &[0x02, 0x80, 7, f[3], f[4]]);
        assert_eq!(&f[5..12], &race[..]);
        // CRC over the frame with its own CRC bytes zeroed must equal the stored one.
        let mut z = f;
        z[3] = 0;
        z[4] = 0;
        assert_eq!(crc16_xmodem(&z), f[3] as u16 | (f[4] as u16) << 8);
    }

    /// 50 49 + 6 header bytes (last two = payload length) + payload.
    fn frame(group: u8, seq: u8, payload_after_seq: &[u8]) -> Vec<u8> {
        // payload starts at byte 10: [group, 0x80|seq, status...]
        let mut payload = vec![group, 0x80 | seq];
        payload.extend_from_slice(payload_after_seq);
        let mut f = vec![0x50, 0x49, 0, 0, 0, 0, 0, 0, payload.len() as u8, 0];
        f.extend(payload);
        f
    }

    #[test]
    fn walks_two_frames_second_is_ours() {
        let other = frame(0x11, 0x00, &[0, 0, 0]);
        let ours = frame(0x06, 5, &[0, 0x6e, 0x0f]); // status 0, 0x0f6e = 3950 mV
        let mut report = vec![0x02, 0];
        report.extend(&other);
        report.extend(&ours);
        report[1] = (report.len() - 2) as u8;
        let m = |p: &[u8]| p.len() >= 13 && p[10] == 0x06 && p[11] == (0x80 | 5);
        let got = find_frame(&report, 2, report[1] as usize, m).unwrap();
        assert_eq!(got, ours);
        assert_eq!(got[13] as u32 | (got[14] as u32) << 8, 3950);
    }

    #[test]
    fn no_match_returns_none() {
        let other = frame(0x11, 0x00, &[0, 0, 0]);
        let mut report = vec![0x02, other.len() as u8];
        report.extend(&other);
        assert!(find_frame(&report, 2, other.len(), |_| false).is_none());
    }

    #[test]
    fn charging_detection() {
        assert_eq!(analyse_samples(&[3900, 3902, 3901]), Some((3900, false)));
        assert_eq!(analyse_samples(&[3950, 3995, 3970]), Some((3950, true))); // wobble
        assert_eq!(analyse_samples(&[4190, 4191]), Some((4190, true))); // high voltage
        assert_eq!(analyse_samples(&[]), None);
    }
}
