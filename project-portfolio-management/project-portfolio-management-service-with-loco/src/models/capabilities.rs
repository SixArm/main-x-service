//! Model glue for the collaboration / automation capability tables:
//! `ActiveModelBehavior` impls plus the finders and the small set of
//! writes shared by more than one controller (the notification insert
//! is written by the automation engine *and* by the scheduled-action
//! sweep, so it lives here rather than being duplicated).

use loco_rs::prelude::*;
use sea_orm::{ConnectionTrait, QueryOrder, QuerySelect};
use uuid::Uuid;

use super::_entities::{automation_runs, automations, notifications, reviews, scheduled_actions};

impl ActiveModelBehavior for reviews::ActiveModel {}
impl ActiveModelBehavior for automations::ActiveModel {}
impl ActiveModelBehavior for automation_runs::ActiveModel {}
impl ActiveModelBehavior for scheduled_actions::ActiveModel {}
impl ActiveModelBehavior for notifications::ActiveModel {}

/// Generate a `find_<x>` finder over an active (not soft-deleted) row.
macro_rules! find_active {
    ($fn_name:ident, $module:ident) => {
        /// Find the active row by public id.
        ///
        /// # Errors
        ///
        /// [`Error::NotFound`] when absent or soft-deleted.
        pub async fn $fn_name<C: ConnectionTrait>(db: &C, pid: Uuid) -> Result<$module::Model> {
            $module::Entity::find()
                .filter($module::Column::Pid.eq(pid))
                .filter($module::Column::DeletedAt.is_null())
                .one(db)
                .await
                .map_err(|e| Error::Model(ModelError::from(e)))?
                .ok_or(Error::NotFound)
        }
    };
}

find_active!(find_review, reviews);
find_active!(find_automation, automations);
find_active!(find_scheduled_action, scheduled_actions);
find_active!(find_notification, notifications);

/// Every live review invitation on one subject, oldest first.
///
/// # Errors
///
/// Propagates database errors.
pub async fn reviews_for_subject<C: ConnectionTrait>(
    db: &C,
    subject_kind: &str,
    subject_pid: Uuid,
) -> Result<Vec<reviews::Model>> {
    reviews::Entity::find()
        .filter(reviews::Column::SubjectKind.eq(subject_kind))
        .filter(reviews::Column::SubjectPid.eq(subject_pid))
        .filter(reviews::Column::DeletedAt.is_null())
        .order_by_asc(reviews::Column::Id)
        .all(db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))
}

/// Every enabled automation whose trigger kind matches, cheapest-first
/// filter before the pure engine decides what actually fires.
///
/// # Errors
///
/// Propagates database errors.
pub async fn enabled_automations<C: ConnectionTrait>(
    db: &C,
    trigger_kind: &str,
) -> Result<Vec<automations::Model>> {
    automations::Entity::find()
        .filter(automations::Column::TriggerKind.eq(trigger_kind))
        .filter(automations::Column::Enabled.eq(true))
        .filter(automations::Column::DeletedAt.is_null())
        .order_by_asc(automations::Column::Id)
        .all(db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))
}

/// Write one in-app notification. In-app only — this service has no
/// email or push transport, and does not pretend to.
///
/// # Errors
///
/// Propagates database errors.
pub async fn notify<C: ConnectionTrait>(
    db: &C,
    recipient_ref: &str,
    subject_kind: &str,
    subject_pid: Uuid,
    kind: &str,
    message: &str,
) -> Result<notifications::Model> {
    notifications::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        recipient_ref: ActiveValue::set(recipient_ref.to_string()),
        subject_kind: ActiveValue::set(subject_kind.to_string()),
        subject_pid: ActiveValue::set(subject_pid),
        kind: ActiveValue::set(kind.to_string()),
        message: ActiveValue::set(message.to_string()),
        read_at: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(|e| Error::Model(ModelError::from(e)))
}

/// Record what one automation firing did. Every run is logged —
/// applied, skipped, or failed — so an automated change is always
/// traceable to the rule that made it.
///
/// # Errors
///
/// Propagates database errors.
pub async fn record_run<C: ConnectionTrait>(
    db: &C,
    automation_pid: Uuid,
    subject_kind: &str,
    subject_pid: Uuid,
    outcome: &str,
    detail: serde_json::Value,
) -> Result<automation_runs::Model> {
    automation_runs::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        automation_pid: ActiveValue::set(automation_pid),
        subject_kind: ActiveValue::set(subject_kind.to_string()),
        subject_pid: ActiveValue::set(subject_pid),
        outcome: ActiveValue::set(outcome.to_string()),
        detail: ActiveValue::set(detail),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(|e| Error::Model(ModelError::from(e)))
}

/// One deadline to hold until it comes due.
#[derive(Debug, Clone)]
pub struct NewScheduledAction {
    /// What the action is about (`idea` / `proposal` / `plan` / `task`).
    pub subject_kind: String,
    /// The subject's pid.
    pub subject_pid: Uuid,
    /// One of `crate::automation::SCHEDULED_ACTION_KINDS`.
    pub action_kind: String,
    /// Action-specific detail (recipient, message, …).
    pub payload: serde_json::Value,
    /// When it comes due.
    pub due_at: chrono::DateTime<chrono::FixedOffset>,
    /// The automation that scheduled it, when it was not a person.
    pub source_automation_pid: Option<Uuid>,
    /// The actor who configured it.
    pub created_by: Option<String>,
}

/// Enqueue a scheduled action ("set and forget"): held until `due_at`,
/// then fired exactly once by the sweep.
///
/// # Errors
///
/// Propagates database errors.
pub async fn schedule<C: ConnectionTrait>(
    db: &C,
    new: NewScheduledAction,
) -> Result<scheduled_actions::Model> {
    let NewScheduledAction {
        subject_kind,
        subject_pid,
        action_kind,
        payload,
        due_at,
        source_automation_pid,
        created_by,
    } = new;
    scheduled_actions::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        subject_kind: ActiveValue::set(subject_kind),
        subject_pid: ActiveValue::set(subject_pid),
        action_kind: ActiveValue::set(action_kind),
        payload: ActiveValue::set(payload),
        due_at: ActiveValue::set(due_at),
        status: ActiveValue::set("pending".to_string()),
        source_automation_pid: ActiveValue::set(source_automation_pid),
        created_by: ActiveValue::set(created_by),
        fired_at: ActiveValue::set(None),
        outcome: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(|e| Error::Model(ModelError::from(e)))
}

/// The pending scheduled actions due at or before `now`, oldest
/// deadline first, capped so one sweep can never run unbounded.
///
/// # Errors
///
/// Propagates database errors.
pub async fn due_actions<C: ConnectionTrait>(
    db: &C,
    now: chrono::DateTime<chrono::FixedOffset>,
    cap: u64,
) -> Result<Vec<scheduled_actions::Model>> {
    scheduled_actions::Entity::find()
        .filter(scheduled_actions::Column::Status.eq("pending"))
        .filter(scheduled_actions::Column::DueAt.lte(now))
        .filter(scheduled_actions::Column::DeletedAt.is_null())
        .order_by_asc(scheduled_actions::Column::DueAt)
        .limit(cap)
        .all(db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))
}
