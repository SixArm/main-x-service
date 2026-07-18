//! Marketing automation (CRM-R6–R9): consent (send-path law),
//! segments with preview, campaigns with the simulated send, and
//! nurture sequences with the idempotent advance sweep.
//!
//! The send paths re-check consent **at send time** (CRM-D6); the
//! advance sweep is idempotent per (enrolment, step) (CRM-D8) — in
//! v1 it runs via `POST /api/nurture/advance` (and the seed/demo
//! flow); a periodic `bg_pg` worker is the roadmap seam.

use loco_rs::prelude::*;
use sea_orm::{QueryOrder, QuerySelect, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ensure_valid, unprocessable};
use crate::auth::MaybeAuthUser;
use crate::metrics::Metrics;
use crate::models::_entities::{
    accounts, activities, campaigns, consent_events, contacts, deals, nurture_enrollments,
    nurture_sequences, nurture_steps, segments,
};
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
use crate::rules::{analytics, lifecycle, segment as segment_rules, tokens};
use crate::streaming;
use crate::validation::Problems;

/// `POST /api/contacts/{pid}/consent` body.
#[derive(Debug, Deserialize)]
struct ConsentPayload {
    action: String,
    source: String,
}

/// `POST /api/segments` body.
#[derive(Debug, Deserialize)]
struct SegmentPayload {
    name: String,
    #[serde(default)]
    filter: segment_rules::Filter,
}

/// `POST /api/campaigns` body.
#[derive(Debug, Deserialize)]
struct CampaignPayload {
    name: String,
    #[serde(default = "default_kind")]
    kind: String,
    #[serde(default)]
    cost_minor: i64,
    #[serde(default = "default_currency")]
    currency: String,
    #[serde(default)]
    segment_pid: Option<Uuid>,
}

/// `POST /api/campaigns/{pid}/status` body.
#[derive(Debug, Deserialize)]
struct CampaignStatusPayload {
    to: String,
}

/// `POST /api/nurture-sequences` body.
#[derive(Debug, Deserialize)]
struct SequencePayload {
    name: String,
    steps: Vec<StepPayload>,
}

/// One nurture step.
#[derive(Debug, Deserialize)]
struct StepPayload {
    delay_hours: i32,
    template_ref: String,
}

/// `POST /api/nurture-sequences/{pid}/enrollments` body.
#[derive(Debug, Deserialize)]
struct EnrollPayload {
    contact_pid: Uuid,
}

/// A `{pid}` reference response.
#[derive(Debug, Serialize)]
struct PidRef {
    pid: String,
}

fn default_kind() -> String {
    "email".to_string()
}
fn default_currency() -> String {
    "GBP".to_string()
}

/// The contact's segment facts (consent + status + channel + tier).
async fn contact_facts<C: sea_orm::ConnectionTrait>(
    db: &C,
    contact: &contacts::Model,
) -> Result<segment_rules::ContactFacts> {
    let tier = if let Some(account_pid) = contact.account_pid {
        accounts::Entity::find()
            .filter(accounts::Column::Pid.eq(account_pid))
            .filter(accounts::Column::DeletedAt.is_null())
            .one(db)
            .await?
            .map(|a| a.tier)
    } else {
        None
    };
    Ok(segment_rules::ContactFacts {
        consent: contact.marketing_consent.clone(),
        status: contact.status.clone(),
        channel: contact.preferred_channel.clone(),
        account_tier: tier,
    })
}

/// `POST /api/contacts/{pid}/consent` — record a consent change:
/// appends the `ConsentEvent`, flips the contact, and a withdrawal
/// exits every active nurture enrolment (CRM-R6, CRM-D6). Audited
/// (compliance evidence).
#[debug_handler]
async fn record_consent(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<ConsentPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("action", tokens::CONSENT_ACTIONS, &payload.action);
    problems.require_text("source", &payload.source);
    ensure_valid(&problems.into_vec())?;
    let contact = records::find_contact(&ctx.db, records::parse_pid(&pid)?).await?;
    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    let txn = ctx.db.begin().await?;
    consent_events::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        contact_pid: ActiveValue::set(contact.pid),
        action: ActiveValue::set(payload.action.clone()),
        source: ActiveValue::set(payload.source.clone()),
        occurred_at: ActiveValue::set(now),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    let contact_pid = contact.pid;
    let mut active: contacts::ActiveModel = contact.into();
    active.marketing_consent = ActiveValue::set(payload.action.clone());
    active.consent_changed_at = ActiveValue::set(Some(now));
    active.update(&txn).await?;
    if payload.action == "withdrawn" {
        // Exit every active enrolment immediately.
        let enrollments = nurture_enrollments::Entity::find()
            .filter(nurture_enrollments::Column::ContactPid.eq(contact_pid))
            .filter(nurture_enrollments::Column::Status.eq("active"))
            .filter(nurture_enrollments::Column::DeletedAt.is_null())
            .all(&txn)
            .await?;
        for enrollment in enrollments {
            let mut active: nurture_enrollments::ActiveModel = enrollment.into();
            active.status = ActiveValue::set("exited".to_string());
            active.next_due_at = ActiveValue::set(None);
            active.update(&txn).await?;
        }
    }
    let kind = format!("consent_{}", payload.action);
    Audit::record(
        &txn,
        "contact",
        contact_pid,
        &kind,
        caller.actor(),
        Some(serde_json::json!({ "source": payload.source })),
    )
    .await?;
    streaming::emit_on(&txn, "contact", &kind, &contact_pid.to_string(), "", caller.actor(), None).await?;
    txn.commit().await?;
    format::empty_json()
}

/// `GET /api/contacts/{pid}/consent` — the append-only history
/// (audited read: compliance evidence, CRM-D7).
#[debug_handler]
async fn consent_history(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let contact = records::find_contact(&ctx.db, records::parse_pid(&pid)?).await?;
    let rows = consent_events::Entity::find()
        .filter(consent_events::Column::ContactPid.eq(contact.pid))
        .order_by_asc(consent_events::Column::Id)
        .all(&ctx.db)
        .await?;
    Audit::record(&ctx.db, "contact", contact.pid, "consent_history_read", caller.actor(), None).await?;
    format::json(rows)
}

/// `POST /api/segments`.
#[debug_handler]
async fn create_segment(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<SegmentPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("name", &payload.name);
    for status in &payload.filter.statuses {
        problems.require_token("filter.statuses", tokens::CONTACT_STATUSES, status);
    }
    for channel in &payload.filter.channels {
        problems.require_token("filter.channels", tokens::CHANNELS, channel);
    }
    for tier in &payload.filter.account_tiers {
        problems.require_token("filter.account_tiers", tokens::ACCOUNT_TIERS, tier);
    }
    ensure_valid(&problems.into_vec())?;
    let txn = ctx.db.begin().await?;
    let row = segments::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        name: ActiveValue::set(payload.name.clone()),
        filter: ActiveValue::set(serde_json::to_value(&payload.filter).unwrap_or_default()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "segment", row.pid, "created", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef { pid: row.pid.to_string() })
}

/// The contacts a segment matches right now (consent-gated).
async fn evaluate_segment(
    db: &DatabaseConnection,
    segment: &segments::Model,
) -> Result<Vec<contacts::Model>> {
    let filter: segment_rules::Filter =
        serde_json::from_value(segment.filter.clone()).unwrap_or_default();
    let all = contacts::Entity::find()
        .filter(contacts::Column::DeletedAt.is_null())
        .all(db)
        .await?;
    let mut matched = Vec::new();
    for contact in all {
        let facts = contact_facts(db, &contact).await?;
        if segment_rules::matches(&filter, &facts) {
            matched.push(contact);
        }
    }
    Ok(matched)
}

/// `GET /api/segments/{pid}/preview` — count + sample before
/// scheduling (CRM-R7).
#[debug_handler]
async fn preview_segment(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let segment = records::find_segment(&ctx.db, records::parse_pid(&pid)?).await?;
    let matched = evaluate_segment(&ctx.db, &segment).await?;
    let sample: Vec<_> = matched.iter().take(5).map(|c| c.display_name.clone()).collect();
    format::json(serde_json::json!({ "count": matched.len(), "sample": sample }))
}

/// `GET /api/segments`.
#[debug_handler]
async fn list_segments(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = segments::Entity::find()
        .filter(segments::Column::DeletedAt.is_null())
        .order_by_asc(segments::Column::Id)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

/// `POST /api/campaigns`.
#[debug_handler]
async fn create_campaign(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<CampaignPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("name", &payload.name);
    problems.require_text("currency", &payload.currency);
    if payload.kind != "email" {
        problems.push("kind must be email (v1)".to_string());
    }
    if payload.cost_minor < 0 {
        problems.push("cost_minor must be non-negative".to_string());
    }
    ensure_valid(&problems.into_vec())?;
    if let Some(segment) = payload.segment_pid {
        records::find_segment(&ctx.db, segment).await?;
    }
    let txn = ctx.db.begin().await?;
    let row = campaigns::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        kind: ActiveValue::set(payload.kind.clone()),
        name: ActiveValue::set(payload.name.clone()),
        status: ActiveValue::set("draft".to_string()),
        cost_minor: ActiveValue::set(payload.cost_minor),
        currency: ActiveValue::set(payload.currency.clone()),
        segment_pid: ActiveValue::set(payload.segment_pid),
        recipients: ActiveValue::set(0),
        delivered: ActiveValue::set(0),
        opened: ActiveValue::set(0),
        clicked: ActiveValue::set(0),
        unsubscribed: ActiveValue::set(0),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "campaign", row.pid, "created", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef { pid: row.pid.to_string() })
}

/// `GET /api/campaigns`.
#[debug_handler]
async fn list_campaigns(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = campaigns::Entity::find()
        .filter(campaigns::Column::DeletedAt.is_null())
        .order_by_asc(campaigns::Column::Id)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

/// `POST /api/campaigns/{pid}/status` — one lifecycle transition.
#[debug_handler]
async fn campaign_status(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<CampaignStatusPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("to", tokens::CAMPAIGN_STATUSES, &payload.to);
    ensure_valid(&problems.into_vec())?;
    let campaign = records::find_campaign(&ctx.db, records::parse_pid(&pid)?).await?;
    lifecycle::check("campaign", lifecycle::CAMPAIGN, &campaign.status, &payload.to)
        .map_err(|e| unprocessable(&e))?;
    let txn = ctx.db.begin().await?;
    let from = campaign.status.clone();
    let name = campaign.name.clone();
    let mut active: campaigns::ActiveModel = campaign.into();
    active.status = ActiveValue::set(payload.to.clone());
    let row = active.update(&txn).await?;
    let kind = match payload.to.as_str() {
        "running" => "campaign_started",
        "completed" => "campaign_completed",
        _ => "campaign_status_changed",
    };
    Audit::record(
        &txn,
        "campaign",
        row.pid,
        kind,
        caller.actor(),
        Some(serde_json::json!({ "from": from })),
    )
    .await?;
    streaming::emit_on(&txn, "campaign", kind, &row.pid.to_string(), &name, caller.actor(), None).await?;
    txn.commit().await?;
    format::json(row)
}

/// `POST /api/campaigns/{pid}/run` — the **simulated send** (demo
/// mode; a real ESP adapter is roadmap, CRM-D8): enumerates the
/// segment **re-checking consent at send time**, writes per-contact
/// touch activities, advances the engagement counters
/// deterministically, and completes the campaign.
#[debug_handler]
async fn run_campaign(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let campaign = records::find_campaign(&ctx.db, records::parse_pid(&pid)?).await?;
    if campaign.status != "scheduled" && campaign.status != "running" {
        return Err(unprocessable(&format!(
            "campaign is {} — schedule it first",
            campaign.status
        )));
    }
    let segment_pid = campaign
        .segment_pid
        .ok_or_else(|| unprocessable("campaign has no segment"))?;
    let segment = records::find_segment(&ctx.db, segment_pid).await?;
    // Consent is structural in the evaluator (CRM-D6) — this IS the
    // send-time re-check.
    let audience = evaluate_segment(&ctx.db, &segment).await?;
    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    let txn = ctx.db.begin().await?;
    for contact in &audience {
        activities::ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            subject_kind: ActiveValue::set("contact".to_string()),
            subject_pid: ActiveValue::set(contact.pid),
            kind: ActiveValue::set("email".to_string()),
            occurred_at: ActiveValue::set(now),
            actor_ref: ActiveValue::set(None),
            summary: ActiveValue::set(format!("Campaign touch: {}", campaign.name)),
            due_on: ActiveValue::set(None),
            done: ActiveValue::set(false),
            deleted_at: ActiveValue::set(None),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
    }
    let recipients = i32::try_from(audience.len()).unwrap_or(i32::MAX);
    let campaign_pid = campaign.pid;
    let name = campaign.name.clone();
    let mut active: campaigns::ActiveModel = campaign.into();
    active.status = ActiveValue::set("completed".to_string());
    active.recipients = ActiveValue::set(recipients);
    // Deterministic demo engagement: all delivered, 3/5 opened, 1/4
    // clicked, none unsubscribed by the simulation.
    active.delivered = ActiveValue::set(recipients);
    active.opened = ActiveValue::set(recipients * 3 / 5);
    active.clicked = ActiveValue::set(recipients / 4);
    let row = active.update(&txn).await?;
    Audit::record(
        &txn,
        "campaign",
        campaign_pid,
        "campaign_completed",
        caller.actor(),
        Some(serde_json::json!({ "recipients": recipients })),
    )
    .await?;
    streaming::emit_on(&txn, "campaign", "campaign_completed", &campaign_pid.to_string(), &name, caller.actor(), None).await?;
    txn.commit().await?;
    format::json(row)
}

/// `GET /api/campaigns/{pid}/funnel` — the funnel + ROI (CRM-R8):
/// recipients → delivered → opened → clicked → leads → won revenue;
/// ROI reports `null` on a zero cost, with absolutes alongside.
#[debug_handler]
async fn campaign_funnel(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let campaign = records::find_campaign(&ctx.db, records::parse_pid(&pid)?).await?;
    let lead_rows = crate::models::_entities::leads::Entity::find()
        .filter(crate::models::_entities::leads::Column::CampaignPid.eq(campaign.pid))
        .filter(crate::models::_entities::leads::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    let attributed_deals = deals::Entity::find()
        .filter(deals::Column::SourceCampaignPid.eq(campaign.pid))
        .filter(deals::Column::DeletedAt.is_null())
        .filter(deals::Column::Won.eq(true))
        .all(&ctx.db)
        .await?;
    // Won revenue in the campaign's currency only (per-currency
    // honesty: other currencies are listed, never summed in).
    let mut same_currency_revenue: i64 = 0;
    let mut other_currency: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
    for deal in &attributed_deals {
        if deal.currency.eq_ignore_ascii_case(&campaign.currency) {
            same_currency_revenue = same_currency_revenue.saturating_add(deal.amount_minor);
        } else {
            *other_currency.entry(deal.currency.clone()).or_insert(0) += deal.amount_minor;
        }
    }
    let roi = analytics::roi(same_currency_revenue, campaign.cost_minor);
    format::json(serde_json::json!({
        "campaign": campaign,
        "leads": lead_rows.len(),
        "won_deals": attributed_deals.len(),
        "won_revenue_minor": same_currency_revenue,
        "won_revenue_other_currencies": other_currency,
        "roi": roi,
    }))
}

/// `POST /api/nurture-sequences` — create with ordered steps.
#[debug_handler]
async fn create_sequence(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<SequencePayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("name", &payload.name);
    if payload.steps.is_empty() || payload.steps.len() > 32 {
        problems.push("a sequence needs 1-32 steps".to_string());
    }
    for step in &payload.steps {
        problems.require_text("steps[].template_ref", &step.template_ref);
        if step.delay_hours < 0 {
            problems.push("delay_hours must be non-negative".to_string());
        }
    }
    ensure_valid(&problems.into_vec())?;
    let txn = ctx.db.begin().await?;
    let sequence = nurture_sequences::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        name: ActiveValue::set(payload.name.clone()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    for (position, step) in payload.steps.iter().enumerate() {
        nurture_steps::ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            sequence_pid: ActiveValue::set(sequence.pid),
            position: ActiveValue::set(i32::try_from(position).unwrap_or(i32::MAX)),
            delay_hours: ActiveValue::set(step.delay_hours),
            template_ref: ActiveValue::set(step.template_ref.clone()),
            deleted_at: ActiveValue::set(None),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
    }
    Audit::record(&txn, "nurture_sequence", sequence.pid, "created", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef { pid: sequence.pid.to_string() })
}

/// `POST /api/nurture-sequences/{pid}/enrollments` — enrol a
/// consented contact; the first step is due immediately + its delay.
#[debug_handler]
async fn enroll(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<EnrollPayload>,
) -> Result<Response> {
    let sequence = records::find_sequence(&ctx.db, records::parse_pid(&pid)?).await?;
    let contact = records::find_contact(&ctx.db, payload.contact_pid).await?;
    if contact.marketing_consent != "granted" {
        return Err(unprocessable("contact has not granted marketing consent"));
    }
    let first_step = nurture_steps::Entity::find()
        .filter(nurture_steps::Column::SequencePid.eq(sequence.pid))
        .filter(nurture_steps::Column::DeletedAt.is_null())
        .order_by_asc(nurture_steps::Column::Position)
        .one(&ctx.db)
        .await?
        .ok_or_else(|| unprocessable("sequence has no steps"))?;
    let due = chrono::Utc::now() + chrono::Duration::hours(i64::from(first_step.delay_hours));
    let txn = ctx.db.begin().await?;
    let row = nurture_enrollments::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        sequence_pid: ActiveValue::set(sequence.pid),
        contact_pid: ActiveValue::set(contact.pid),
        current_step: ActiveValue::set(0),
        next_due_at: ActiveValue::set(Some(due.into())),
        status: ActiveValue::set("active".to_string()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "nurture_enrollment", row.pid, "created", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef { pid: row.pid.to_string() })
}

/// `POST /api/nurture/advance` — the **idempotent advance sweep**
/// (CRM-R9, CRM-D8): for each active enrolment past its `next_due_at`,
/// simulate the step send (touch activity + `nurture_step_sent`),
/// advance to the next step or complete. Consent is re-checked at
/// send time; each (enrolment, step) sends at most once because the
/// row's `current_step` advances in the same transaction.
#[debug_handler]
async fn advance_nurture(State(ctx): State<AppContext>, caller: MaybeAuthUser) -> Result<Response> {
    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    let due = nurture_enrollments::Entity::find()
        .filter(nurture_enrollments::Column::Status.eq("active"))
        .filter(nurture_enrollments::Column::DeletedAt.is_null())
        .filter(nurture_enrollments::Column::NextDueAt.lte(now))
        .limit(500)
        .all(&ctx.db)
        .await?;
    let mut sent = 0_u64;
    let mut completed = 0_u64;
    for enrollment in due {
        let txn = ctx.db.begin().await?;
        // Re-load + lock: the sweep is safe to run concurrently.
        let Some(row) = nurture_enrollments::Entity::find()
            .filter(nurture_enrollments::Column::Pid.eq(enrollment.pid))
            .filter(nurture_enrollments::Column::Status.eq("active"))
            .lock_exclusive()
            .one(&txn)
            .await?
        else {
            txn.commit().await?;
            continue;
        };
        if row.next_due_at.is_none_or(|d| d > now) {
            txn.commit().await?;
            continue; // another sweep advanced it already
        }
        // Send-time consent re-check (CRM-D6).
        let contact = records::find_contact(&txn, row.contact_pid).await?;
        if contact.marketing_consent != "granted" {
            let mut active: nurture_enrollments::ActiveModel = row.into();
            active.status = ActiveValue::set("exited".to_string());
            active.next_due_at = ActiveValue::set(None);
            active.update(&txn).await?;
            txn.commit().await?;
            continue;
        }
        let steps = nurture_steps::Entity::find()
            .filter(nurture_steps::Column::SequencePid.eq(row.sequence_pid))
            .filter(nurture_steps::Column::DeletedAt.is_null())
            .order_by_asc(nurture_steps::Column::Position)
            .all(&txn)
            .await?;
        let Some(step) = steps.get(usize::try_from(row.current_step).unwrap_or(usize::MAX)) else {
            let mut active: nurture_enrollments::ActiveModel = row.into();
            active.status = ActiveValue::set("completed".to_string());
            active.next_due_at = ActiveValue::set(None);
            active.update(&txn).await?;
            txn.commit().await?;
            completed += 1;
            continue;
        };
        // Simulated step send: the touch activity + event.
        activities::ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            subject_kind: ActiveValue::set("contact".to_string()),
            subject_pid: ActiveValue::set(contact.pid),
            kind: ActiveValue::set("email".to_string()),
            occurred_at: ActiveValue::set(now),
            actor_ref: ActiveValue::set(None),
            summary: ActiveValue::set(format!("Nurture step {} ({})", step.position, step.template_ref)),
            due_on: ActiveValue::set(None),
            done: ActiveValue::set(false),
            deleted_at: ActiveValue::set(None),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
        streaming::emit_on(
            &txn,
            "nurture_enrollment",
            "nurture_step_sent",
            &row.pid.to_string(),
            &step.template_ref,
            caller.actor(),
            Some(serde_json::json!({ "step": step.position })),
        )
        .await?;
        let next_index = row.current_step + 1;
        let mut active: nurture_enrollments::ActiveModel = row.into();
        if let Some(next_step) = steps.get(usize::try_from(next_index).unwrap_or(usize::MAX)) {
            active.current_step = ActiveValue::set(next_index);
            let next_due = now + chrono::Duration::hours(i64::from(next_step.delay_hours));
            active.next_due_at = ActiveValue::set(Some(next_due));
        } else {
            active.current_step = ActiveValue::set(next_index);
            active.status = ActiveValue::set("completed".to_string());
            active.next_due_at = ActiveValue::set(None);
            completed += 1;
        }
        active.update(&txn).await?;
        txn.commit().await?;
        sent += 1;
        Metrics::global().nurture_step_sent_total.inc();
    }
    format::json(serde_json::json!({ "sent": sent, "completed": completed }))
}

/// `GET /api/nurture-sequences` — sequences with steps + live
/// enrolment counts.
#[debug_handler]
async fn list_sequences(State(ctx): State<AppContext>) -> Result<Response> {
    use sea_orm::PaginatorTrait;
    let rows = nurture_sequences::Entity::find()
        .filter(nurture_sequences::Column::DeletedAt.is_null())
        .order_by_asc(nurture_sequences::Column::Id)
        .all(&ctx.db)
        .await?;
    let mut out = Vec::with_capacity(rows.len());
    for sequence in rows {
        let steps = nurture_steps::Entity::find()
            .filter(nurture_steps::Column::SequencePid.eq(sequence.pid))
            .filter(nurture_steps::Column::DeletedAt.is_null())
            .order_by_asc(nurture_steps::Column::Position)
            .all(&ctx.db)
            .await?;
        let active = nurture_enrollments::Entity::find()
            .filter(nurture_enrollments::Column::SequencePid.eq(sequence.pid))
            .filter(nurture_enrollments::Column::Status.eq("active"))
            .filter(nurture_enrollments::Column::DeletedAt.is_null())
            .count(&ctx.db)
            .await?;
        out.push(serde_json::json!({
            "sequence": sequence, "steps": steps, "active_enrollments": active,
        }));
    }
    format::json(out)
}

/// The marketing routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/contacts/{pid}/consent", post(record_consent))
        .add("/contacts/{pid}/consent", get(consent_history))
        .add("/segments", post(create_segment))
        .add("/segments", get(list_segments))
        .add("/segments/{pid}/preview", get(preview_segment))
        .add("/campaigns", post(create_campaign))
        .add("/campaigns", get(list_campaigns))
        .add("/campaigns/{pid}/status", post(campaign_status))
        .add("/campaigns/{pid}/run", post(run_campaign))
        .add("/campaigns/{pid}/funnel", get(campaign_funnel))
        .add("/nurture-sequences", post(create_sequence))
        .add("/nurture-sequences", get(list_sequences))
        .add("/nurture-sequences/{pid}/enrollments", post(enroll))
        .add("/nurture/advance", post(advance_nurture))
}
