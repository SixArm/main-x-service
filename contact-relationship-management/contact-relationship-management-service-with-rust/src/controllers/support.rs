//! Customer service & support (CRM-R10–R12): tickets with derived
//! SLA deadlines and breach facts, SLA policies, the breach sweep,
//! and the versioned knowledge base.

use loco_rs::prelude::*;
use sea_orm::{QueryOrder, QuerySelect, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ensure_valid, unprocessable};
use crate::auth::MaybeAuthUser;
use crate::metrics::Metrics;
use crate::models::_entities::{activities, articles, sla_policies, tickets};
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
use crate::rules::{lifecycle, sla, tokens};
use crate::streaming;
use crate::validation::Problems;

/// `POST /api/sla-policies` body.
#[derive(Debug, Deserialize)]
struct PolicyPayload {
    priority: String,
    first_response_minutes: i32,
    resolution_minutes: i32,
}

/// `POST /api/tickets` body.
#[derive(Debug, Deserialize)]
struct TicketPayload {
    title: String,
    #[serde(default = "default_priority")]
    priority: String,
    #[serde(default = "default_ticket_channel")]
    channel: String,
    #[serde(default)]
    contact_pid: Option<Uuid>,
    #[serde(default)]
    assignee_ref: Option<String>,
}

/// `POST /api/tickets/{pid}/status` body.
#[derive(Debug, Deserialize)]
struct TicketStatusPayload {
    to: String,
}

/// `PUT /api/tickets/{pid}/priority` body (audited; re-derives SLA).
#[derive(Debug, Deserialize)]
struct PriorityPayload {
    priority: String,
    reason: String,
}

/// `POST /api/articles` body.
#[derive(Debug, Deserialize)]
struct ArticlePayload {
    title: String,
    body: String,
    #[serde(default)]
    keywords: Option<String>,
}

/// A `{pid}` reference response.
#[derive(Debug, Serialize)]
struct PidRef {
    pid: String,
}

fn default_priority() -> String {
    "normal".to_string()
}
fn default_ticket_channel() -> String {
    "web".to_string()
}

/// The SLA targets for a priority, when a policy exists.
async fn targets_for<C: sea_orm::ConnectionTrait>(
    db: &C,
    priority: &str,
) -> Result<Option<sla::Targets>> {
    let policy = sla_policies::Entity::find()
        .filter(sla_policies::Column::Priority.eq(priority))
        .filter(sla_policies::Column::DeletedAt.is_null())
        .one(db)
        .await?;
    Ok(policy.map(|p| sla::Targets {
        first_response_minutes: p.first_response_minutes,
        resolution_minutes: p.resolution_minutes,
    }))
}

/// `POST /api/sla-policies`.
#[debug_handler]
async fn create_policy(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<PolicyPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("priority", tokens::PRIORITIES, &payload.priority);
    if payload.first_response_minutes <= 0 || payload.resolution_minutes <= 0 {
        problems.push("SLA minutes must be positive".to_string());
    }
    if payload.first_response_minutes > payload.resolution_minutes {
        problems.push("first response target exceeds the resolution target".to_string());
    }
    ensure_valid(&problems.into_vec())?;
    let txn = ctx.db.begin().await?;
    let row = sla_policies::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        priority: ActiveValue::set(payload.priority.clone()),
        first_response_minutes: ActiveValue::set(payload.first_response_minutes),
        resolution_minutes: ActiveValue::set(payload.resolution_minutes),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "sla_policy", row.pid, "created", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef { pid: row.pid.to_string() })
}

/// `GET /api/sla-policies`.
#[debug_handler]
async fn list_policies(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = sla_policies::Entity::find()
        .filter(sla_policies::Column::DeletedAt.is_null())
        .order_by_asc(sla_policies::Column::Id)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

/// `POST /api/tickets` — open; deadlines derive from the priority's
/// policy at open (CRM-R11).
#[debug_handler]
async fn create_ticket(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<TicketPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("title", &payload.title);
    problems.require_token("priority", tokens::PRIORITIES, &payload.priority);
    problems.require_token("channel", tokens::TICKET_CHANNELS, &payload.channel);
    problems.ref_opt("assignee_ref", entity_ref::EntityType::Worker, payload.assignee_ref.as_deref());
    ensure_valid(&problems.into_vec())?;
    let account_pid = if let Some(contact) = payload.contact_pid {
        records::find_contact(&ctx.db, contact).await?.account_pid
    } else {
        None
    };
    let opened_at: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    let deadlines = targets_for(&ctx.db, &payload.priority)
        .await?
        .map(|targets| sla::deadlines(opened_at, targets));
    let txn = ctx.db.begin().await?;
    let row = tickets::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        contact_pid: ActiveValue::set(payload.contact_pid),
        account_pid: ActiveValue::set(account_pid),
        assignee_ref: ActiveValue::set(payload.assignee_ref.clone()),
        title: ActiveValue::set(payload.title.clone()),
        priority: ActiveValue::set(payload.priority.clone()),
        channel: ActiveValue::set(payload.channel.clone()),
        status: ActiveValue::set("open".to_string()),
        opened_at: ActiveValue::set(opened_at),
        first_response_due_at: ActiveValue::set(deadlines.as_ref().map(|d| d.first_response_due_at)),
        resolution_due_at: ActiveValue::set(deadlines.as_ref().map(|d| d.resolution_due_at)),
        first_responded_at: ActiveValue::set(None),
        resolved_at: ActiveValue::set(None),
        first_response_breached: ActiveValue::set(false),
        resolution_breached: ActiveValue::set(false),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "ticket", row.pid, "ticket_opened", caller.actor(), None).await?;
    streaming::emit_on(&txn, "ticket", "ticket_opened", &row.pid.to_string(), &row.title, caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef { pid: row.pid.to_string() })
}

/// `GET /api/tickets?status=` — the queue with **live** breach flags
/// (computed on read; the sweep persists + emits them).
#[derive(Debug, Deserialize)]
struct TicketListParams {
    #[serde(default)]
    status: Option<String>,
}

#[debug_handler]
async fn list_tickets(
    State(ctx): State<AppContext>,
    Query(params): Query<TicketListParams>,
) -> Result<Response> {
    let mut query = tickets::Entity::find().filter(tickets::Column::DeletedAt.is_null());
    if let Some(status) = &params.status {
        query = query.filter(tickets::Column::Status.eq(status));
    }
    let rows = query
        .order_by_asc(tickets::Column::Id)
        .limit(1000)
        .all(&ctx.db)
        .await?;
    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    let rows: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|ticket| {
            let live_first = ticket.first_response_due_at.is_some_and(|due| {
                sla::first_response_breached(now, due, ticket.first_responded_at)
            });
            let live_resolution = ticket
                .resolution_due_at
                .is_some_and(|due| sla::resolution_breached(now, due, ticket.resolved_at));
            let mut value = serde_json::to_value(&ticket).unwrap_or_default();
            if let Some(object) = value.as_object_mut() {
                object.insert("live_first_response_breached".into(), live_first.into());
                object.insert("live_resolution_breached".into(), live_resolution.into());
            }
            value
        })
        .collect();
    format::json(rows)
}

/// `GET /api/tickets/{pid}` — one ticket + its activities.
#[debug_handler]
async fn get_ticket(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let ticket = records::find_ticket(&ctx.db, records::parse_pid(&pid)?).await?;
    let activity_rows = activities::Entity::find()
        .filter(activities::Column::SubjectKind.eq("ticket"))
        .filter(activities::Column::SubjectPid.eq(ticket.pid))
        .filter(activities::Column::DeletedAt.is_null())
        .order_by_asc(activities::Column::Id)
        .all(&ctx.db)
        .await?;
    format::json(serde_json::json!({ "ticket": ticket, "activities": activity_rows }))
}

/// `POST /api/tickets/{pid}/status` — one lifecycle transition;
/// `resolved` stamps `resolved_at`.
#[debug_handler]
async fn ticket_status(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<TicketStatusPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("to", tokens::TICKET_STATUSES, &payload.to);
    ensure_valid(&problems.into_vec())?;
    let ticket = records::find_ticket(&ctx.db, records::parse_pid(&pid)?).await?;
    lifecycle::check("ticket", lifecycle::TICKET, &ticket.status, &payload.to)
        .map_err(|e| unprocessable(&e))?;
    let txn = ctx.db.begin().await?;
    let from = ticket.status.clone();
    let title = ticket.title.clone();
    let mut active: tickets::ActiveModel = ticket.into();
    active.status = ActiveValue::set(payload.to.clone());
    if payload.to == "resolved" {
        active.resolved_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    } else if payload.to == "open" && from == "resolved" {
        active.resolved_at = ActiveValue::set(None); // reopen clears it
    }
    let row = active.update(&txn).await?;
    let kind = match payload.to.as_str() {
        "resolved" => "ticket_resolved",
        "closed" => "ticket_closed",
        _ => "ticket_status_changed",
    };
    Audit::record(
        &txn,
        "ticket",
        row.pid,
        kind,
        caller.actor(),
        Some(serde_json::json!({ "from": from })),
    )
    .await?;
    streaming::emit_on(&txn, "ticket", kind, &row.pid.to_string(), &title, caller.actor(), None).await?;
    txn.commit().await?;
    format::json(row)
}

/// `PUT /api/tickets/{pid}/priority` — audited priority change;
/// re-derives the deadlines from the opened time (CRM-R11).
#[debug_handler]
async fn ticket_priority(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<PriorityPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("priority", tokens::PRIORITIES, &payload.priority);
    problems.require_text("reason", &payload.reason);
    ensure_valid(&problems.into_vec())?;
    let ticket = records::find_ticket(&ctx.db, records::parse_pid(&pid)?).await?;
    let deadlines = targets_for(&ctx.db, &payload.priority)
        .await?
        .map(|targets| sla::deadlines(ticket.opened_at, targets));
    let txn = ctx.db.begin().await?;
    let from = ticket.priority.clone();
    let mut active: tickets::ActiveModel = ticket.into();
    active.priority = ActiveValue::set(payload.priority.clone());
    active.first_response_due_at = ActiveValue::set(deadlines.as_ref().map(|d| d.first_response_due_at));
    active.resolution_due_at = ActiveValue::set(deadlines.as_ref().map(|d| d.resolution_due_at));
    let row = active.update(&txn).await?;
    Audit::record(
        &txn,
        "ticket",
        row.pid,
        "priority_changed",
        caller.actor(),
        Some(serde_json::json!({ "from": from, "to": payload.priority, "reason": payload.reason })),
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `POST /api/sla/sweep` — persist breach facts and emit
/// `sla_breached` **once per breach** (idempotent: only rows whose
/// stored flag is still false flip and emit).
#[debug_handler]
async fn sla_sweep(State(ctx): State<AppContext>, caller: MaybeAuthUser) -> Result<Response> {
    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    let open = tickets::Entity::find()
        .filter(tickets::Column::DeletedAt.is_null())
        .filter(tickets::Column::Status.is_in(["open", "pending", "resolved"]))
        .all(&ctx.db)
        .await?;
    let mut breaches = 0_u64;
    for ticket in open {
        let first = ticket
            .first_response_due_at
            .is_some_and(|due| sla::first_response_breached(now, due, ticket.first_responded_at));
        let resolution = ticket
            .resolution_due_at
            .is_some_and(|due| sla::resolution_breached(now, due, ticket.resolved_at));
        let new_first = first && !ticket.first_response_breached;
        let new_resolution = resolution && !ticket.resolution_breached;
        if !new_first && !new_resolution {
            continue;
        }
        let txn = ctx.db.begin().await?;
        let pid = ticket.pid;
        let mut active: tickets::ActiveModel = ticket.into();
        if new_first {
            active.first_response_breached = ActiveValue::set(true);
        }
        if new_resolution {
            active.resolution_breached = ActiveValue::set(true);
        }
        active.update(&txn).await?;
        streaming::emit_on(
            &txn,
            "ticket",
            "sla_breached",
            &pid.to_string(),
            "",
            caller.actor(),
            Some(serde_json::json!({
                "first_response": new_first, "resolution": new_resolution,
            })),
        )
        .await?;
        txn.commit().await?;
        breaches += u64::from(new_first) + u64::from(new_resolution);
        Metrics::global().sla_breached_total.inc();
    }
    format::json(serde_json::json!({ "new_breaches": breaches }))
}

/// `POST /api/articles` — draft an article.
#[debug_handler]
async fn create_article(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<ArticlePayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("title", &payload.title);
    problems.require_text("body", &payload.body);
    problems.cap_opt("keywords", payload.keywords.as_deref());
    ensure_valid(&problems.into_vec())?;
    let txn = ctx.db.begin().await?;
    let row = articles::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        title: ActiveValue::set(payload.title.clone()),
        body: ActiveValue::set(payload.body.clone()),
        keywords: ActiveValue::set(payload.keywords.clone()),
        status: ActiveValue::set("draft".to_string()),
        version: ActiveValue::set(1),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "article", row.pid, "created", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef { pid: row.pid.to_string() })
}

/// `PUT /api/articles/{pid}` — edit; a **published** edit bumps the
/// version (CRM-R12).
#[debug_handler]
async fn update_article(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<ArticlePayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("title", &payload.title);
    problems.require_text("body", &payload.body);
    ensure_valid(&problems.into_vec())?;
    let article = records::find_article(&ctx.db, records::parse_pid(&pid)?).await?;
    if article.status == "archived" {
        return Err(unprocessable("archived articles are read-only"));
    }
    let txn = ctx.db.begin().await?;
    let published = article.status == "published";
    let version = article.version;
    let mut active: articles::ActiveModel = article.into();
    active.title = ActiveValue::set(payload.title.clone());
    active.body = ActiveValue::set(payload.body.clone());
    active.keywords = ActiveValue::set(payload.keywords.clone());
    if published {
        active.version = ActiveValue::set(version + 1);
    }
    let row = active.update(&txn).await?;
    Audit::record(&txn, "article", row.pid, "updated", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(row)
}

/// `POST /api/articles/{pid}/status` — publish / archive.
#[derive(Debug, Deserialize)]
struct ArticleStatusPayload {
    to: String,
}

#[debug_handler]
async fn article_status(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<ArticleStatusPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("to", tokens::ARTICLE_STATUSES, &payload.to);
    ensure_valid(&problems.into_vec())?;
    let article = records::find_article(&ctx.db, records::parse_pid(&pid)?).await?;
    lifecycle::check("article", lifecycle::ARTICLE, &article.status, &payload.to)
        .map_err(|e| unprocessable(&e))?;
    let txn = ctx.db.begin().await?;
    let mut active: articles::ActiveModel = article.into();
    active.status = ActiveValue::set(payload.to.clone());
    let row = active.update(&txn).await?;
    let kind = if payload.to == "published" { "article_published" } else { "updated" };
    Audit::record(&txn, "article", row.pid, kind, caller.actor(), None).await?;
    streaming::emit_on(&txn, "article", kind, &row.pid.to_string(), &row.title, caller.actor(), None).await?;
    txn.commit().await?;
    format::json(row)
}

/// `GET /api/articles?q=` — keyword search (ILIKE-style contains over
/// title/keywords; published first).
#[derive(Debug, Deserialize)]
struct ArticleSearchParams {
    #[serde(default)]
    q: Option<String>,
}

#[debug_handler]
async fn list_articles(
    State(ctx): State<AppContext>,
    Query(params): Query<ArticleSearchParams>,
) -> Result<Response> {
    let rows = articles::Entity::find()
        .filter(articles::Column::DeletedAt.is_null())
        .order_by_asc(articles::Column::Id)
        .limit(500)
        .all(&ctx.db)
        .await?;
    let rows: Vec<_> = if let Some(q) = params.q.as_deref().map(str::to_ascii_lowercase) {
        rows.into_iter()
            .filter(|a| {
                a.title.to_ascii_lowercase().contains(&q)
                    || a.keywords.as_deref().unwrap_or("").to_ascii_lowercase().contains(&q)
            })
            .collect()
    } else {
        rows
    };
    format::json(rows)
}

/// The support routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/sla-policies", post(create_policy))
        .add("/sla-policies", get(list_policies))
        .add("/tickets", post(create_ticket))
        .add("/tickets", get(list_tickets))
        .add("/tickets/{pid}", get(get_ticket))
        .add("/tickets/{pid}/status", post(ticket_status))
        .add("/tickets/{pid}/priority", put(ticket_priority))
        .add("/sla/sweep", post(sla_sweep))
        .add("/articles", post(create_article))
        .add("/articles", get(list_articles))
        .add("/articles/{pid}", put(update_article))
        .add("/articles/{pid}/status", post(article_status))
}
