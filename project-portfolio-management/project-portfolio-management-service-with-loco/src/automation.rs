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

use serde_json::{Value, json};
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
    // Added 2026-09-02 (FR-32's "a date arriving"), scoped narrowly to
    // `milestones.due` — the one dated field in this service with an
    // unambiguous "arrived" reading. Unlike every trigger above, this
    // one is not fired from a single write: a due date stays true every
    // time a sweep looks at it, so firing needs its own exactly-once
    // claim (`automation_milestone_fires`,
    // `controllers::automation::sweep_milestone_due`) rather than a
    // one-shot call from `fire()`. Carries no status filter — see
    // `validate_trigger` below.
    "milestone_due",
    // Added 2026-09-18 (T-28g), narrowed to `plans.target_date` — "the
    // plan's end" — the same way `milestone_due` was narrowed to one
    // field rather than guessing a general task-date convention. Fired
    // from a single write (`controllers::plans::update`), like
    // `plan_phase_changed`. Carries no status filter, but repurposes
    // `TriggerFact::from_status`/`to_status` to carry the **old/new
    // date strings themselves** rather than a filterable status — see
    // `validate_trigger` below.
    "plan_timeframe_changed",
    // Added 2026-09-18 (T-28g), the milestone analogue of the above —
    // `milestones.due` changing via the new `PUT
    // /plans/{pid}/milestones/{m_pid}` reschedule endpoint (T-28g also
    // added this write path; none existed before, since a milestone
    // could previously only be created and completed).
    "milestone_due_changed",
];

/// What an automation may do when it fires.
pub const ACTION_KINDS: &[&str] = &[
    "assign",
    "add_label",
    "notify",
    "schedule_action",
    "set_task_status",
    // Added 2026-09-18 (T-28g). Computes each direct finish-start
    // successor's implied new dates and writes a notification carrying
    // the proposal — it moves nothing.
    "propose_reschedule",
    // Added 2026-09-18 (T-28g). The opt-in twin of `propose_reschedule`:
    // applies the same computation instead of only proposing it.
    "shift_dependents",
];

/// Actions that change a task's status. These are applied without
/// re-entering the engine, so automations cannot cascade.
pub const ACTIONS_THAT_MUTATE_STATUS: &[&str] = &["set_task_status"];

/// Actions that change a plan's own timeframe. Applied the same
/// no-cascade way `ACTIONS_THAT_MUTATE_STATUS` already is: writing a
/// successor's new dates does **not** re-fire `plan_timeframe_changed`
/// on it, or a chain of dependencies would cascade through the engine
/// rather than stopping at the one deadline that actually moved.
pub const ACTIONS_THAT_MUTATE_TIMEFRAME: &[&str] = &["shift_dependents"];

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
        // `propose_reschedule` needs the identical shape: who gets
        // told about the proposed shifts.
        "notify" | "propose_reschedule" => {
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
        // No parameters: re-derives the same computation
        // `propose_reschedule` would, applying it instead of only
        // proposing it.
        "shift_dependents" => {}
        _ => unreachable!("action_kind was checked against ACTION_KINDS above"),
    }
    Ok(())
}

/// Highest number of actions one rule may declare. A bound, not a
/// design target: nothing about the engine needs a cap, but an
/// unbounded array is an unbounded number of `automation_runs` rows per
/// firing, and a request body nobody could plausibly declare by hand
/// past a few dozen is more likely a mistake than an intention.
pub const MAX_ACTIONS_PER_RULE: usize = 20;

/// One parsed, already-validated action from a rule's `actions` array.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedAction {
    /// One of [`ACTION_KINDS`].
    pub kind: String,
    /// The action's own parameters, already checked by [`validate_action`].
    pub value: Value,
}

/// Validate a rule's **ordered list** of actions (FR-32: "more than one
/// action per rule, applied in declared order").
///
/// Array order **is** declared order — there is no separate position
/// field to keep in sync with it. Every element is validated with
/// [`validate_action`]; the first problem found names its 0-based index
/// so a caller with several actions can tell which one is wrong.
///
/// # Errors
///
/// A message naming the offending index and field, or the empty-array /
/// too-many-actions / malformed-element case.
pub fn validate_actions(
    actions: &[Value],
    task_statuses: &[&str],
) -> Result<Vec<ParsedAction>, String> {
    if actions.is_empty() {
        return Err("actions must declare at least one action".to_string());
    }
    if actions.len() > MAX_ACTIONS_PER_RULE {
        return Err(format!(
            "actions may declare at most {MAX_ACTIONS_PER_RULE}, found {}",
            actions.len()
        ));
    }
    let mut parsed = Vec::with_capacity(actions.len());
    for (index, action) in actions.iter().enumerate() {
        let kind = action
            .get("kind")
            .and_then(Value::as_str)
            .filter(|k| !k.trim().is_empty())
            .ok_or_else(|| format!("actions[{index}].kind is required"))?;
        let value = action.get("value").cloned().unwrap_or_else(|| json!({}));
        validate_action(kind, &value, task_statuses)
            .map_err(|e| format!("actions[{index}]: {e}"))?;
        parsed.push(ParsedAction {
            kind: kind.to_string(),
            value,
        });
    }
    Ok(parsed)
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

// -- T-28g: deadline-shift trigger and rescheduling ---------------------

/// One direct finish-start successor of a shifted item, as the caller
/// already has it loaded (one query, no N+1).
#[derive(Debug, Clone, Copy)]
pub struct SuccessorFact {
    /// The `plan_dependencies` edge.
    pub edge_pid: Uuid,
    /// The successor plan.
    pub successor_pid: Uuid,
    /// May start this many days after the predecessor finishes.
    pub lag_days: i32,
    /// The successor's own current start, when it has one.
    pub current_start: Option<chrono::NaiveDate>,
    /// The successor's own current end, when it has one.
    pub current_end: Option<chrono::NaiveDate>,
}

/// One proposed (or applied) shift of a direct successor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct ProposedShift {
    /// The `plan_dependencies` edge this shift is proposed through.
    pub edge_pid: Uuid,
    /// The successor plan.
    pub successor_pid: Uuid,
    /// Same sign and magnitude as the predecessor's own move — a
    /// finish-start lag is a fixed gap, so shifting the predecessor's
    /// end by N days shifts the successor's implied dates by the same
    /// N days, whatever the lag is. `+5` is +5 regardless of lag; lag
    /// only ever appears in the **violation** check below, not in the
    /// shift amount itself.
    pub delta_days: i64,
    /// `current_start + delta_days`, when the successor has a start.
    pub proposed_start: Option<chrono::NaiveDate>,
    /// `current_end + delta_days`, when the successor has an end.
    pub proposed_end: Option<chrono::NaiveDate>,
    /// Whether the successor's dependency was **already** violated
    /// before this shift (`current_start < old predecessor end +
    /// lag`) — named rather than silently proposed over. Applying the
    /// same delta to both ends of an already-violated pair cannot fix
    /// or worsen the violation, so this is computed once, not
    /// recomputed against the proposed dates.
    pub already_violated: bool,
}

/// Compute the direct successors' implied shift from one plan's
/// timeframe change.
///
/// `old_end` / `new_end` are the shifted plan's own `target_date`
/// before and after the write that fired the trigger; `delta_days` is
/// derived from the two, not supplied separately, so a caller cannot
/// pass a delta inconsistent with the dates it also passes.
#[must_use]
pub fn propose_shifts(
    old_end: chrono::NaiveDate,
    new_end: chrono::NaiveDate,
    successors: &[SuccessorFact],
) -> Vec<ProposedShift> {
    let delta_days = (new_end - old_end).num_days();
    successors
        .iter()
        .map(|s| {
            let lag = chrono::Days::new(u64::try_from(s.lag_days.max(0)).unwrap_or(0));
            let old_earliest = old_end + lag;
            let already_violated = s.current_start.is_some_and(|start| start < old_earliest);
            ProposedShift {
                edge_pid: s.edge_pid,
                successor_pid: s.successor_pid,
                delta_days,
                proposed_start: s
                    .current_start
                    .and_then(|d| d.checked_add_signed(chrono::Duration::days(delta_days))),
                proposed_end: s
                    .current_end
                    .and_then(|d| d.checked_add_signed(chrono::Duration::days(delta_days))),
                already_violated,
            }
        })
        .collect()
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
    fn an_empty_actions_array_is_refused() {
        let err = validate_actions(&[], STATUSES).expect_err("must refuse");
        assert!(err.contains("at least one"), "{err}");
    }

    #[test]
    fn too_many_actions_is_refused() {
        let who = format!("person:{}", Uuid::new_v4());
        let actions: Vec<Value> = (0..=MAX_ACTIONS_PER_RULE)
            .map(|_| json!({ "kind": "assign", "value": { "assignee_ref": who } }))
            .collect();
        let err = validate_actions(&actions, STATUSES).expect_err("must refuse");
        assert!(err.contains("at most"), "{err}");
    }

    #[test]
    fn actions_are_validated_in_declared_order_and_the_index_names_the_bad_one() {
        let who = format!("person:{}", Uuid::new_v4());
        let actions = vec![
            json!({ "kind": "assign", "value": { "assignee_ref": who } }),
            json!({ "kind": "add_label", "value": { "label": "" } }),
        ];
        let err = validate_actions(&actions, STATUSES).expect_err("must refuse");
        assert!(err.starts_with("actions[1]:"), "{err}");
    }

    #[test]
    fn a_well_formed_action_list_parses_in_order() {
        let who = format!("person:{}", Uuid::new_v4());
        let actions = vec![
            json!({ "kind": "assign", "value": { "assignee_ref": who } }),
            json!({ "kind": "add_label", "value": { "label": "fast-track" } }),
        ];
        let parsed = validate_actions(&actions, STATUSES).expect("valid");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].kind, "assign");
        assert_eq!(parsed[1].kind, "add_label");
        assert_eq!(parsed[1].value["label"], "fast-track");
    }

    #[test]
    fn an_action_with_no_kind_is_refused() {
        let err = validate_actions(&[json!({ "value": {} })], STATUSES).expect_err("must refuse");
        assert!(err.contains("actions[0].kind"), "{err}");
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
    fn milestone_due_is_a_recognised_statusless_trigger() {
        assert!(is_token(TRIGGER_KINDS, "milestone_due"));
        assert!(validate_trigger("milestone_due", None, None, STATUSES).is_ok());
        // A date arriving has no from/to status of any kind — neither a
        // task status nor a phase.
        assert!(validate_trigger("milestone_due", None, Some("done"), STATUSES).is_err());
    }

    #[test]
    fn a_milestone_due_rule_matches_its_own_plan_only() {
        let mine = Uuid::new_v4();
        let theirs = Uuid::new_v4();
        let rule = RuleFact {
            enabled: true,
            plan_pid: Some(mine),
            trigger_kind: "milestone_due".to_string(),
            from_status: None,
            to_status: None,
        };
        let fact_for = |plan: Uuid| TriggerFact {
            kind: "milestone_due".to_string(),
            plan_pid: plan,
            from_status: None,
            to_status: None,
        };
        assert!(rule_matches(&rule, &fact_for(mine)));
        assert!(!rule_matches(&rule, &fact_for(theirs)));
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

    #[test]
    fn timeframe_mutating_actions_are_declared_so_the_engine_can_avoid_cascades() {
        assert!(ACTIONS_THAT_MUTATE_TIMEFRAME.contains(&"shift_dependents"));
        for action in ACTIONS_THAT_MUTATE_TIMEFRAME {
            assert!(
                ACTION_KINDS.contains(action),
                "{action} must be a real action"
            );
        }
    }

    // -- T-28g: propose_shifts ------------------------------------------

    fn date(y: i32, m: u32, d: u32) -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    /// **Shifting a plan's end by 5 days proposes +5 on a finish-start
    /// successor** — the exact acceptance wording — whatever the lag
    /// is, because a fixed lag does not change how a delta propagates
    /// through it.
    #[test]
    fn a_five_day_shift_proposes_five_days_on_the_successor_whatever_the_lag() {
        let successors = [
            SuccessorFact {
                edge_pid: Uuid::from_u128(1),
                successor_pid: Uuid::from_u128(2),
                lag_days: 0,
                current_start: Some(date(2026, 2, 1)),
                current_end: Some(date(2026, 2, 10)),
            },
            SuccessorFact {
                edge_pid: Uuid::from_u128(3),
                successor_pid: Uuid::from_u128(4),
                lag_days: 10,
                current_start: Some(date(2026, 2, 11)),
                current_end: Some(date(2026, 2, 20)),
            },
        ];
        let shifts = propose_shifts(date(2026, 1, 31), date(2026, 2, 5), &successors);
        assert_eq!(shifts[0].delta_days, 5);
        assert_eq!(shifts[0].proposed_start, Some(date(2026, 2, 6)));
        assert_eq!(shifts[1].delta_days, 5, "the lag does not change the delta");
        assert_eq!(shifts[1].proposed_start, Some(date(2026, 2, 16)));
    }

    /// A negative shift (pulled earlier) proposes a negative delta.
    #[test]
    fn a_pulled_forward_end_proposes_a_negative_delta() {
        let successors = [SuccessorFact {
            edge_pid: Uuid::from_u128(1),
            successor_pid: Uuid::from_u128(2),
            lag_days: 0,
            current_start: Some(date(2026, 2, 10)),
            current_end: None,
        }];
        let shifts = propose_shifts(date(2026, 2, 1), date(2026, 1, 27), &successors);
        assert_eq!(shifts[0].delta_days, -5);
        assert_eq!(shifts[0].proposed_start, Some(date(2026, 2, 5)));
    }

    /// A successor whose dependency was **already violated** before
    /// the shift is named as such, not silently proposed over.
    #[test]
    fn an_already_violated_successor_is_named() {
        // Predecessor ends 2026-02-01, lag 5 ⇒ earliest_start 2026-02-06.
        // The successor already starts 2026-02-03 — before that — so
        // the dependency was already violated, before any shift.
        let successors = [SuccessorFact {
            edge_pid: Uuid::from_u128(1),
            successor_pid: Uuid::from_u128(2),
            lag_days: 5,
            current_start: Some(date(2026, 2, 3)),
            current_end: None,
        }];
        let shifts = propose_shifts(date(2026, 2, 1), date(2026, 2, 8), &successors);
        assert!(shifts[0].already_violated);

        // A successor that already respects the earliest start is not
        // flagged.
        let clean = [SuccessorFact {
            edge_pid: Uuid::from_u128(3),
            successor_pid: Uuid::from_u128(4),
            lag_days: 5,
            current_start: Some(date(2026, 2, 10)),
            current_end: None,
        }];
        let ok = propose_shifts(date(2026, 2, 1), date(2026, 2, 8), &clean);
        assert!(!ok[0].already_violated);
    }

    /// A successor with no dates at all proposes no dates, and is
    /// never treated as violated (there is nothing to violate).
    #[test]
    fn an_undated_successor_proposes_nothing_and_is_never_violated() {
        let successors = [SuccessorFact {
            edge_pid: Uuid::from_u128(1),
            successor_pid: Uuid::from_u128(2),
            lag_days: 0,
            current_start: None,
            current_end: None,
        }];
        let shifts = propose_shifts(date(2026, 2, 1), date(2026, 2, 6), &successors);
        assert_eq!(shifts[0].proposed_start, None);
        assert_eq!(shifts[0].proposed_end, None);
        assert!(!shifts[0].already_violated);
    }

    /// A zero-day "shift" (no actual change) proposes zero on every
    /// successor — pinned so a caller cannot accidentally fire this
    /// off a no-op write.
    #[test]
    fn a_zero_day_shift_proposes_nothing() {
        let successors = [SuccessorFact {
            edge_pid: Uuid::from_u128(1),
            successor_pid: Uuid::from_u128(2),
            lag_days: 0,
            current_start: Some(date(2026, 2, 1)),
            current_end: Some(date(2026, 2, 10)),
        }];
        let shifts = propose_shifts(date(2026, 2, 1), date(2026, 2, 1), &successors);
        assert_eq!(shifts[0].delta_days, 0);
        assert_eq!(shifts[0].proposed_start, Some(date(2026, 2, 1)));
    }
}
