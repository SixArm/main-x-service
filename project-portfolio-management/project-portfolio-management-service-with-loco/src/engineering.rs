//! Pure rules for the engineering-team features — task statuses, the
//! honest burndown derivation, the MoSCoW tag convention, and the
//! milestone kinds — behind `controllers::engineering`. No I/O: the
//! burndown only ever counts real completion stamps, and the MoSCoW
//! bands come from an explicit, disclosed tag convention.

use std::collections::BTreeMap;

use chrono::NaiveDate;

/// The Kanban task statuses (board column order).
pub const TASK_STATUSES: &[&str] = &["todo", "in_progress", "in_review", "done", "blocked"];

/// Milestone kinds for the delivery calendar. Absent reads as
/// `milestone`.
pub const MILESTONE_KINDS: &[&str] = &["milestone", "demo", "release", "checkpoint"];

/// The `MoSCoW` bands, priority order.
pub const MOSCOW_BANDS: &[&str] = &["must", "should", "could", "wont"];

/// Parse one plan tag against the `MoSCoW` convention
/// (`moscow:<band>`); unknown bands are rejected, never guessed.
#[must_use]
pub fn parse_moscow_tag(tag: &str) -> Option<&'static str> {
    let band = tag.trim().strip_prefix("moscow:")?.trim().to_lowercase();
    MOSCOW_BANDS.iter().find(|b| **b == band).copied()
}

/// One burndown point: the remaining open-task count at end of `date`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct BurndownPoint {
    /// The day.
    pub date: NaiveDate,
    /// Tasks not yet completed at the end of that day.
    pub remaining: usize,
}

/// The honest burndown: for each day of `[from, to]`, `total` minus
/// the completions stamped **on or before** that day. Only real
/// `done_at` stamps count — no ideal line, no interpolation (a
/// front-end may draw an ideal line, labelled as such). Windows longer
/// than 366 days are truncated (a sprint is not a year).
#[must_use]
pub fn burndown(
    total: usize,
    done_dates: &[NaiveDate],
    from: NaiveDate,
    to: NaiveDate,
) -> Vec<BurndownPoint> {
    let mut points = Vec::new();
    let mut day = from;
    let mut steps = 0;
    while day <= to && steps <= 366 {
        let done = done_dates.iter().filter(|d| **d <= day).count();
        points.push(BurndownPoint {
            date: day,
            remaining: total.saturating_sub(done),
        });
        day = match day.succ_opt() {
            Some(next) => next,
            None => break,
        };
        steps += 1;
    }
    points
}

/// Sprint-note categories (retro + RAD feedback log). Only `action`
/// and `feedback` notes convert to tasks.
pub const NOTE_CATEGORIES: &[&str] = &["went_well", "improve", "action", "feedback"];

/// Note categories that may convert into a task.
pub const CONVERTIBLE_NOTE_CATEGORIES: &[&str] = &["action", "feedback"];

/// `DevOps` event kinds. A `recovery` must reference its `incident`;
/// an `incident` may declare the deploy that caused it.
pub const DEVOPS_EVENT_KINDS: &[&str] = &["deploy", "incident", "recovery"];

/// Parse the `PROJECT_PORTFOLIO_MANAGEMENT_WIP_LIMITS` JSON — a map of
/// task status → per-item cap, e.g. `{"in_progress": 3}`. Absent /
/// blank / unparsable / unknown-status keys / non-positive caps ⇒
/// `None` (no limits enforced; the board says so rather than inventing
/// caps).
#[must_use]
pub fn parse_wip_limits(raw: Option<&str>) -> Option<BTreeMap<String, usize>> {
    let raw = raw?.trim();
    if raw.is_empty() {
        return None;
    }
    let parsed: BTreeMap<String, i64> = serde_json::from_str(raw).ok()?;
    let mut limits = BTreeMap::new();
    for (status, cap) in parsed {
        if !TASK_STATUSES.contains(&status.as_str()) || cap < 1 {
            return None;
        }
        let cap = usize::try_from(cap).ok()?;
        limits.insert(status, cap);
    }
    Some(limits)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        s.parse().expect("date")
    }

    #[test]
    fn moscow_tags_parse_and_reject() {
        assert_eq!(parse_moscow_tag("moscow:must"), Some("must"));
        assert_eq!(parse_moscow_tag(" moscow:WONT "), Some("wont"));
        assert_eq!(
            parse_moscow_tag("moscow:maybe"),
            None,
            "unknown band refused"
        );
        assert_eq!(
            parse_moscow_tag("tech:rust"),
            None,
            "non-moscow tag ignored"
        );
    }

    #[test]
    fn burndown_counts_only_real_completions() {
        let points = burndown(
            3,
            &[d("2026-07-02"), d("2026-07-03")],
            d("2026-07-01"),
            d("2026-07-04"),
        );
        let remaining: Vec<usize> = points.iter().map(|p| p.remaining).collect();
        assert_eq!(
            remaining,
            vec![3, 2, 1, 1],
            "steps down only on real done days"
        );
        assert_eq!(points[0].date, d("2026-07-01"));
    }

    #[test]
    fn burndown_never_goes_negative_and_bounds_the_window() {
        let points = burndown(
            1,
            &[d("2026-07-01"), d("2026-07-01")],
            d("2026-07-01"),
            d("2026-07-02"),
        );
        assert_eq!(points[1].remaining, 0, "saturates at zero");
        let long = burndown(1, &[], d("2020-01-01"), d("2030-01-01"));
        assert!(long.len() <= 367, "window truncated");
    }
    #[test]
    fn wip_limits_parse_or_decline() {
        assert_eq!(parse_wip_limits(None), None);
        assert_eq!(parse_wip_limits(Some("nonsense")), None);
        assert_eq!(
            parse_wip_limits(Some(r#"{"sideways": 3}"#)),
            None,
            "unknown status refused"
        );
        assert_eq!(
            parse_wip_limits(Some(r#"{"in_progress": 0}"#)),
            None,
            "non-positive refused"
        );
        let limits = parse_wip_limits(Some(r#"{"in_progress": 3, "in_review": 2}"#)).unwrap();
        assert_eq!(limits["in_progress"], 3);
        assert_eq!(limits["in_review"], 2);
    }
}
