//! Pure rules for learning & development: proficiency scale, the
//! mentorship lifecycle, and the honest path-progress derivation. No
//! I/O; progress counts only real course completions, never
//! interpolated.

/// Skill categories.
pub const SKILL_CATEGORIES: &[&str] = &["technical", "leadership", "compliance", "domain", "other"];

/// The proficiency / target scale (declared 1–5).
#[must_use]
pub fn valid_proficiency(level: i32) -> bool {
    (1..=5).contains(&level)
}

/// Mentorship statuses.
pub const MENTORSHIP_STATUSES: &[&str] = &["proposed", "active", "completed", "ended"];

/// The mentorship lifecycle: proposed → active → completed, and
/// `ended` reachable from proposed or active. Terminal states do not
/// transition.
///
/// # Errors
///
/// A human-readable refusal naming the legal moves.
pub fn mentorship_transition(current: &str, to: &str) -> Result<(), String> {
    if !MENTORSHIP_STATUSES.contains(&to) {
        return Err(format!("unknown status `{to}` (statuses: {MENTORSHIP_STATUSES:?})"));
    }
    let ok = matches!(
        (current, to),
        ("proposed", "active" | "ended") | ("active", "completed" | "ended")
    );
    if ok {
        Ok(())
    } else {
        Err(format!(
            "illegal transition `{current}` → `{to}` (proposed→active→completed, or end an open one)"
        ))
    }
}

/// Path progress: how many of `step_courses` a member has completed,
/// given the set of course refs they have **completed** enrolments
/// for. Returns `(completed, total)`; `total` is the step count.
/// Counts real completions only.
#[must_use]
pub fn path_progress(step_courses: &[String], completed_courses: &[String]) -> (usize, usize) {
    let done = step_courses
        .iter()
        .filter(|course| completed_courses.iter().any(|c| c == *course))
        .count();
    (done, step_courses.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proficiency_is_one_to_five() {
        assert!(valid_proficiency(1) && valid_proficiency(5));
        assert!(!valid_proficiency(0) && !valid_proficiency(6));
    }

    #[test]
    fn mentorship_lifecycle() {
        assert!(mentorship_transition("proposed", "active").is_ok());
        assert!(mentorship_transition("active", "completed").is_ok());
        assert!(mentorship_transition("proposed", "ended").is_ok());
        assert!(mentorship_transition("proposed", "completed").is_err(), "must activate first");
        assert!(mentorship_transition("completed", "active").is_err(), "terminal");
        assert!(mentorship_transition("active", "sideways").is_err(), "unknown");
    }

    #[test]
    fn progress_counts_only_real_completions() {
        let steps = vec!["course:a".to_string(), "course:b".to_string(), "course:c".to_string()];
        let done = vec!["course:a".to_string(), "course:c".to_string(), "course:z".to_string()];
        assert_eq!(path_progress(&steps, &done), (2, 3));
        assert_eq!(path_progress(&steps, &[]), (0, 3));
    }
}
