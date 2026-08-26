//! Pure rules for **workflow automation** and the **set-and-forget**
//! scheduler: which rules a board move matches, whether a rule's
//! action is well-formed, and which scheduled actions are due. DB-free
//! and exhaustively unit-tested.
//!
//! Three safety rules shape this module:
//!
//! - **No invented triggers.** A rule with `to_status` unset matches
//!   *every* move of its trigger kind; a rule with it set matches only
//!   that column. A rule scoped to a plan never fires on another plan.
//! - **No automation cascades.** An action that mutates a task's
//!   status is applied **without** re-entering the engine
//!   ([`ACTIONS_THAT_MUTATE_STATUS`]), so one operator move can never
//!   ripple into an unbounded chain.
//! - **Nothing fires early.** [`is_due`] is a plain `due_at <= now`
//!   comparison against a caller-supplied clock; the module has no
//!   clock of its own.

use serde_json::Value;
use uuid::Uuid;

/// What can fire an automation.
pub const TRIGGER_KINDS: &[&str] = &[
    "task_moved",
    "review_submitted",
    "plan_stage_changed",
    // Added 2026-08-26 with the project phase (FR-30 / FR-32). Distinct
    // from `plan_stage_changed`, which is the **governance gate**: the
    // two are separate ordered vocabularies (entity spec §1.5.1), and
    // collapsing them into one trigger would make a rule fire on the
    // wrong kind of change.
    "plan_phase_changed",
];

/// What an automation may do when it fires.
pub const ACTION_KINDS: &[&str] = &[
    "assign",
    "add_label",
    "notify",
    "schedule_action",
    "set_task_status",
];

/// Actions that change a task's status. These are applied without
/// re-entering the engine, so automations cannot cascade.
pub const ACTIONS_THAT_MUTATE_STATUS: &[&str] = &["set_task_status"];

/// What a scheduled action may do when its deadline arrives. A
/// deliberately small set: everything here is either a notification or
/// a status change this service already owns.
pub const SCHEDULED_ACTION_KINDS: &[&str] = &["notify", "expire_review"];

/// Scheduled-action lifecycle.
pub const SCHEDULED_ACTION_STATUSES: &[&str] = &["pending", "fired", "cancelled"];

/// What an automation run did. `skipped` records a rule that matched
/// but could not be applied (e.g. its target had already changed);
/// `failed` records a refusal. Both are logged rather than swallowed.
pub const RUN_OUTCOMES: &[&str] = &["applied", "skipped", "failed"];

/// Longest deadline a `schedule_action` may set, in days. A year is
/// already beyond any board cadence; beyond it the rule is more likely
/// a typo than an intention.
pub const MAX_SCHEDULE_DAYS: i64 = 365;

/// Longest label an `add_label` action may apply (matches the tag
/// convention used by the plan payload).
pub const MAX_LABEL_LEN: usize = 64;

/// The facts of one thing that just happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggerFact {
    /// One of [`TRIGGER_KINDS`].
    pub kind: String,
    /// The plan the subject belongs to.
    pub plan_pid: Uuid,
    /// The status moved out of, when the trigger has one.
    pub from_status: Option<String>,
    /// The status moved into, when the trigger has one.
    pub to_status: Option<String>,
}

/// The matchable surface of one stored automation rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleFact {
    /// Whether the operator has the rule switched on.
    pub enabled: bool,
    /// The plan the rule is scoped to; `None` = every plan.
    pub plan_pid: Option<Uuid>,
    /// One of [`TRIGGER_KINDS`].
    pub trigger_kind: String,
    /// Only fire when moving out of this status; `None` = any.
    pub from_status: Option<String>,
    /// Only fire when moving into this status; `None` = any.
    pub to_status: Option<String>,
}

/// Whether `rule` fires for `fact`.
///
/// A disabled rule never fires. A plan-scoped rule fires only for its
/// own plan. An unset `from_status` / `to_status` on the rule is a
/// wildcard; a set one must match exactly — and if the rule constrains
/// a status the fact does not carry, it does **not** fire (fail
/// closed).
#[must_use]
pub fn rule_matches(rule: &RuleFact, fact: &TriggerFact) -> bool {
    if !rule.enabled || rule.trigger_kind != fact.kind {
        return false;
    }
    if rule.plan_pid.is_some_and(|scoped| scoped != fact.plan_pid) {
        return false;
    }
    let status_matches = |want: Option<&String>, got: Option<&String>| match want {
        None => true,
        Some(want) => got.is_some_and(|got| got == want),
    };
    status_matches(rule.from_status.as_ref(), fact.from_status.as_ref())
        && status_matches(rule.to_status.as_ref(), fact.to_status.as_ref())
}

/// Whether `value` is a member of the closed set `set`.
#[must_use]
pub fn is_token(set: &[&str], value: &str) -> bool {
    set.contains(&value)
}

/// Whether a reference names a person-like entity we can assign work
/// to or notify: an `EntityRef` URN over one of the family's
/// person-bearing services, with a real UUID — the same shape the task
/// and governance controllers already enforce.
#[must_use]
pub fn person_like_ref(value: &str) -> bool {
    value.trim().split_once(':').is_some_and(|(scheme, id)| {
        matches!(scheme, "person" | "worker" | "organization") && Uuid::parse_str(id).is_ok()
    })
}

/// Read a required non-blank string field from an action's value.
fn required_str<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("action_value.{field} is required and must be a non-blank string"))
}

/// Validate one rule's action against its declared kind.
///
/// The action is stored as JSON, so this is the only place its shape
/// is enforced — a malformed rule is refused at write time rather than
/// failing silently at fire time, when nobody is watching.
///
/// # Errors
///
/// A message naming the offending field.
pub fn validate_action(
    action_kind: &str,
    value: &Value,
    task_statuses: &[&str],
) -> Result<(), String> {
    if !is_token(ACTION_KINDS, action_kind) {
        return Err(format!("action_kind must be one of {ACTION_KINDS:?}"));
    }
    if !value.is_object() {
        return Err("action_value must be a JSON object".to_string());
    }
    match action_kind {
        "assign" => {
            let assignee = required_str(value, "assignee_ref")?;
            if !person_like_ref(assignee) {
                return Err(
                    "action_value.assignee_ref must be a person:/worker:/organization: URN"
                        .to_string(),
                );
            }
        }
        "add_label" => {
            let label = required_str(value, "label")?;
            if label.len() > MAX_LABEL_LEN {
                return Err(format!(
                    "action_value.label is capped at {MAX_LABEL_LEN} characters"
                ));
            }
        }
        "notify" => {
            let recipient = required_str(value, "recipient_ref")?;
            if !person_like_ref(recipient) {
                return Err(
                    "action_value.recipient_ref must be a person:/worker:/organization: URN"
                        .to_string(),
                );
            }
        }
        "schedule_action" => {
            let kind = required_str(value, "action_kind")?;
            if !is_token(SCHEDULED_ACTION_KINDS, kind) {
                return Err(format!(
                    "action_value.action_kind must be one of {SCHEDULED_ACTION_KINDS:?}"
                ));
            }
            let days = value
                .get("in_days")
                .and_then(Value::as_i64)
                .ok_or_else(|| "action_value.in_days is required (an integer)".to_string())?;
            if !(1..=MAX_SCHEDULE_DAYS).contains(&days) {
                return Err(format!(
                    "action_value.in_days must be between 1 and {MAX_SCHEDULE_DAYS}"
                ));
            }
            if kind == "notify" {
                let recipient = required_str(value, "recipient_ref")?;
                if !person_like_ref(recipient) {
                    return Err(
                        "action_value.recipient_ref must be a person:/worker:/organization: URN"
                            .to_string(),
                    );
                }
            }
        }
        "set_task_status" => {
            let status = required_str(value, "status")?;
            if !task_statuses.contains(&status) {
                return Err(format!(
                    "action_value.status must be one of {task_statuses:?}"
                ));
            }
        }
        _ => unreachable!("action_kind was checked against ACTION_KINDS above"),
    }
    Ok(())
}

/// Validate a trigger definition: the kind must be known, and the
/// status filters only mean something for status-carrying triggers.
///
/// # Errors
///
/// A message naming the offending field.
pub fn validate_trigger(
    trigger_kind: &str,
    from_status: Option<&str>,
    to_status: Option<&str>,
    task_statuses: &[&str],
) -> Result<(), String> {
    if !is_token(TRIGGER_KINDS, trigger_kind) {
        return Err(format!("trigger_kind must be one of {TRIGGER_KINDS:?}"));
    }
    if trigger_kind == "task_moved" {
        for (field, status) in [("from_status", from_status), ("to_status", to_status)] {
            if let Some(status) = status
                && !task_statuses.contains(&status)
            {
                return Err(format!("{field} must be one of {task_statuses:?}"));
            }
        }
    } else if trigger_kind == "plan_phase_changed" {
        // A phase trigger filters on phases, not task statuses. The
        // vocabularies are disjoint, so validating one against the
        // other would reject every legitimate rule.
        for (field, phase) in [("from_status", from_status), ("to_status", to_status)] {
            if let Some(phase) = phase
                && project_portfolio_management_matcher::PlanPhase::parse(phase).is_none()
            {
                return Err(format!(
                    "{field} must be a project phase (initiating, planning, executing, \
                     controlling, closing) for a `plan_phase_changed` trigger"
                ));
            }
        }
    } else if from_status.is_some() || to_status.is_some() {
        return Err(format!(
            "from_status / to_status only apply to `task_moved` or \
             `plan_phase_changed`, not `{trigger_kind}`"
        ));
    }
    Ok(())
}

/// Whether a scheduled action is due at `now`. Exclusive of cancelled
/// and already-fired rows — those are the caller's filter, but the
/// comparison itself is here so "due" means one thing everywhere.
#[must_use]
pub fn is_due(
    status: &str,
    due_at: chrono::DateTime<chrono::Utc>,
    now: chrono::DateTime<chrono::Utc>,
) -> bool {
    status == "pending" && due_at <= now
}

#[cfg(test)]
mod tests {
    /// A phase trigger filters on **phases**, and a task trigger on
    /// **task statuses**. The two vocabularies are disjoint, so each
    /// must be validated against its own — validating a phase against
    /// the task statuses would reject every legitimate phase rule, and
    /// the reverse would let a typo through.
    #[test]
    fn a_phase_trigger_filters_on_phases_not_task_statuses() {
        use super::validate_trigger;
        let task_statuses = ["todo", "in_progress", "done"];

        assert!(
            validate_trigger(
                "plan_phase_changed",
                None,
                Some("executing"),
                &task_statuses
            )
            .is_ok()
        );
        assert!(
            validate_trigger("plan_phase_changed", Some("planning"), None, &task_statuses).is_ok()
        );
        // A task status is not a phase.
        assert!(
            validate_trigger(
                "plan_phase_changed",
                None,
                Some("in_progress"),
                &task_statuses
            )
            .is_err()
        );
        // And a phase is not a task status.
        assert!(validate_trigger("task_moved", None, Some("executing"), &task_statuses).is_err());
        // The gate vocabulary is a third thing, and takes no filters.
        assert!(
            validate_trigger(
                "plan_stage_changed",
                None,
                Some("g2_definition"),
                &task_statuses
            )
            .is_err()
        );
    }

    use super::*;
    use serde_json::json;

    const STATUSES: &[&str] = &["todo", "in_progress", "in_review", "done", "blocked"];

    fn rule(enabled: bool, plan: Option<Uuid>, from: Option<&str>, to: Option<&str>) -> RuleFact {
        RuleFact {
            enabled,
            plan_pid: plan,
            trigger_kind: "task_moved".to_string(),
            from_status: from.map(std::string::ToString::to_string),
            to_status: to.map(std::string::ToString::to_string),
        }
    }

    fn moved(plan: Uuid, from: &str, to: &str) -> TriggerFact {
        TriggerFact {
            kind: "task_moved".to_string(),
            plan_pid: plan,
            from_status: Some(from.to_string()),
            to_status: Some(to.to_string()),
        }
    }

    #[test]
    fn an_unset_status_filter_is_a_wildcard() {
        let plan = Uuid::new_v4();
        assert!(rule_matches(
            &rule(true, None, None, None),
            &moved(plan, "todo", "in_progress")
        ));
        assert!(rule_matches(
            &rule(true, None, None, Some("in_progress")),
            &moved(plan, "todo", "in_progress")
        ));
    }

    #[test]
    fn a_set_status_filter_must_match_exactly() {
        let plan = Uuid::new_v4();
        assert!(!rule_matches(
            &rule(true, None, None, Some("done")),
            &moved(plan, "todo", "in_progress")
        ));
        assert!(!rule_matches(
            &rule(true, None, Some("in_review"), Some("done")),
            &moved(plan, "todo", "done")
        ));
    }

    #[test]
    fn a_disabled_rule_never_fires() {
        let plan = Uuid::new_v4();
        assert!(!rule_matches(
            &rule(false, None, None, None),
            &moved(plan, "todo", "done")
        ));
    }

    #[test]
    fn a_plan_scoped_rule_does_not_fire_on_another_plan() {
        let mine = Uuid::new_v4();
        let theirs = Uuid::new_v4();
        assert!(rule_matches(
            &rule(true, Some(mine), None, None),
            &moved(mine, "todo", "done")
        ));
        assert!(!rule_matches(
            &rule(true, Some(mine), None, None),
            &moved(theirs, "todo", "done")
        ));
    }

    #[test]
    fn a_rule_for_a_different_trigger_kind_does_not_fire() {
        let plan = Uuid::new_v4();
        let mut r = rule(true, None, None, None);
        r.trigger_kind = "review_submitted".to_string();
        assert!(!rule_matches(&r, &moved(plan, "todo", "done")));
    }

    #[test]
    fn a_status_constrained_rule_fails_closed_on_a_statusless_fact() {
        let fact = TriggerFact {
            kind: "task_moved".to_string(),
            plan_pid: Uuid::new_v4(),
            from_status: None,
            to_status: None,
        };
        assert!(!rule_matches(&rule(true, None, None, Some("done")), &fact));
        assert!(rule_matches(&rule(true, None, None, None), &fact));
    }

    #[test]
    fn assign_actions_require_a_person_like_reference() {
        let who = format!("person:{}", Uuid::new_v4());
        assert!(validate_action("assign", &json!({ "assignee_ref": who }), STATUSES).is_ok());
        assert!(validate_action("assign", &json!({"assignee_ref": "nurse-jo"}), STATUSES).is_err());
        // A person-like prefix with a non-UUID tail is still not a ref.
        assert!(
            validate_action(
                "assign",
                &json!({"assignee_ref": "person:abc-123"}),
                STATUSES
            )
            .is_err()
        );
        assert!(validate_action("assign", &json!({}), STATUSES).is_err());
    }

    #[test]
    fn label_actions_are_bounded() {
        assert!(validate_action("add_label", &json!({"label": "fast-track"}), STATUSES).is_ok());
        let long = "x".repeat(MAX_LABEL_LEN + 1);
        assert!(validate_action("add_label", &json!({ "label": long }), STATUSES).is_err());
        assert!(validate_action("add_label", &json!({"label": "  "}), STATUSES).is_err());
    }

    #[test]
    fn scheduled_actions_bound_their_horizon_and_kind() {
        let ok = json!({"action_kind": "expire_review", "in_days": 14});
        assert!(validate_action("schedule_action", &ok, STATUSES).is_ok());
        let too_far = json!({"action_kind": "expire_review", "in_days": MAX_SCHEDULE_DAYS + 1});
        assert!(validate_action("schedule_action", &too_far, STATUSES).is_err());
        let zero = json!({"action_kind": "expire_review", "in_days": 0});
        assert!(validate_action("schedule_action", &zero, STATUSES).is_err());
        let unknown = json!({"action_kind": "delete_everything", "in_days": 1});
        assert!(validate_action("schedule_action", &unknown, STATUSES).is_err());
        // A scheduled notify still needs somewhere to send it.
        let notify_without_recipient = json!({"action_kind": "notify", "in_days": 3});
        assert!(validate_action("schedule_action", &notify_without_recipient, STATUSES).is_err());
        let notify_with_recipient = json!({
            "action_kind": "notify", "in_days": 3,
            "recipient_ref": format!("worker:{}", Uuid::new_v4()),
        });
        assert!(validate_action("schedule_action", &notify_with_recipient, STATUSES).is_ok());
    }

    #[test]
    fn set_task_status_actions_must_name_a_real_column() {
        assert!(validate_action("set_task_status", &json!({"status": "done"}), STATUSES).is_ok());
        assert!(
            validate_action("set_task_status", &json!({"status": "shipped"}), STATUSES).is_err()
        );
    }

    #[test]
    fn unknown_action_kinds_and_non_objects_are_refused() {
        assert!(validate_action("launch_rocket", &json!({}), STATUSES).is_err());
        assert!(validate_action("add_label", &json!("fast-track"), STATUSES).is_err());
    }

    #[test]
    fn triggers_validate_their_status_filters() {
        assert!(validate_trigger("task_moved", None, Some("done"), STATUSES).is_ok());
        assert!(validate_trigger("task_moved", None, Some("shipped"), STATUSES).is_err());
        assert!(validate_trigger("launch", None, None, STATUSES).is_err());
    }

    #[test]
    fn status_filters_are_refused_on_statusless_triggers() {
        let err = validate_trigger("review_submitted", None, Some("done"), STATUSES)
            .expect_err("must refuse");
        assert!(err.contains("task_moved"), "{err}");
        assert!(validate_trigger("review_submitted", None, None, STATUSES).is_ok());
    }

    #[test]
    fn due_is_a_plain_comparison_and_only_for_pending_rows() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-07-22T12:00:00Z")
            .expect("fixed timestamp")
            .with_timezone(&chrono::Utc);
        let earlier = now - chrono::Duration::minutes(1);
        let later = now + chrono::Duration::minutes(1);
        assert!(is_due("pending", earlier, now));
        assert!(is_due("pending", now, now), "due exactly now is due");
        assert!(!is_due("pending", later, now));
        assert!(!is_due("cancelled", earlier, now));
        assert!(!is_due("fired", earlier, now), "never fires twice");
    }

    #[test]
    fn status_mutating_actions_are_declared_so_the_engine_can_avoid_cascades() {
        assert!(ACTIONS_THAT_MUTATE_STATUS.contains(&"set_task_status"));
        for action in ACTIONS_THAT_MUTATE_STATUS {
            assert!(
                ACTION_KINDS.contains(action),
                "{action} must be a real action"
            );
        }
    }
}
