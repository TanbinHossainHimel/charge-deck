//! Local time as "HH:MM", using the Windows clock (no extra crate needed).

use windows::Win32::System::SystemInformation::GetLocalTime;

pub fn now_hhmm() -> String {
    // SAFETY: GetLocalTime has no preconditions; it just returns the time.
    let t = unsafe { GetLocalTime() };
    format!("{:02}:{:02}", t.wHour, t.wMinute)
}
