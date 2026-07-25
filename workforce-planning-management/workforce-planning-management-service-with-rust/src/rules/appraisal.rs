//! Pure rules for 360° appraisals (WPM-R29): rater groups, the
//! nomination bounds, the lifecycle, the response-completeness check,
//! and the group-floored aggregation (WPM-D21). No I/O; arithmetic
//! never panics.

use std::collections::BTreeMap;

/// The closed rater groups. External raters are deferred (WPM has no
/// identity for them).
pub const GROUPS: &[&str] = &["self", "manager", "peer", "report"];

/// Most raters an appraisal may nominate (research-conventional cap).
pub const MAX_RATERS: usize = 12;

/// Fewest **non-self** nominations required to start collecting.
pub const MIN_NON_SELF_RATERS: usize = 3;

/// The group disclosure floor for `peer` / `report` cells (WPM-D21).
/// `manager` and `self` disclose at 1 by convention.
pub const GROUP_FLOOR: usize = 3;

/// Most competencies an appraisal may declare.
pub const MAX_COMPETENCIES: usize = 12;

/// The appraisal lifecycle: draft → collecting → shared. Terminal at
/// shared; no way back (nominations freeze at collecting).
///
/// # Errors
///
/// A human-readable refusal naming the legal moves.
pub fn transition(current: &str, to: &str) -> Result<(), String> {
    let ok = matches!((current, to), ("draft", "collecting") | ("collecting", "shared"));
    if ok {
        Ok(())
    } else {
        Err(format!(
            "illegal transition `{current}` → `{to}` (draft → collecting → shared)"
        ))
    }
}

/// Whether a group under the WPM-D21 floor rule may disclose with
/// `count` responses: `manager`/`self` at 1; `peer`/`report` at
/// [`GROUP_FLOOR`]. Unknown groups never disclose.
#[must_use]
pub fn group_discloses(group: &str, count: usize) -> bool {
    match group {
        "manager" | "self" => count >= 1,
        "peer" | "report" => count >= GROUP_FLOOR,
        _ => false,
    }
}

/// Check a submitted score map against the declared competencies:
/// every declared competency exactly once, every score on the 1–5
/// scale, nothing undeclared.
///
/// # Errors
///
/// A human-readable refusal naming the first problem found.
pub fn check_scores(declared: &[String], scores: &BTreeMap<String, i32>) -> Result<(), String> {
    for competency in declared {
        match scores.get(competency) {
            None => return Err(format!("missing score for `{competency}`")),
            Some(score) if !(1..=5).contains(score) => {
                return Err(format!("score for `{competency}` must be 1-5, got {score}"));
            }
            Some(_) => {}
        }
    }
    if let Some(extra) = scores.keys().find(|k| !declared.contains(k)) {
        return Err(format!("`{extra}` is not a declared competency"));
    }
    Ok(())
}

/// Aggregate one group's scores for one competency: `(count, mean)`.
/// Empty ⇒ `None` (no responses is not a zero).
#[must_use]
#[allow(clippy::cast_precision_loss)] // display mean
pub fn competency_mean(scores: &[i32]) -> Option<(usize, f64)> {
    if scores.is_empty() {
        return None;
    }
    let total: i64 = scores.iter().map(|&s| i64::from(s)).sum();
    Some((scores.len(), total as f64 / scores.len() as f64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_is_one_way() {
        assert!(transition("draft", "collecting").is_ok());
        assert!(transition("collecting", "shared").is_ok());
        assert!(transition("draft", "shared").is_err(), "no skipping collection");
        assert!(transition("shared", "collecting").is_err(), "terminal");
        assert!(transition("collecting", "draft").is_err(), "nominations freeze");
    }

    /// The WPM-D21 floor: peer/report need 3; manager/self disclose
    /// at 1 by convention; an unknown group never discloses.
    #[test]
    fn group_floor_matrix() {
        assert!(!group_discloses("peer", 2));
        assert!(group_discloses("peer", 3));
        assert!(!group_discloses("report", 2));
        assert!(group_discloses("manager", 1));
        assert!(group_discloses("self", 1));
        assert!(!group_discloses("manager", 0));
        assert!(!group_discloses("astrologer", 99), "unknown group");
    }

    /// Score completeness: every declared competency, on-scale, and
    /// nothing undeclared.
    #[test]
    fn score_check_matrix() {
        let declared = vec!["communication".to_string(), "delivery".to_string()];
        let good: BTreeMap<String, i32> =
            [("communication".into(), 4), ("delivery".into(), 3)].into();
        assert!(check_scores(&declared, &good).is_ok());
        let missing: BTreeMap<String, i32> = [("communication".into(), 4)].into();
        assert!(check_scores(&declared, &missing).unwrap_err().contains("delivery"));
        let off_scale: BTreeMap<String, i32> =
            [("communication".into(), 9), ("delivery".into(), 3)].into();
        assert!(check_scores(&declared, &off_scale).unwrap_err().contains("1-5"));
        let undeclared: BTreeMap<String, i32> = [
            ("communication".into(), 4),
            ("delivery".into(), 3),
            ("astrology".into(), 5),
        ]
        .into();
        assert!(check_scores(&declared, &undeclared).unwrap_err().contains("astrology"));
    }

    #[test]
    fn mean_carries_count_and_never_zeroes_the_empty() {
        assert_eq!(competency_mean(&[]), None, "no responses is not a 0");
        let (count, mean) = competency_mean(&[2, 3, 4]).unwrap();
        assert_eq!(count, 3);
        assert!((mean - 3.0).abs() < f64::EPSILON);
    }
}
