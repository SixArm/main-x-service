//! Outbound webhooks (CMS-R23, CMS-D12) — registration and dispatch.
//!
//! ## Dispatch reads the outbox, and says so when it cannot
//!
//! Deliveries are driven from the **event record**, so no extension can
//! observe a change the audit trail does not contain. That record is
//! durable only under `CMS_EVENT_TRANSPORT=outbox`; with the default
//! in-memory transport there is nothing reliable to dispatch from, and
//! this endpoint **says so** rather than silently delivering a subset
//! that vanishes on restart. Shipping a delivery path that quietly
//! drops events would be worse than not shipping one.
//!
//! ## The delivery client
//!
//! HTTPS only, **no redirects followed** (the family SSRF rule — a
//! redirect is an attacker-chosen host), a short timeout, a capped
//! response read, and a signed body carrying its own timestamp. Every
//! attempt is logged whether it succeeded or not.
//!
//! ## What is deliberately not here
//!
//! No in-process retry loop with sleeps. Retries are *scheduled* by
//! recording the attempt and letting the next dispatch pick the event
//! up again — the same posture as the publish sweep, and for the same
//! reason: a process that holds work in memory loses it on restart.

use axum::extract::State;
use loco_rs::prelude::*;
use sea_orm::{QueryOrder, QuerySelect, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ensure_valid, unprocessable};
use crate::auth::MaybeAuthUser;
use crate::models::_entities::{event_outbox, webhook_deliveries, webhooks};
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
use crate::rules::webhook;
use crate::validation::Problems;

/// Per-request timeout for a delivery.
const DELIVERY_TIMEOUT_SECS: u64 = 5;
/// How much of a receiver's response body is read before giving up on
/// it — enough to log a useful error, not enough to be a memory sink.
const MAX_RESPONSE_BYTES: usize = 4096;

/// `POST /api/sites/{pid}/webhooks` body.
#[derive(Debug, Deserialize)]
struct RegisterPayload {
    name: String,
    url: String,
    /// Kinds to receive; empty means all of them.
    #[serde(default)]
    event_kinds: Vec<String>,
}

/// The one response that carries the secret.
#[derive(Debug, Serialize)]
struct RegisteredView {
    pid: String,
    url: String,
    /// The signing secret — **shown once**, and never by any read.
    secret: String,
    signature_header: &'static str,
    timestamp_header: &'static str,
    note: &'static str,
}

/// `POST /api/sites/{pid}/webhooks` — register a subscription.
#[debug_handler]
async fn register(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    caller: MaybeAuthUser,
    Json(payload): Json<RegisterPayload>,
) -> Result<Response> {
    let site = records::find_site(&ctx.db, records::parse_pid(&pid)?).await?;
    let mut problems = Problems::new();
    problems.require_text("name", &payload.name);
    problems.cap_list("event_kinds", &payload.event_kinds);
    let mut problems = problems.into_vec();
    if let Err(refusal) = webhook::check_url(&payload.url) {
        problems.push(refusal.message().to_string());
    }
    ensure_valid(&problems)?;

    let secret = webhook::mint_secret();
    let txn = ctx.db.begin().await?;
    let row = webhooks::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        site_pid: ActiveValue::set(site.pid),
        name: ActiveValue::set(payload.name.clone()),
        url: ActiveValue::set(payload.url.trim().to_string()),
        event_kinds: ActiveValue::set(serde_json::json!(payload.event_kinds)),
        secret: ActiveValue::set(secret.clone()),
        active: ActiveValue::set(true),
        last_delivered_at: ActiveValue::set(None),
        consecutive_failures: ActiveValue::set(0),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    // The audit row records the subscription — never the secret
    // (security invariant 9).
    Audit::record(
        &txn,
        "webhook",
        row.pid,
        "created",
        caller.actor(),
        Some(serde_json::json!({
            "site": site.key,
            "name": row.name,
            "url": row.url,
            "event_kinds": payload.event_kinds,
        })),
    )
    .await?;
    txn.commit().await?;

    format::json(RegisteredView {
        pid: row.pid.to_string(),
        url: row.url,
        secret,
        signature_header: webhook::SIGNATURE_HEADER,
        timestamp_header: webhook::TIMESTAMP_HEADER,
        note: "verify HMAC-SHA256 over `{timestamp}.{body}` with this secret; it is shown once \
               and never returned by a read",
    })
}

/// `GET /api/sites/{pid}/webhooks` — the subscriptions, without their
/// secrets.
#[debug_handler]
async fn list(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let site = records::find_site(&ctx.db, records::parse_pid(&pid)?).await?;
    let rows = webhooks::Entity::find()
        .filter(webhooks::Column::SitePid.eq(site.pid))
        .filter(webhooks::Column::DeletedAt.is_null())
        .order_by_asc(webhooks::Column::Id)
        .all(&ctx.db)
        .await?;
    format::json(serde_json::json!({
        "webhooks": rows,
        "note": "secrets are never returned by a read; re-register to rotate one",
    }))
}

/// `GET /api/webhooks/{pid}/deliveries` — the attempt log.
#[debug_handler]
async fn deliveries(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let hook = records::find_webhook(&ctx.db, records::parse_pid(&pid)?).await?;
    let rows = webhook_deliveries::Entity::find()
        .filter(webhook_deliveries::Column::WebhookPid.eq(hook.pid))
        .order_by_desc(webhook_deliveries::Column::Id)
        .limit(200)
        .all(&ctx.db)
        .await?;
    format::json(serde_json::json!({
        "webhook": { "pid": hook.pid, "url": hook.url, "active": hook.active,
                     "consecutive_failures": hook.consecutive_failures },
        "deliveries": rows,
    }))
}

/// `DELETE /api/webhooks/{pid}` — withdraw a subscription.
#[debug_handler]
async fn remove(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let hook = records::find_webhook(&ctx.db, records::parse_pid(&pid)?).await?;
    let txn = ctx.db.begin().await?;
    let hook_pid = hook.pid;
    let mut active: webhooks::ActiveModel = hook.into();
    active.deleted_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    active.active = ActiveValue::set(false);
    active.update(&txn).await?;
    Audit::record(&txn, "webhook", hook_pid, "deleted", caller.actor(), None).await?;
    txn.commit().await?;
    format::empty_json()
}

/// One delivery outcome.
#[derive(Debug, Serialize)]
pub struct DeliveryOutcome {
    /// The subscription.
    pub webhook_pid: Uuid,
    /// The event delivered.
    pub event_id: Uuid,
    /// `delivered`, `failed`, or `abandoned`.
    pub state: &'static str,
    /// The HTTP status, when there was one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
}

/// Deliver one event to one subscription, recording the attempt.
async fn deliver(
    db: &DatabaseConnection,
    hook: &webhooks::Model,
    event: &event_outbox::Model,
    attempt: i32,
) -> Result<DeliveryOutcome> {
    let body = event.payload.to_string();
    let timestamp = chrono::Utc::now().timestamp();
    let signature = webhook::sign(&hook.secret, timestamp, &body);

    // No redirects: a redirect is an attacker-chosen host, and this
    // request carries a signature the receiver trusts.
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(DELIVERY_TIMEOUT_SECS))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| Error::Message(format!("webhook client: {e}")))?;
    let sent = client
        .post(&hook.url)
        .header(webhook::SIGNATURE_HEADER, &signature)
        .header(webhook::TIMESTAMP_HEADER, timestamp.to_string())
        .header(webhook::EVENT_ID_HEADER, event.event_id.to_string())
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .await;

    let (state, status, error) = match sent {
        Ok(response) => {
            let status = response.status().as_u16();
            // Read a bounded slice of the body: enough to log a useful
            // error, not enough to be a memory sink.
            let detail = response
                .text()
                .await
                .map(|text| text.chars().take(MAX_RESPONSE_BYTES).collect::<String>())
                .unwrap_or_default();
            if webhook::is_success(status) {
                ("delivered", Some(status), None)
            } else if webhook::is_retryable(status) && attempt < webhook::MAX_ATTEMPTS {
                ("failed", Some(status), Some(detail))
            } else {
                ("abandoned", Some(status), Some(detail))
            }
        }
        Err(error) => {
            let detail = error.to_string();
            if attempt < webhook::MAX_ATTEMPTS {
                ("failed", None, Some(detail))
            } else {
                ("abandoned", None, Some(detail))
            }
        }
    };

    let txn = db.begin().await?;
    webhook_deliveries::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        webhook_pid: ActiveValue::set(hook.pid),
        event_id: ActiveValue::set(event.event_id),
        event_kind: ActiveValue::set(event.kind.clone()),
        attempt: ActiveValue::set(attempt),
        state: ActiveValue::set(state.to_string()),
        status_code: ActiveValue::set(status.map(i32::from)),
        error: ActiveValue::set(
            error
                .clone()
                .map(|text| text.chars().take(500).collect::<String>()),
        ),
        delivered_at: ActiveValue::set((state == "delivered").then(|| chrono::Utc::now().into())),
        ..Default::default()
    }
    .insert(&txn)
    .await?;

    // Re-read inside the transaction: `hook` was loaded before this
    // pass began, and a dispatch that delivers several events to one
    // subscription would otherwise write the same stale count back each
    // time — the failure counter would advance once per *pass* rather
    // than once per failure, and the deactivation threshold would take
    // twenty passes to reach instead of twenty failures.
    let current = webhooks::Entity::find()
        .filter(webhooks::Column::Pid.eq(hook.pid))
        .one(&txn)
        .await?
        .ok_or(Error::NotFound)?;
    let failures = if state == "delivered" {
        0
    } else {
        current.consecutive_failures.saturating_add(1)
    };
    let mut active: webhooks::ActiveModel = current.into();
    active.consecutive_failures = ActiveValue::set(failures);
    if state == "delivered" {
        active.last_delivered_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    }
    // A receiver that has been broken for a long time is not helped by
    // more traffic.
    if failures >= webhook::FAILURE_DEACTIVATION_THRESHOLD {
        active.active = ActiveValue::set(false);
        tracing::warn!(
            webhook = %hook.pid, failures,
            "deactivating a webhook after repeated failures"
        );
    }
    active.update(&txn).await?;
    txn.commit().await?;

    Ok(DeliveryOutcome {
        webhook_pid: hook.pid,
        event_id: event.event_id,
        state,
        status,
    })
}

/// Deliver every outbox event that a live subscription wants and has
/// not already received.
///
/// # Errors
///
/// When a query or write fails. A failed *delivery* is recorded, not
/// returned as an error: one broken receiver must not stop the others.
pub async fn run_dispatch(db: &DatabaseConnection) -> Result<Vec<DeliveryOutcome>> {
    let hooks = webhooks::Entity::find()
        .filter(webhooks::Column::DeletedAt.is_null())
        .filter(webhooks::Column::Active.eq(true))
        .all(db)
        .await?;
    if hooks.is_empty() {
        return Ok(Vec::new());
    }
    let events = event_outbox::Entity::find()
        .order_by_desc(event_outbox::Column::Id)
        .limit(200)
        .all(db)
        .await?;

    let mut outcomes = Vec::new();
    for hook in &hooks {
        let subscribed: Vec<String> =
            serde_json::from_value(hook.event_kinds.clone()).unwrap_or_default();
        for event in &events {
            if !webhook::wants(&subscribed, &event.kind) {
                continue;
            }
            // Attempts already made for this (webhook, event).
            let prior = webhook_deliveries::Entity::find()
                .filter(webhook_deliveries::Column::WebhookPid.eq(hook.pid))
                .filter(webhook_deliveries::Column::EventId.eq(event.event_id))
                .order_by_desc(webhook_deliveries::Column::Attempt)
                .all(db)
                .await?;
            // Delivered or abandoned: nothing more to do. This is what
            // makes a rerun safe.
            if prior
                .iter()
                .any(|row| row.state == "delivered" || row.state == "abandoned")
            {
                continue;
            }
            let attempt = prior.iter().map(|row| row.attempt).max().unwrap_or(0) + 1;
            // Respect the backoff: a failed attempt waits before the
            // next one rather than being retried on every sweep.
            if let Some(last) = prior.iter().max_by_key(|row| row.attempt) {
                let wait = webhook::backoff_secs(attempt);
                let ready_at = last.created_at + chrono::Duration::seconds(wait);
                if ready_at > chrono::Utc::now() {
                    continue;
                }
            }
            outcomes.push(deliver(db, hook, event, attempt).await?);
        }
    }
    Ok(outcomes)
}

/// `POST /api/webhooks/dispatch` — deliver what is due.
#[debug_handler]
async fn dispatch(State(ctx): State<AppContext>) -> Result<Response> {
    // Honest about the transport: with the in-memory event bus there is
    // no durable record to dispatch from, and pretending otherwise
    // would deliver a subset that disappears on restart.
    if !crate::streaming::transport().is_outbox() {
        return Err(unprocessable(
            "webhook dispatch reads the durable event outbox; set CMS_EVENT_TRANSPORT=outbox. \
             With the in-memory transport there is no durable record to deliver from, and \
             delivering a subset that vanishes on restart would be worse than delivering none",
        ));
    }
    let outcomes = run_dispatch(&ctx.db).await?;
    format::json(serde_json::json!({
        "as_of": chrono::Utc::now(),
        "delivered": outcomes.iter().filter(|o| o.state == "delivered").count(),
        "failed": outcomes.iter().filter(|o| o.state == "failed").count(),
        "abandoned": outcomes.iter().filter(|o| o.state == "abandoned").count(),
        "outcomes": outcomes,
    }))
}

/// The webhook routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/sites/{pid}/webhooks", post(register).get(list))
        .add("/webhooks/{pid}", delete(remove))
        .add("/webhooks/{pid}/deliveries", get(deliveries))
        .add("/webhooks/dispatch", post(dispatch))
}
