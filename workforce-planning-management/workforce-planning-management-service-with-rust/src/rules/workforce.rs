//! Time & attendance and scheduling rules (WPM-R4, WPM-R6), DB-free.
//!
//! Time is recorded in whole **minutes** (no float hours). Overtime is
//! **derived** against the contracted day, scaled by FTE (WPM-D3).

/// Minutes in a day — the hard cap on one day's recorded time.
pub const DAY_MINUTES: i32 = 24 * 60;

/// The contracted full-time working day, in minutes (7.5 h).
pub const CONTRACTED_DAY_MINUTES: i32 = 450;

/// Validate one time entry's minutes: positive, and a single day's
/// total (existing + new) may not exceed 24 h (WPM-R4).
///
/// # Errors
///
/// A human-readable refusal (`422` material).
pub fn check_day_minutes(existing_minutes: i32, new_minutes: i32) -> Result<(), String> {
    if new_minutes <= 0 {
        return Err("minutes must be positive".to_string());
    }
    if new_minutes > DAY_MINUTES {
        return Err(format!(
            "minutes {new_minutes} exceeds a day ({DAY_MINUTES})"
        ));
    }
    let total = existing_minutes.saturating_add(new_minutes);
    if total > DAY_MINUTES {
        return Err(format!(
            "day total {total} minutes exceeds 24h ({existing_minutes} already recorded)"
        ));
    }
    Ok(())
}

/// The contracted day for an employee at `fte_percent` (100 ⇒ full
/// time), rounded down to whole minutes.
#[must_use]
pub fn contracted_day_minutes(fte_percent: i32) -> i32 {
    (CONTRACTED_DAY_MINUTES * fte_percent.clamp(0, 100)) / 100
}

/// Derived overtime for one day: recorded `regular` minutes beyond the
/// FTE-scaled contracted day, plus all explicit `overtime` minutes.
#[must_use]
pub fn overtime_minutes(
    regular_minutes: i32,
    explicit_overtime_minutes: i32,
    fte_percent: i32,
) -> i32 {
    let contracted = contracted_day_minutes(fte_percent);
    let derived = (regular_minutes - contracted).max(0);
    derived.saturating_add(explicit_overtime_minutes.max(0))
}

/// Whether two half-open time windows `[start, end)` overlap — the
/// shift double-booking check (WPM-R6).
#[must_use]
pub fn windows_overlap(
    a_start: chrono::DateTime<chrono::FixedOffset>,
    a_end: chrono::DateTime<chrono::FixedOffset>,
    b_start: chrono::DateTime<chrono::FixedOffset>,
    b_end: chrono::DateTime<chrono::FixedOffset>,
) -> bool {
    a_start < b_end && b_start < a_end
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{FixedOffset, TimeZone};

    fn t(h: u32) -> chrono::DateTime<FixedOffset> {
        FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2026, 8, 3, h, 0, 0)
            .unwrap()
    }

    /// Day-minute caps: zero/negative refused, a single >24h entry
    /// refused, and the running day total enforced.
    #[test]
    fn day_minute_caps() {
        assert!(check_day_minutes(0, 480).is_ok());
        assert!(check_day_minutes(0, 0).is_err());
        assert!(check_day_minutes(0, -5).is_err());
        assert!(check_day_minutes(0, DAY_MINUTES + 1).is_err());
        assert!(check_day_minutes(1000, 441).is_err());
        assert!(check_day_minutes(1000, 440).is_ok());
    }

    /// The contracted day scales by FTE and clamps silly percentages.
    #[test]
    fn contracted_day_scales_by_fte() {
        assert_eq!(contracted_day_minutes(100), 450);
        assert_eq!(contracted_day_minutes(50), 225);
        assert_eq!(contracted_day_minutes(0), 0);
        assert_eq!(contracted_day_minutes(150), 450);
        assert_eq!(contracted_day_minutes(-10), 0);
    }

    /// Overtime = regular beyond the FTE-scaled day + explicit
    /// overtime; never negative.
    #[test]
    fn overtime_derivation() {
        assert_eq!(overtime_minutes(450, 0, 100), 0);
        assert_eq!(overtime_minutes(510, 0, 100), 60);
        assert_eq!(overtime_minutes(300, 0, 50), 75); // 225 contracted at 0.5 FTE
        assert_eq!(overtime_minutes(200, 30, 100), 30); // under contract + explicit
        assert_eq!(overtime_minutes(500, -10, 100), 50); // junk explicit ignored
    }

    /// Half-open windows: back-to-back shifts do NOT overlap;
    /// containment and partial overlap do.
    #[test]
    fn window_overlap() {
        assert!(!windows_overlap(t(8), t(16), t(16), t(23)));
        assert!(windows_overlap(t(8), t(16), t(15), t(23)));
        assert!(windows_overlap(t(8), t(20), t(9), t(10)));
        assert!(!windows_overlap(t(8), t(9), t(10), t(11)));
    }
}
