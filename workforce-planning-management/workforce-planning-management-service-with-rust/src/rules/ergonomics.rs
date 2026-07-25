//! Pure rules for ergonomic (DSE) workstation assessments (WPM-R32 /
//! WPM-D24): the default checklist, the completion gate, and the
//! open-issue count. Every item names **equipment or environment** —
//! there is deliberately no symptom in the vocabulary.

/// The default DSE checklist (UK Display Screen Equipment shape).
pub const DSE_ITEMS: &[&str] = &[
    "Screen at a comfortable height and distance, free of flicker",
    "Chair adjustable, with lower-back support",
    "Keyboard and mouse positioned to keep wrists straight",
    "Enough desk space; documents holdable beside the screen",
    "Lighting adequate; screen free of glare and reflections",
    "Leg room clear; feet supported",
    "Work pattern allows breaks or changes of activity",
    "Software and display legible and responsive",
];

/// Completing an assessment requires every item answered — recording
/// an obligation and not enforcing it would make the record worse
/// than useless (the WPM-D15 posture).
///
/// # Errors
///
/// A human-readable refusal counting the unanswered items.
pub fn may_complete(answers: &[Option<bool>]) -> Result<(), String> {
    let unanswered = answers.iter().filter(|a| a.is_none()).count();
    if unanswered == 0 {
        Ok(())
    } else {
        Err(format!(
            "{unanswered} item(s) unanswered — every item needs ok or issue"
        ))
    }
}

/// How many answered items flagged an issue (`ok == false`).
#[must_use]
pub fn open_issues(answers: &[Option<bool>]) -> usize {
    answers.iter().filter(|a| **a == Some(false)).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The default checklist is non-empty and names equipment, not
    /// symptoms — no item mentions the body's state (WPM-D24).
    #[test]
    fn default_checklist_is_about_the_workstation() {
        assert_eq!(DSE_ITEMS.len(), 8);
        for item in DSE_ITEMS {
            let lower = item.to_lowercase();
            for symptom in ["pain", "ache", "strain", "injury", "symptom"] {
                assert!(!lower.contains(symptom), "{item:?} must not name a symptom");
            }
        }
    }

    #[test]
    fn completion_requires_every_answer() {
        assert!(may_complete(&[Some(true), Some(false)]).is_ok());
        let err = may_complete(&[Some(true), None, None]).unwrap_err();
        assert!(err.contains('2'), "counts the unanswered");
        assert!(
            may_complete(&[]).is_ok(),
            "an empty checklist has nothing unanswered"
        );
    }

    #[test]
    fn open_issues_counts_flagged_only() {
        assert_eq!(
            open_issues(&[Some(true), Some(false), None, Some(false)]),
            2
        );
        assert_eq!(open_issues(&[Some(true), None]), 0);
    }
}
