//! Local time as "1:05 PM", using the Windows clock (no extra crate needed).

use windows::Win32::System::SystemInformation::GetLocalTime;

pub fn now_hhmm() -> String {
    // SAFETY: GetLocalTime has no preconditions; it just returns the time.
    let t = unsafe { GetLocalTime() };
    format_12h(t.wHour, t.wMinute)
}

/// 13:05 -> "1:05 PM", 0:07 -> "12:07 AM"
fn format_12h(hour: u16, minute: u16) -> String {
    let suffix = if hour >= 12 { "PM" } else { "AM" };
    let h12 = if hour % 12 == 0 { 12 } else { hour % 12 };
    format!("{h12}:{minute:02} {suffix}")
}

#[cfg(test)]
mod tests {
    use super::format_12h;

    #[test]
    fn twelve_hour_clock() {
        assert_eq!(format_12h(13, 5), "1:05 PM");
        assert_eq!(format_12h(0, 7), "12:07 AM");
        assert_eq!(format_12h(12, 0), "12:00 PM");
        assert_eq!(format_12h(9, 30), "9:30 AM");
    }
}
