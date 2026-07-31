//! Editorial workflow (CMS-R9–R12): transitions, publishing,
//! scheduling, and locks.
//!
//! Everything here runs inside `SELECT … FOR UPDATE` on the variant
//! row, because every operation is a read-then-write on the same state
//! and two editors pressing publish at the same moment must produce one
//! outcome, not two half-applied ones (CMS-D15).
//!
//! ## Publishing names a revision
//!
//! `publish` sets `published_revision_pid` to a **specific** revision —
//! the current one by default, or an explicitly named earlier one. Two
//! consequences fall out and are pinned by tests:
//!
//! - Editing after publishing changes nothing on the live site until
//!   the next publish. "Save" and "go live" are different verbs.
//! - `first_published_at` survives unpublish and republish, because
//!   "when did this first appear" is a different question from "what is
//!   live now" — and the first one is the one an archive or a legal
//!   query asks.
//!
//! ## What is deliberately missing
//!
//! Unpublish should leave a redirect to a declared replacement, or a
//! `410 Gone` marker, rather than a bare 404 (CMS-R10). Routes do not
//! exist until CMS-T16, so this records the replacement in the audit
//! row and stops there. Faking a redirect against a routing table that
//! does not exist would be worse than the gap.

use loco_rs::prelude::*;
use sea_orm::{QueryOrder, QuerySelect, TransactionTrait};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::{ensure_valid, unprocessable};
use crate::auth::MaybeAuthUser;
use crate::models::_entities::{entries, entry_variants, redirects, revisions, routes};
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
use crate::rules::lifecycle::{self, Action};
use crate::streaming;
use crate::validation::Problems;

/// Default advisory-lock lifetime.
const DEFAULT_LOCK_SECS: i64 = 900;

/// `POST …/transition` body.
#[derive(Debug, Deserialize)]
struct TransitionPayload {
    /// One of [`crate::rules::lifecycle::ACTIONS`].
    action: String,
    /// Required for reject / unpublish / archive / restore.
    #[serde(default)]
    reason: Option<String>,
    /// Assigned on `submit`.
    #[serde(default)]
    reviewer_ref: Option<String>,
    /// The revision to publish; defaults to the variant's current one.
    #[serde(default)]
    revision_pid: Option<Uuid>,
    /// On unpublish, the entry that replaces this one (recorded now,
    /// wired to a redirect with CMS-T16).
    #[serde(default)]
    replacement_entry_pid: Option<Uuid>,
}

/// What a transition produced.
#[derive(Debug, Serialize)]
struct TransitionView {
    variant_pid: String,
    from: String,
    to: String,
    published_revision_pid: Option<String>,
    first_published_at: Option<chrono::DateTime<chrono::FixedOffset>>,
}

/// `POST …/schedule` body.
#[derive(Debug, Deserialize)]
struct SchedulePayload {
    #[serde(default)]
    publish_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    #[serde(default)]
    unpublish_at: Option<chrono::DateTime<chrono::FixedOffset>>,
}

/// `POST …/lock` body.
#[derive(Debug, Deserialize)]
struct LockPayload {
    #[serde(default)]
    ttl_secs: Option<i64>,
    /// Required to take a lock somebody else holds.
    #[serde(default)]
    reason: Option<String>,
}

/// Load the entry and lock its variant for update.
async fn locked_variant(
    txn: &sea_orm::DatabaseTransaction,
    entry_pid: Uuid,
    locale: &str,
) -> Result<entry_variants::Model> {
    entry_variants::Entity::find()
        .filter(entry_variants::Column::EntryPid.eq(entry_pid))
        .filter(entry_variants::Column::Locale.eq(locale))
        .filter(entry_variants::Column::DeletedAt.is_null())
        .lock_exclusive()
        .one(txn)
        .await?
        .ok_or(Error::NotFound)
}

/// `POST /api/entries/{pid}/variants/{locale}/transition`.
#[debug_handler]
#[allow(clippy::too_many_lines)] // one transition, applied end to end
async fn transition(
    State(ctx): State<AppContext>,
    Path((pid, locale)): Path<(String, String)>,
    caller: MaybeAuthUser,
    Json(payload): Json<TransitionPayload>,
) -> Result<Response> {
    let entry = records::find_entry(&ctx.db, records::parse_pid(&pid)?).await?;
    let site = records::find_site(&ctx.db, entry.site_pid).await?;
    let Some(action) = Action::parse(&payload.action) else {
        return Err(unprocessable(&format!(
            "unknown action {:?}; expected one of {:?}",
            payload.action,
            lifecycle::ACTIONS
        )));
    };
    let mut problems = Problems::new();
    problems.ref_opt(
        "reviewer_ref",
        entity_ref::EntityType::Worker,
        payload.reviewer_ref.as_deref(),
    );
    problems.cap_opt("reason", payload.reason.as_deref());
    ensure_valid(&problems.into_vec())?;
    if action.requires_reason() && payload.reason.as_ref().is_none_or(|r| r.trim().is_empty()) {
        return Err(unprocessable(&format!(
            "{} requires a reason",
            action.as_str()
        )));
    }

    let txn = ctx.db.begin().await?;
    let variant = locked_variant(&txn, entry.pid, &locale).await?;
    let from = variant.status.clone();
    let to = match lifecycle::next(&from, action) {
        Ok(to) => to,
        Err(message) => {
            txn.rollback().await?;
            return Err(unprocessable(&message));
        }
    };

    // Separation of duties: the person who wrote it is not the person
    // who signs it off. Enforced in the machine rather than trusted to
    // habit, and only where the site asks for it.
    if action == Action::Approve && site.require_distinct_approver {
        let author = current_revision_author(&txn, &variant).await?;
        if let (Some(author), Some(actor)) = (author.as_deref(), caller.actor())
            && author == actor
        {
            txn.rollback().await?;
            return Err(unprocessable(
                "this site requires a distinct approver: the author of the current revision \
                 cannot approve it",
            ));
        }
    }

    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    let variant_row = variant.clone();
    let mut published_revision = variant.published_revision_pid;
    let mut first_published_at = variant.first_published_at;
    let mut published_at = variant.published_at;
    let mut event_data = serde_json::json!({ "from": from, "to": to });

    if action == Action::Publish {
        let revision_pid = payload
            .revision_pid
            .or(variant.current_revision_pid)
            .ok_or_else(|| unprocessable("this variant has no revision to publish"))?;
        let revision = records::find_revision(&txn, revision_pid).await?;
        if revision.variant_pid != variant.pid {
            txn.rollback().await?;
            return Err(unprocessable(
                "that revision belongs to a different variant",
            ));
        }
        // The gate: the same function the publish-check read uses.
        let blockers = super::entries::publish_blockers_for(&txn, &entry, &revision).await?;
        if !blockers.is_empty() {
            txn.rollback().await?;
            let summary = blockers
                .iter()
                .map(|b| format!("{} ({}): {}", b.rule, b.subject, b.remedy))
                .collect::<Vec<_>>()
                .join("; ");
            return Err(unprocessable(&format!("not ready to publish — {summary}")));
        }
        published_revision = Some(revision.pid);
        published_at = Some(now);
        // Preserved across unpublish and republish: "when did this first
        // appear" is a different question from "what is live now".
        first_published_at = first_published_at.or(Some(now));
        // Coming back to the address it always had: clear any marker a
        // previous unpublish left there.
        clear_marker(&txn, &variant_row).await?;
        event_data = serde_json::json!({
            "from": from,
            "to": to,
            "revision_pid": revision.pid,
            "revision_number": revision.number,
        });
    } else if action == Action::Unpublish {
        published_revision = None;
        published_at = None;
        let marker =
            leave_marker(&txn, &entry, &variant_row, payload.replacement_entry_pid).await?;
        event_data = serde_json::json!({
            "from": from,
            "to": to,
            "replacement_entry_pid": payload.replacement_entry_pid,
            "left_behind": marker,
        });
    }

    let variant_pid = variant.pid;
    let mut active: entry_variants::ActiveModel = variant.into();
    active.status = ActiveValue::set(to.to_string());
    active.published_revision_pid = ActiveValue::set(published_revision);
    active.published_at = ActiveValue::set(published_at);
    active.first_published_at = ActiveValue::set(first_published_at);
    if action == Action::Submit {
        active.reviewer_ref = ActiveValue::set(payload.reviewer_ref.clone());
    }
    if matches!(action, Action::Publish | Action::Unpublish) {
        // A schedule that has been overtaken by a manual action is
        // cleared, so the sweep cannot re-apply it later and surprise
        // whoever acted.
        active.scheduled_publish_at = ActiveValue::set(None);
        active.scheduled_unpublish_at = ActiveValue::set(None);
    }
    let updated = active.update(&txn).await?;

    Audit::record(
        &txn,
        "variant",
        variant_pid,
        action.as_str(),
        caller.actor(),
        Some(serde_json::json!({
            "entry": entry.key,
            "locale": locale,
            "from": from,
            "to": to,
            "reason": payload.reason,
            "reviewer": payload.reviewer_ref,
            "published_revision_pid": published_revision,
            "replacement_entry_pid": payload.replacement_entry_pid,
            "owner": entry.owner_ref,
        })),
    )
    .await?;
    let kind = match action {
        Action::Publish => "variant_published",
        Action::Unpublish => "variant_unpublished",
        Action::Submit => "variant_submitted",
        Action::Approve => "variant_approved",
        Action::Reject => "variant_rejected",
        Action::Archive => "variant_archived",
        Action::Restore => "variant_restored",
    };
    streaming::emit_on(
        &txn,
        "variant",
        kind,
        &variant_pid.to_string(),
        &entry.key,
        caller.actor(),
        Some(event_data),
    )
    .await?;
    txn.commit().await?;

    let metrics = crate::metrics::Metrics::global();
    match action {
        Action::Publish => metrics.variant_published_total.inc(),
        Action::Unpublish => metrics.variant_unpublished_total.inc(),
        _ => {}
    }

    format::json(TransitionView {
        variant_pid: variant_pid.to_string(),
        from,
        to: to.to_string(),
        published_revision_pid: updated.published_revision_pid.map(|p| p.to_string()),
        first_published_at: updated.first_published_at,
    })
}

/// The current route of a variant, if it has one.
async fn current_route(
    txn: &sea_orm::DatabaseTransaction,
    variant: &entry_variants::Model,
) -> Result<Option<routes::Model>> {
    let row = routes::Entity::find()
        .filter(routes::Column::VariantPid.eq(variant.pid))
        .filter(routes::Column::IsCurrent.eq(true))
        .one(txn)
        .await?;
    Ok(row)
}

/// Leave a redirect or a `410` marker at the address an unpublished
/// page is vacating.
///
/// A declared replacement becomes a `301`; with none, a `410` says the
/// page is gone. Either beats a bare `404`, which tells a reader
/// nothing and tells a crawler to keep asking.
async fn leave_marker(
    txn: &sea_orm::DatabaseTransaction,
    entry: &entries::Model,
    variant: &entry_variants::Model,
    replacement: Option<Uuid>,
) -> Result<Option<serde_json::Value>> {
    let Some(route) = current_route(txn, variant).await? else {
        return Ok(None);
    };
    let replacement_path = match replacement {
        Some(replacement_pid) => {
            let siblings = entry_variants::Entity::find()
                .filter(entry_variants::Column::EntryPid.eq(replacement_pid))
                .filter(entry_variants::Column::Locale.eq(variant.locale.clone()))
                .filter(entry_variants::Column::DeletedAt.is_null())
                .one(txn)
                .await?;
            match siblings {
                Some(sibling) => routes::Entity::find()
                    .filter(routes::Column::VariantPid.eq(sibling.pid))
                    .filter(routes::Column::IsCurrent.eq(true))
                    .one(txn)
                    .await?
                    .map(|row| row.path),
                None => None,
            }
        }
        None => None,
    };
    redirects::Entity::delete_many()
        .filter(redirects::Column::SitePid.eq(entry.site_pid))
        .filter(redirects::Column::Locale.eq(variant.locale.clone()))
        .filter(redirects::Column::FromPath.eq(route.path.clone()))
        .exec(txn)
        .await?;
    let status = if replacement_path.is_some() { 301 } else { 410 };
    redirects::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        site_pid: ActiveValue::set(entry.site_pid),
        locale: ActiveValue::set(variant.locale.clone()),
        from_path: ActiveValue::set(route.path.clone()),
        to_path: ActiveValue::set(replacement_path.clone()),
        status: ActiveValue::set(status),
        reason: ActiveValue::set("unpublish".to_string()),
        ..Default::default()
    }
    .insert(txn)
    .await?;
    Ok(Some(serde_json::json!({
        "path": route.path,
        "status": status,
        "to": replacement_path,
    })))
}

/// Remove the marker left by a previous unpublish, so a republished
/// page answers at its own address again.
async fn clear_marker(
    txn: &sea_orm::DatabaseTransaction,
    variant: &entry_variants::Model,
) -> Result<()> {
    let Some(route) = current_route(txn, variant).await? else {
        return Ok(());
    };
    redirects::Entity::delete_many()
        .filter(redirects::Column::Locale.eq(variant.locale.clone()))
        .filter(redirects::Column::FromPath.eq(route.path))
        .filter(redirects::Column::Reason.eq("unpublish"))
        .exec(txn)
        .await?;
    Ok(())
}

/// The author of a variant's current revision, if there is one.
async fn current_revision_author(
    txn: &sea_orm::DatabaseTransaction,
    variant: &entry_variants::Model,
) -> Result<Option<String>> {
    let Some(pid) = variant.current_revision_pid else {
        return Ok(None);
    };
    let revision = revisions::Entity::find()
        .filter(revisions::Column::Pid.eq(pid))
        .one(txn)
        .await?;
    Ok(revision.and_then(|r| r.author_ref))
}

/// `POST /api/entries/{pid}/variants/{locale}/schedule`.
///
/// A schedule is only accepted where the transition it will perform
/// would be legal *now* — scheduling a publish for a draft that nobody
/// has approved would silently bypass review at 3am.
#[debug_handler]
async fn schedule(
    State(ctx): State<AppContext>,
    Path((pid, locale)): Path<(String, String)>,
    caller: MaybeAuthUser,
    Json(payload): Json<SchedulePayload>,
) -> Result<Response> {
    let entry = records::find_entry(&ctx.db, records::parse_pid(&pid)?).await?;
    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    let txn = ctx.db.begin().await?;
    let variant = locked_variant(&txn, entry.pid, &locale).await?;

    let mut problems = Vec::new();
    if let Some(at) = payload.publish_at {
        if at <= now {
            problems.push("publish_at must be in the future".to_string());
        }
        if lifecycle::next(&variant.status, Action::Publish).is_err() {
            problems.push(format!(
                "cannot schedule a publish from {:?}: approve it first",
                variant.status
            ));
        }
    }
    if let Some(at) = payload.unpublish_at {
        if at <= now {
            problems.push("unpublish_at must be in the future".to_string());
        }
        if let (Some(publish_at), Some(unpublish_at)) = (payload.publish_at, payload.unpublish_at)
            && unpublish_at <= publish_at
        {
            problems.push("unpublish_at must be after publish_at".to_string());
        }
        if variant.status != "published" && payload.publish_at.is_none() {
            problems.push(format!(
                "cannot schedule an unpublish from {:?}: it is not live",
                variant.status
            ));
        }
    }
    if !problems.is_empty() {
        txn.rollback().await?;
        return Err(super::validation_error(&problems));
    }

    let variant_pid = variant.pid;
    let mut active: entry_variants::ActiveModel = variant.into();
    active.scheduled_publish_at = ActiveValue::set(payload.publish_at);
    active.scheduled_unpublish_at = ActiveValue::set(payload.unpublish_at);
    let updated = active.update(&txn).await?;
    Audit::record(
        &txn,
        "variant",
        variant_pid,
        "scheduled",
        caller.actor(),
        Some(serde_json::json!({
            "entry": entry.key,
            "locale": locale,
            "publish_at": payload.publish_at,
            "unpublish_at": payload.unpublish_at,
        })),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "variant",
        "variant_scheduled",
        &variant_pid.to_string(),
        &entry.key,
        caller.actor(),
        Some(serde_json::json!({
            "publish_at": payload.publish_at,
            "unpublish_at": payload.unpublish_at,
        })),
    )
    .await?;
    txn.commit().await?;
    format::json(updated)
}

/// `POST /api/entries/{pid}/variants/{locale}/lock` — take or extend
/// the advisory lock.
///
/// Advisory, and the docs say so: the authoritative protection against
/// losing work is the `base_revision_pid` check on every save
/// (CMS-R3). A lock reduces collisions; it does not prevent them, and
/// claiming otherwise would be the more dangerous kind of wrong.
#[debug_handler]
async fn lock(
    State(ctx): State<AppContext>,
    Path((pid, locale)): Path<(String, String)>,
    caller: MaybeAuthUser,
    Json(payload): Json<LockPayload>,
) -> Result<Response> {
    let entry = records::find_entry(&ctx.db, records::parse_pid(&pid)?).await?;
    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    let ttl = payload
        .ttl_secs
        .unwrap_or(DEFAULT_LOCK_SECS)
        .clamp(60, 86_400);
    let txn = ctx.db.begin().await?;
    let variant = locked_variant(&txn, entry.pid, &locale).await?;

    let held_by = variant.locked_by_ref.clone();
    let still_held = variant.locked_until.is_some_and(|until| until > now);
    let holder_is_caller = held_by.as_deref() == caller.actor() && caller.actor().is_some();
    let stealing = still_held && !holder_is_caller && held_by.is_some();
    if stealing && payload.reason.as_ref().is_none_or(|r| r.trim().is_empty()) {
        txn.rollback().await?;
        return Err(super::conflict(&format!(
            "this variant is locked by {} until {}; take it anyway by sending a reason",
            held_by.as_deref().unwrap_or("someone"),
            variant
                .locked_until
                .map_or_else(|| "?".to_string(), |t| t.to_rfc3339())
        )));
    }

    let variant_pid = variant.pid;
    let mut active: entry_variants::ActiveModel = variant.into();
    active.locked_by_ref = ActiveValue::set(caller.actor().map(ToString::to_string));
    active.locked_until = ActiveValue::set(Some(now + chrono::Duration::seconds(ttl)));
    let updated = active.update(&txn).await?;
    Audit::record(
        &txn,
        "variant",
        variant_pid,
        if stealing { "lock_stolen" } else { "locked" },
        caller.actor(),
        Some(serde_json::json!({
            "entry": entry.key,
            "locale": locale,
            "previous_holder": held_by,
            "reason": payload.reason,
            "ttl_secs": ttl,
        })),
    )
    .await?;
    txn.commit().await?;
    format::json(serde_json::json!({
        "locked_by_ref": updated.locked_by_ref,
        "locked_until": updated.locked_until,
        "stolen": stealing,
        "advisory": "a lock reduces collisions; the authoritative check is base_revision_pid on save",
    }))
}

/// `DELETE /api/entries/{pid}/variants/{locale}/lock` — release it.
#[debug_handler]
async fn unlock(
    State(ctx): State<AppContext>,
    Path((pid, locale)): Path<(String, String)>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let entry = records::find_entry(&ctx.db, records::parse_pid(&pid)?).await?;
    let txn = ctx.db.begin().await?;
    let variant = locked_variant(&txn, entry.pid, &locale).await?;
    let variant_pid = variant.pid;
    let mut active: entry_variants::ActiveModel = variant.into();
    active.locked_by_ref = ActiveValue::set(None);
    active.locked_until = ActiveValue::set(None);
    active.update(&txn).await?;
    Audit::record(
        &txn,
        "variant",
        variant_pid,
        "unlocked",
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::empty_json()
}

/// One schedule the sweep acted on, or deliberately did not.
#[derive(Debug, Serialize)]
pub struct SweepOutcome {
    /// The variant concerned.
    pub variant_pid: Uuid,
    /// `published`, `unpublished`, or `skipped`.
    pub outcome: &'static str,
    /// Why, when it was skipped or refused.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Apply every due schedule (CMS-R12).
///
/// Idempotent by construction: each variant is locked, its due schedule
/// field is cleared **in the same transaction** as the transition it
/// triggers, and the transition is re-checked against the variant's
/// current state. A rerun therefore finds nothing due; two overlapping
/// sweeps serialize on the row and the second sees the cleared field.
///
/// A schedule whose variant has since moved (someone published it by
/// hand, or archived it) is **skipped and recorded**, never forced —
/// the clock should not overrule a person who acted more recently.
///
/// # Errors
///
/// When a query or write fails.
pub async fn run_sweep(db: &DatabaseConnection, actor: Option<&str>) -> Result<Vec<SweepOutcome>> {
    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    let due = entry_variants::Entity::find()
        .filter(entry_variants::Column::DeletedAt.is_null())
        .filter(
            sea_orm::Condition::any()
                .add(entry_variants::Column::ScheduledPublishAt.lte(now))
                .add(entry_variants::Column::ScheduledUnpublishAt.lte(now)),
        )
        .order_by_asc(entry_variants::Column::Id)
        .limit(500)
        .all(db)
        .await?;

    let mut outcomes = Vec::new();
    for candidate in due {
        let txn = db.begin().await?;
        let Some(variant) = entry_variants::Entity::find()
            .filter(entry_variants::Column::Pid.eq(candidate.pid))
            .filter(entry_variants::Column::DeletedAt.is_null())
            .lock_exclusive()
            .one(&txn)
            .await?
        else {
            txn.rollback().await?;
            continue;
        };
        let publish_due = variant.scheduled_publish_at.is_some_and(|at| at <= now);
        let unpublish_due = variant.scheduled_unpublish_at.is_some_and(|at| at <= now);
        if !publish_due && !unpublish_due {
            // Another sweep got here first and cleared it.
            txn.rollback().await?;
            continue;
        }
        let action = if publish_due {
            Action::Publish
        } else {
            Action::Unpublish
        };
        let outcome = apply_scheduled(&txn, &variant, action, now, actor).await?;
        txn.commit().await?;
        outcomes.push(outcome);
    }
    Ok(outcomes)
}

/// Apply one due schedule inside its transaction.
#[allow(clippy::too_many_lines)] // one scheduled transition, end to end
async fn apply_scheduled(
    txn: &sea_orm::DatabaseTransaction,
    variant: &entry_variants::Model,
    action: Action,
    now: chrono::DateTime<chrono::FixedOffset>,
    actor: Option<&str>,
) -> Result<SweepOutcome> {
    // Every read here goes through `txn`, not the pool: this function
    // runs while the variant row is locked, and reaching for a second
    // connection would deadlock against the one it already holds.
    let entry = records::find_entry(txn, variant.entry_pid).await?;
    let skip = |detail: String| SweepOutcome {
        variant_pid: variant.pid,
        outcome: "skipped",
        detail: Some(detail),
    };

    let Ok(to) = lifecycle::next(&variant.status, action) else {
        clear_schedules(txn, variant).await?;
        let detail = format!(
            "{} is no longer legal from {:?}",
            action.as_str(),
            variant.status
        );
        record_skip(txn, variant, &entry, &detail, actor).await?;
        return Ok(skip(detail));
    };

    let mut first_published_at = variant.first_published_at;
    // Unpublishing clears both; publishing sets them from the revision
    // it makes live.
    let mut published_revision = None;
    let mut published_at = None;
    if action == Action::Publish {
        let Some(revision_pid) = variant.current_revision_pid else {
            clear_schedules(txn, variant).await?;
            let detail = "no revision to publish".to_string();
            record_skip(txn, variant, &entry, &detail, actor).await?;
            return Ok(skip(detail));
        };
        let revision = records::find_revision(txn, revision_pid).await?;
        // The scheduled path runs the same gate as the manual one: a
        // clock is not a reason to publish a page with no alt text.
        let blockers = super::entries::publish_blockers_for(txn, &entry, &revision).await?;
        if !blockers.is_empty() {
            clear_schedules(txn, variant).await?;
            let detail = format!(
                "blocked: {}",
                blockers
                    .iter()
                    .map(|b| b.rule.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            record_skip(txn, variant, &entry, &detail, actor).await?;
            return Ok(skip(detail));
        }
        published_revision = Some(revision.pid);
        published_at = Some(now);
        first_published_at = first_published_at.or(Some(now));
    }

    let mut active: entry_variants::ActiveModel = variant.clone().into();
    active.status = ActiveValue::set(to.to_string());
    active.published_revision_pid = ActiveValue::set(published_revision);
    active.published_at = ActiveValue::set(published_at);
    active.first_published_at = ActiveValue::set(first_published_at);
    // Clearing the due field inside this transaction is what makes the
    // sweep idempotent.
    if action == Action::Publish {
        active.scheduled_publish_at = ActiveValue::set(None);
    } else {
        active.scheduled_unpublish_at = ActiveValue::set(None);
    }
    active.update(txn).await?;

    Audit::record(
        txn,
        "variant",
        variant.pid,
        action.as_str(),
        // The trigger is recorded: "who published this at 3am" has an
        // answer, and the answer is the schedule.
        Some("system:schedule"),
        Some(serde_json::json!({
            "entry": entry.key,
            "locale": variant.locale,
            "from": variant.status,
            "to": to,
            "trigger": "schedule",
            "scheduled_by": actor,
            "published_revision_pid": published_revision,
        })),
    )
    .await?;
    // The ordinary event: a consumer cannot tell whether a human or the
    // clock did it, and need not care — the audit row records which.
    streaming::emit_on(
        txn,
        "variant",
        if action == Action::Publish {
            "variant_published"
        } else {
            "variant_unpublished"
        },
        &variant.pid.to_string(),
        &entry.key,
        Some("system:schedule"),
        Some(serde_json::json!({ "trigger": "schedule", "to": to })),
    )
    .await?;

    let metrics = crate::metrics::Metrics::global();
    if action == Action::Publish {
        metrics.variant_published_total.inc();
    } else {
        metrics.variant_unpublished_total.inc();
    }
    metrics.scheduled_execution_total.inc();

    Ok(SweepOutcome {
        variant_pid: variant.pid,
        outcome: if action == Action::Publish {
            "published"
        } else {
            "unpublished"
        },
        detail: None,
    })
}

/// Clear both schedule fields (used when a due schedule is skipped, so
/// it does not retry every sweep forever).
async fn clear_schedules(
    txn: &sea_orm::DatabaseTransaction,
    variant: &entry_variants::Model,
) -> Result<()> {
    let mut active: entry_variants::ActiveModel = variant.clone().into();
    active.scheduled_publish_at = ActiveValue::set(None);
    active.scheduled_unpublish_at = ActiveValue::set(None);
    active.update(txn).await?;
    Ok(())
}

/// Record a skipped schedule, so an operator can find out why their
/// page did not go live without reading logs.
async fn record_skip(
    txn: &sea_orm::DatabaseTransaction,
    variant: &entry_variants::Model,
    entry: &entries::Model,
    detail: &str,
    actor: Option<&str>,
) -> Result<()> {
    Audit::record(
        txn,
        "variant",
        variant.pid,
        "schedule_skipped",
        Some("system:schedule"),
        Some(serde_json::json!({
            "entry": entry.key,
            "locale": variant.locale,
            "status": variant.status,
            "detail": detail,
            "scheduled_by": actor,
        })),
    )
    .await?;
    Ok(())
}

/// `POST /api/schedules/sweep` — apply due schedules now.
///
/// The operational surface for the sweep: a system scheduler runs the
/// `schedule_sweep` CLI task, and this endpoint exists so an operator
/// (and the test suite) can drive the same function on demand. A
/// periodic in-process `bg_pg` worker is the documented seam
/// (`../../spec/tasks.md`).
#[debug_handler]
async fn sweep(State(ctx): State<AppContext>, caller: MaybeAuthUser) -> Result<Response> {
    let outcomes = run_sweep(&ctx.db, caller.actor()).await?;
    format::json(serde_json::json!({
        "as_of": chrono::Utc::now(),
        "applied": outcomes.iter().filter(|o| o.outcome != "skipped").count(),
        "skipped": outcomes.iter().filter(|o| o.outcome == "skipped").count(),
        "outcomes": outcomes,
    }))
}

/// `GET /api/sites/{pid}/schedules` — what is queued, so a schedule is
/// visible before it fires rather than only afterwards.
#[debug_handler]
async fn list_schedules(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
) -> Result<Response> {
    let site = records::find_site(&ctx.db, records::parse_pid(&pid)?).await?;
    let entry_rows = entries::Entity::find()
        .filter(entries::Column::SitePid.eq(site.pid))
        .filter(entries::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    let mut queued = Vec::new();
    for entry in entry_rows {
        let variants = entry_variants::Entity::find()
            .filter(entry_variants::Column::EntryPid.eq(entry.pid))
            .filter(entry_variants::Column::DeletedAt.is_null())
            .all(&ctx.db)
            .await?;
        for variant in variants {
            if variant.scheduled_publish_at.is_none() && variant.scheduled_unpublish_at.is_none() {
                continue;
            }
            queued.push(serde_json::json!({
                "entry_key": entry.key,
                "entry_pid": entry.pid,
                "locale": variant.locale,
                "status": variant.status,
                "publish_at": variant.scheduled_publish_at,
                "unpublish_at": variant.scheduled_unpublish_at,
            }));
        }
    }
    format::json(serde_json::json!({
        "as_of": chrono::Utc::now(),
        "queued": queued,
    }))
}

/// A site's live variants — the "what is published right now" read.
#[debug_handler]
async fn published(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let site = records::find_site(&ctx.db, records::parse_pid(&pid)?).await?;
    let entry_rows = entries::Entity::find()
        .filter(entries::Column::SitePid.eq(site.pid))
        .filter(entries::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    let mut live: Vec<Value> = Vec::new();
    for entry in entry_rows {
        let variants = entry_variants::Entity::find()
            .filter(entry_variants::Column::EntryPid.eq(entry.pid))
            .filter(entry_variants::Column::DeletedAt.is_null())
            .filter(entry_variants::Column::PublishedRevisionPid.is_not_null())
            .all(&ctx.db)
            .await?;
        for variant in variants {
            // Whether newer work is waiting behind the live revision is
            // the question an editor actually asks of this list.
            let has_newer_draft = variant.current_revision_pid != variant.published_revision_pid;
            live.push(serde_json::json!({
                "entry_key": entry.key,
                "entry_pid": entry.pid,
                "locale": variant.locale,
                "published_revision_pid": variant.published_revision_pid,
                "published_at": variant.published_at,
                "first_published_at": variant.first_published_at,
                "has_unpublished_changes": has_newer_draft,
            }));
        }
    }
    format::json(serde_json::json!({ "as_of": chrono::Utc::now(), "published": live }))
}

/// The workflow routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add(
            "/entries/{pid}/variants/{locale}/transition",
            post(transition),
        )
        .add("/entries/{pid}/variants/{locale}/schedule", post(schedule))
        .add(
            "/entries/{pid}/variants/{locale}/lock",
            post(lock).delete(unlock),
        )
        .add("/schedules/sweep", post(sweep))
        .add("/sites/{pid}/schedules", get(list_schedules))
        .add("/sites/{pid}/published", get(published))
}
