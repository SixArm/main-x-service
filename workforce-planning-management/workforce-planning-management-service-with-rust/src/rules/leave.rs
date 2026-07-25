//! Leave balance arithmetic (WPM-R5), DB-free.
//!
//! Balances are whole **days** per employee / kind / year. Annual
//! leave over the remaining balance is refused; sick leave may go
//! negative but the result is flagged; other kinds behave like
//! annual (balance-enforced) except `unpaid`, which has no balance.

/// The verdict on taking `days` against a balance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BalanceCheck {
    /// Within balance — proceed.
    Ok {
        /// Days remaining after the request.
        remaining_after: i32,
    },
    /// Allowed, but the balance goes negative (sick leave) — flag it.
    NegativeFlagged {
        /// The (negative) days remaining after the request.
        remaining_after: i32,
    },
    /// Refused: the request exceeds the remaining balance.
    OverBalance {
        /// Days remaining before the request.
        remaining: i32,
        /// Days requested.
        requested: i32,
    },
}

/// Check taking `requested` days of `kind` leave against
/// `entitled - used`. `unpaid` leave always passes (no balance).
#[must_use]
pub fn check_balance(kind: &str, entitled: i32, used: i32, requested: i32) -> BalanceCheck {
    if kind == "unpaid" {
        return BalanceCheck::Ok {
            remaining_after: entitled.saturating_sub(used).saturating_sub(requested),
        };
    }
    let remaining = entitled.saturating_sub(used);
    let remaining_after = remaining.saturating_sub(requested);
    if remaining_after >= 0 {
        BalanceCheck::Ok { remaining_after }
    } else if kind == "sick" {
        BalanceCheck::NegativeFlagged { remaining_after }
    } else {
        BalanceCheck::OverBalance {
            remaining,
            requested,
        }
    }
}

/// Inclusive day span of a leave request (`start..=end`), refusing a
/// reversed range or a span over a year.
///
/// # Errors
///
/// A human-readable refusal (`422` material) for a reversed or
/// oversized range.
pub fn day_span(start: chrono::NaiveDate, end: chrono::NaiveDate) -> Result<i32, String> {
    if end < start {
        return Err(format!("end_on {end} is before start_on {start}"));
    }
    let days = (end - start).num_days() + 1;
    if days > 366 {
        return Err(format!("leave spans {days} days; the cap is 366"));
    }
    i32::try_from(days).map_err(|_| "leave span overflows".to_string())
}

/// Whether two inclusive date ranges overlap (shift-vs-leave and
/// leave-vs-leave conflict checks).
#[must_use]
pub fn ranges_overlap(
    a_start: chrono::NaiveDate,
    a_end: chrono::NaiveDate,
    b_start: chrono::NaiveDate,
    b_end: chrono::NaiveDate,
) -> bool {
    a_start <= b_end && b_start <= a_end
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    /// Annual leave: within balance passes with the new remainder;
    /// over balance is refused with both figures.
    #[test]
    fn annual_balance_is_enforced() {
        assert_eq!(
            check_balance("annual", 25, 20, 5),
            BalanceCheck::Ok { remaining_after: 0 }
        );
        assert_eq!(
            check_balance("annual", 25, 20, 6),
            BalanceCheck::OverBalance {
                remaining: 5,
                requested: 6
            }
        );
    }

    /// Sick leave may go negative but is flagged; within-balance sick
    /// leave is a plain Ok.
    #[test]
    fn sick_negative_is_flagged_not_refused() {
        assert_eq!(
            check_balance("sick", 10, 8, 2),
            BalanceCheck::Ok { remaining_after: 0 }
        );
        assert_eq!(
            check_balance("sick", 10, 8, 5),
            BalanceCheck::NegativeFlagged {
                remaining_after: -3
            }
        );
    }

    /// Unpaid leave has no balance to enforce.
    #[test]
    fn unpaid_always_passes() {
        assert!(matches!(
            check_balance("unpaid", 0, 0, 30),
            BalanceCheck::Ok { .. }
        ));
    }

    /// Day spans are inclusive; reversed and oversized ranges are
    /// refused with readable messages.
    #[test]
    fn day_span_rules() {
        assert_eq!(day_span(d(2026, 8, 3), d(2026, 8, 7)).unwrap(), 5);
        assert_eq!(day_span(d(2026, 8, 3), d(2026, 8, 3)).unwrap(), 1);
        assert!(
            day_span(d(2026, 8, 7), d(2026, 8, 3))
                .unwrap_err()
                .contains("before")
        );
        assert!(
            day_span(d(2026, 1, 1), d(2028, 1, 1))
                .unwrap_err()
                .contains("cap")
        );
    }

    /// Range overlap: touching endpoints overlap; disjoint ranges do
    /// not.
    #[test]
    fn overlap_rules() {
        assert!(ranges_overlap(
            d(2026, 8, 1),
            d(2026, 8, 5),
            d(2026, 8, 5),
            d(2026, 8, 9)
        ));
        assert!(ranges_overlap(
            d(2026, 8, 1),
            d(2026, 8, 9),
            d(2026, 8, 3),
            d(2026, 8, 4)
        ));
        assert!(!ranges_overlap(
            d(2026, 8, 1),
            d(2026, 8, 4),
            d(2026, 8, 5),
            d(2026, 8, 9)
        ));
    }
}
