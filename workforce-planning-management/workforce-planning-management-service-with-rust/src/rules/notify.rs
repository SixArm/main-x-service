//! Pure rules for in-app notifications (WPM-R31): the kind vocabulary
//! and the appraisal fan-out — who is told what on which lifecycle
//! move. Reference-only by design (WPM-D23); no I/O.

use uuid::Uuid;

/// The closed notification kinds.
pub const KINDS: &[&str] = &["appraisal_request", "appraisal_shared", "adjustment_update"];

/// Recipients of an appraisal lifecycle move:
/// - `collecting` ⇒ **every** rater (self included — the
///   self-assessment is a task too), each getting an
///   `appraisal_request`;
/// - `shared` ⇒ the subject, getting an `appraisal_shared`;
/// - any other move ⇒ nobody.
#[must_use]
pub fn appraisal_recipients(
    to: &str,
    subject_pid: Uuid,
    rater_pids: &[Uuid],
) -> Vec<(Uuid, &'static str)> {
    match to {
        "collecting" => rater_pids
            .iter()
            .map(|pid| (*pid, "appraisal_request"))
            .collect(),
        "shared" => vec![(subject_pid, "appraisal_shared")],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Collecting notifies every rater (self included); sharing
    /// notifies the subject; drafts notify nobody.
    #[test]
    fn fan_out_matrix() {
        let subject = Uuid::new_v4();
        let peer = Uuid::new_v4();
        let raters = vec![subject, peer];
        let on_collect = appraisal_recipients("collecting", subject, &raters);
        assert_eq!(on_collect.len(), 2);
        assert!(
            on_collect
                .iter()
                .all(|(_, kind)| *kind == "appraisal_request")
        );
        assert!(
            on_collect.iter().any(|(pid, _)| *pid == subject),
            "self-assessment is a task"
        );
        let on_share = appraisal_recipients("shared", subject, &raters);
        assert_eq!(on_share, vec![(subject, "appraisal_shared")]);
        assert!(appraisal_recipients("draft", subject, &raters).is_empty());
    }

    #[test]
    fn kinds_are_closed() {
        assert_eq!(
            KINDS,
            &["appraisal_request", "appraisal_shared", "adjustment_update"]
        );
    }
}
