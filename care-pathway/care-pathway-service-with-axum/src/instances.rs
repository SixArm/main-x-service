//! Pure rules for the care-pathway instance layer — the enrolment
//! status lifecycle, urgency + care-team vocabularies, the review
//! hygiene helper, and stalled-journey detection (spec `13-tasks.md`
//! T-14j). No I/O.

/// Instance statuses.
pub const INSTANCE_STATUSES: &[&str] = &["active", "on_hold", "completed", "discontinued"];

/// Urgency levels (routine → emergency).
pub const URGENCY_LEVELS: &[&str] = &["routine", "urgent", "emergency"];

/// Care-team roles.
pub const TEAM_ROLES: &[&str] = &[
    "lead_clinician",
    "gp",
    "specialist",
    "nurse",
    "mental_health",
    "coordinator",
    "other",
];

/// Recorded closure outcomes (declared at close, never inferred).
pub const OUTCOMES: &[&str] = &[
    "improved",
    "stable",
    "deteriorated",
    "deceased",
    "transferred",
    "not_achieved",
    "other",
];

/// Instance event kinds (recorded, not inferred).
pub const EVENT_KINDS: &[&str] = &["note", "review", "escalation", "de_escalation", "referral"];

/// Whether a status is terminal (no further transitions).
#[must_use]
pub fn is_terminal(status: &str) -> bool {
    matches!(status, "completed" | "discontinued")
}

/// The enrolment lifecycle: `active` ↔ `on_hold`, and either may go to
/// a terminal `completed` / `discontinued`. Terminal states do not
/// transition.
///
/// # Errors
///
/// A human-readable refusal.
pub fn instance_transition(current: &str, to: &str) -> Result<(), String> {
    if !INSTANCE_STATUSES.contains(&to) {
        return Err(format!(
            "unknown status `{to}` (statuses: {INSTANCE_STATUSES:?})"
        ));
    }
    if is_terminal(current) {
        return Err(format!("`{current}` is terminal and does not transition"));
    }
    let ok = matches!(
        (current, to),
        ("active", "on_hold" | "completed" | "discontinued")
            | ("on_hold", "active" | "completed" | "discontinued")
    );
    if ok {
        Ok(())
    } else {
        Err(format!("illegal transition `{current}` → `{to}`"))
    }
}

/// The closed, ordered vocabulary [`LastActivity::source`] draws from
/// (spec T-14j). `review` is deliberately not a separate entry:
/// `POST .../review` already records an `instance_events` row (`kind:
/// "review"`), so it is covered by `event` without a second,
/// redundant signal to keep in sync.
pub const ACTIVITY_SOURCES: &[&str] = &[
    "segment_start",
    "segment_end",
    "step_done",
    "event",
    "enrolled_on",
];

/// The instant of an instance's most recent tracked activity, and
/// which source it came from — never grouped by actor (spec T-14j).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LastActivity {
    /// Epoch milliseconds of the most recent tracked activity.
    pub at_ms: i64,
    /// One of [`ACTIVITY_SOURCES`].
    pub source: &'static str,
}

/// Fold an instance's tracked activity sources into the single most
/// recent one. `enrolled_on_ms` is the floor every instance has from
/// the moment it exists — a brand-new instance with no segments,
/// steps, or events yet is never "stalled since forever".
#[must_use]
pub fn last_activity(
    enrolled_on_ms: i64,
    latest_segment_start_ms: Option<i64>,
    latest_segment_end_ms: Option<i64>,
    latest_step_done_ms: Option<i64>,
    latest_event_ms: Option<i64>,
) -> LastActivity {
    let mut best = LastActivity {
        at_ms: enrolled_on_ms,
        source: "enrolled_on",
    };
    for (at, source) in [
        (latest_segment_start_ms, "segment_start"),
        (latest_segment_end_ms, "segment_end"),
        (latest_step_done_ms, "step_done"),
        (latest_event_ms, "event"),
    ] {
        if let Some(at) = at
            && at > best.at_ms
        {
            best = LastActivity { at_ms: at, source };
        }
    }
    best
}

/// Whether an instant this stale counts as stalled: strictly more than
/// `idle_days` days before `as_of_ms`. The comparison is retroactive —
/// idle-since is `activity_ms` itself, not when the silence happened
/// to be noticed (IPPA's own `Process.time_out` framing). Whether the
/// instance is open at all is the caller's job (a closed instance is
/// never listed, regardless of how stale its last activity reads).
#[must_use]
pub fn is_stalled(activity_ms: i64, as_of_ms: i64, idle_days: i64) -> bool {
    let idle_ms = idle_days.max(0) * 86_400_000;
    as_of_ms.saturating_sub(activity_ms) > idle_ms
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_holds_and_closes() {
        assert!(instance_transition("active", "on_hold").is_ok());
        assert!(instance_transition("on_hold", "active").is_ok());
        assert!(instance_transition("active", "completed").is_ok());
        assert!(instance_transition("on_hold", "discontinued").is_ok());
        assert!(
            instance_transition("active", "active").is_err(),
            "no self-loop"
        );
        assert!(
            instance_transition("completed", "active").is_err(),
            "terminal"
        );
        assert!(
            instance_transition("active", "sideways").is_err(),
            "unknown"
        );
    }

    #[test]
    fn terminal_states() {
        assert!(is_terminal("completed") && is_terminal("discontinued"));
        assert!(!is_terminal("active") && !is_terminal("on_hold"));
    }

    const DAY_MS: i64 = 86_400_000;

    /// Acceptance: an instance whose last event was 61 days ago is
    /// listed at `idle_days=60` and not at 90.
    #[test]
    fn sixty_one_days_idle_crosses_sixty_but_not_ninety() {
        let activity = last_activity(0, None, None, None, Some(61 * DAY_MS));
        assert_eq!(activity.at_ms, 61 * DAY_MS);
        assert_eq!(activity.source, "event");
        let as_of = 122 * DAY_MS; // "now" = 61 days after that event
        assert!(is_stalled(activity.at_ms, as_of, 60));
        assert!(!is_stalled(activity.at_ms, as_of, 90));
    }

    /// Acceptance: an instance with an open segment started 5 days
    /// ago is not listed — the segment's own start counts as recent
    /// activity even though it has no end yet.
    #[test]
    fn a_recently_started_open_segment_is_not_stalled() {
        let as_of = 100 * DAY_MS;
        let started = as_of - 5 * DAY_MS;
        let activity = last_activity(0, Some(started), None, None, None);
        assert_eq!(activity.at_ms, started);
        assert_eq!(activity.source, "segment_start");
        assert!(!is_stalled(activity.at_ms, as_of, 60));
    }

    /// The latest source wins regardless of which one it is.
    #[test]
    fn the_most_recent_source_wins() {
        let activity = last_activity(
            0,
            Some(10 * DAY_MS),
            Some(20 * DAY_MS),
            Some(30 * DAY_MS),
            Some(40 * DAY_MS),
        );
        assert_eq!(activity.at_ms, 40 * DAY_MS);
        assert_eq!(activity.source, "event");
    }

    /// A brand-new instance with nothing recorded yet falls back to
    /// `enrolled_on` — never "stalled since forever".
    #[test]
    fn a_fresh_instance_falls_back_to_enrolled_on() {
        let activity = last_activity(5 * DAY_MS, None, None, None, None);
        assert_eq!(activity.at_ms, 5 * DAY_MS);
        assert_eq!(activity.source, "enrolled_on");
    }

    /// Exactly `idle_days` old is not yet stalled — the comparison is
    /// strictly greater than, matching the "older than N days" text.
    #[test]
    fn exactly_the_threshold_is_not_yet_stalled() {
        let as_of = 60 * DAY_MS;
        assert!(!is_stalled(0, as_of, 60));
        assert!(is_stalled(0, as_of + 1, 60));
    }

    /// A negative `idle_days` is clamped to zero rather than making
    /// every instance appear even more overdue than it is.
    #[test]
    fn negative_idle_days_is_clamped_to_zero() {
        assert!(is_stalled(0, DAY_MS, -5));
    }
}
