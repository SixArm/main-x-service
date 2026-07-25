//! The relationship layer (CRM-R1, CRM-R2): contacts + accounts as
//! URN wrappers, the merged timeline, the manual repoint endpoint,
//! and activities. Every mutation runs on one transaction (CRM-D9).

use loco_rs::prelude::*;
use sea_orm::{QueryOrder, QuerySelect, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ensure_valid, unprocessable};
use crate::auth::MaybeAuthUser;
use crate::models::_entities::{accounts, activities, contacts, deals, tickets};
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
use crate::rules::tokens;
use crate::streaming;
use crate::validation::Problems;

/// `POST /api/contacts` body.
#[derive(Debug, Deserialize)]
struct ContactPayload {
    person_ref: String,
    display_name: String,
    #[serde(default)]
    account_pid: Option<Uuid>,
    #[serde(default)]
    owner_ref: Option<String>,
    #[serde(default)]
    job_title: Option<String>,
    #[serde(default = "default_channel")]
    preferred_channel: String,
}

/// `POST /api/accounts` body.
#[derive(Debug, Deserialize)]
struct AccountPayload {
    organization_ref: String,
    display_name: String,
    #[serde(default = "default_tier")]
    tier: String,
    #[serde(default)]
    owner_ref: Option<String>,
    #[serde(default)]
    industry: Option<String>,
}

/// `POST /api/contacts/{pid}/repoint` body — the manual post-merge
/// repoint (CRM-D1).
#[derive(Debug, Deserialize)]
struct RepointPayload {
    person_ref: String,
    reason: String,
}

/// `POST /api/activities` body.
#[derive(Debug, Deserialize)]
struct ActivityPayload {
    subject_kind: String,
    subject_pid: Uuid,
    kind: String,
    summary: String,
    #[serde(default)]
    occurred_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    #[serde(default)]
    actor_ref: Option<String>,
    #[serde(default)]
    due_on: Option<chrono::NaiveDate>,
    /// Optional recorded sentiment (`positive` / `neutral` / `negative`).
    #[serde(default)]
    sentiment: Option<String>,
}

/// A `{pid}` reference response.
#[derive(Debug, Serialize)]
struct PidRef {
    pid: String,
}

fn default_channel() -> String {
    "email".to_string()
}
fn default_tier() -> String {
    "prospect".to_string()
}

/// `POST /api/contacts` — create the wrapper (consent starts `never`).
#[debug_handler]
async fn create_contact(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<ContactPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_ref(
        "person_ref",
        entity_ref::EntityType::Person,
        &payload.person_ref,
    );
    problems.require_text("display_name", &payload.display_name);
    problems.ref_opt(
        "owner_ref",
        entity_ref::EntityType::Worker,
        payload.owner_ref.as_deref(),
    );
    problems.require_token(
        "preferred_channel",
        tokens::CHANNELS,
        &payload.preferred_channel,
    );
    problems.cap_opt("job_title", payload.job_title.as_deref());
    ensure_valid(&problems.into_vec())?;
    if let Some(account) = payload.account_pid {
        records::find_account(&ctx.db, account).await?;
    }
    let txn = ctx.db.begin().await?;
    let row = contacts::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        person_ref: ActiveValue::set(payload.person_ref.clone()),
        account_pid: ActiveValue::set(payload.account_pid),
        owner_ref: ActiveValue::set(payload.owner_ref.clone()),
        display_name: ActiveValue::set(payload.display_name.clone()),
        status: ActiveValue::set("active".to_string()),
        job_title: ActiveValue::set(payload.job_title.clone()),
        preferred_channel: ActiveValue::set(payload.preferred_channel.clone()),
        marketing_consent: ActiveValue::set("never".to_string()),
        consent_changed_at: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(
        &txn,
        "contact",
        row.pid,
        "created",
        caller.actor(),
        Some(serde_json::json!({ "owner": row.owner_ref })),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "contact",
        "created",
        &row.pid.to_string(),
        &row.display_name,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: row.pid.to_string(),
    })
}

/// `GET /api/contacts` — active contacts.
#[debug_handler]
async fn list_contacts(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = contacts::Entity::find()
        .filter(contacts::Column::DeletedAt.is_null())
        .order_by_asc(contacts::Column::Id)
        .limit(1000)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

/// `GET /api/contacts/{pid}` — the contact + its merged timeline
/// (activities, deals, tickets — chronological; CRM-R1).
#[debug_handler]
async fn get_contact(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let contact = records::find_contact(&ctx.db, records::parse_pid(&pid)?).await?;
    let activity_rows = activities::Entity::find()
        .filter(activities::Column::SubjectKind.eq("contact"))
        .filter(activities::Column::SubjectPid.eq(contact.pid))
        .filter(activities::Column::DeletedAt.is_null())
        .order_by_desc(activities::Column::OccurredAt)
        .limit(200)
        .all(&ctx.db)
        .await?;
    let deal_rows = deals::Entity::find()
        .filter(deals::Column::PrimaryContactPid.eq(contact.pid))
        .filter(deals::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    let ticket_rows = tickets::Entity::find()
        .filter(tickets::Column::ContactPid.eq(contact.pid))
        .filter(tickets::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    format::json(serde_json::json!({
        "contact": contact,
        "activities": activity_rows,
        "deals": deal_rows,
        "tickets": ticket_rows,
    }))
}

/// `POST /api/contacts/{pid}/repoint` — repoint the wrapper after an
/// upstream registry merge (manual v1; audited with the reason).
#[debug_handler]
async fn repoint_contact(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<RepointPayload>,
) -> Result<Response> {
    let contact = records::find_contact(&ctx.db, records::parse_pid(&pid)?).await?;
    let mut problems = Problems::new();
    problems.require_ref(
        "person_ref",
        entity_ref::EntityType::Person,
        &payload.person_ref,
    );
    problems.require_text("reason", &payload.reason);
    ensure_valid(&problems.into_vec())?;
    let txn = ctx.db.begin().await?;
    let from = contact.person_ref.clone();
    let mut active: contacts::ActiveModel = contact.into();
    active.person_ref = ActiveValue::set(payload.person_ref.clone());
    let row = active.update(&txn).await?;
    Audit::record(
        &txn,
        "contact",
        row.pid,
        "repointed",
        caller.actor(),
        Some(
            serde_json::json!({ "from": from, "to": payload.person_ref, "reason": payload.reason }),
        ),
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `POST /api/accounts`.
#[debug_handler]
async fn create_account(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<AccountPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_ref(
        "organization_ref",
        entity_ref::EntityType::Organization,
        &payload.organization_ref,
    );
    problems.require_text("display_name", &payload.display_name);
    problems.require_token("tier", tokens::ACCOUNT_TIERS, &payload.tier);
    problems.ref_opt(
        "owner_ref",
        entity_ref::EntityType::Worker,
        payload.owner_ref.as_deref(),
    );
    problems.cap_opt("industry", payload.industry.as_deref());
    ensure_valid(&problems.into_vec())?;
    let txn = ctx.db.begin().await?;
    let row = accounts::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        organization_ref: ActiveValue::set(payload.organization_ref.clone()),
        owner_ref: ActiveValue::set(payload.owner_ref.clone()),
        display_name: ActiveValue::set(payload.display_name.clone()),
        tier: ActiveValue::set(payload.tier.clone()),
        industry: ActiveValue::set(payload.industry.clone()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "account", row.pid, "created", caller.actor(), None).await?;
    streaming::emit_on(
        &txn,
        "account",
        "created",
        &row.pid.to_string(),
        &row.display_name,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: row.pid.to_string(),
    })
}

/// `GET /api/accounts`.
#[debug_handler]
async fn list_accounts(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = accounts::Entity::find()
        .filter(accounts::Column::DeletedAt.is_null())
        .order_by_asc(accounts::Column::Id)
        .limit(500)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

/// `GET /api/accounts/{pid}` — account + its contacts and deals.
#[debug_handler]
async fn get_account(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let account = records::find_account(&ctx.db, records::parse_pid(&pid)?).await?;
    let contact_rows = contacts::Entity::find()
        .filter(contacts::Column::AccountPid.eq(account.pid))
        .filter(contacts::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    let deal_rows = deals::Entity::find()
        .filter(deals::Column::AccountPid.eq(account.pid))
        .filter(deals::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    format::json(serde_json::json!({
        "account": account, "contacts": contact_rows, "deals": deal_rows,
    }))
}

/// `POST /api/activities` — log an interaction against any
/// relationship object (CRM-R2). Stamps the ticket first-response
/// when an assignee's outbound call/email hits an open ticket.
#[debug_handler]
async fn create_activity(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<ActivityPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token(
        "subject_kind",
        tokens::ACTIVITY_SUBJECTS,
        &payload.subject_kind,
    );
    problems.token_opt(
        "sentiment",
        crate::rules::engagement::SENTIMENTS,
        payload.sentiment.as_deref(),
    );
    problems.require_token("kind", tokens::ACTIVITY_KINDS, &payload.kind);
    problems.require_text("summary", &payload.summary);
    problems.ref_opt(
        "actor_ref",
        entity_ref::EntityType::Worker,
        payload.actor_ref.as_deref(),
    );
    ensure_valid(&problems.into_vec())?;
    let occurred_at = payload
        .occurred_at
        .unwrap_or_else(|| chrono::Utc::now().into());
    let txn = ctx.db.begin().await?;
    let row = activities::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        subject_kind: ActiveValue::set(payload.subject_kind.clone()),
        sentiment: ActiveValue::set(payload.sentiment.clone()),
        subject_pid: ActiveValue::set(payload.subject_pid),
        kind: ActiveValue::set(payload.kind.clone()),
        occurred_at: ActiveValue::set(occurred_at),
        actor_ref: ActiveValue::set(payload.actor_ref.clone()),
        summary: ActiveValue::set(payload.summary.clone()),
        due_on: ActiveValue::set(payload.due_on),
        done: ActiveValue::set(false),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    // First-response stamping (CRM-R10): the assignee's first
    // outbound call/email on an open/pending ticket.
    if payload.subject_kind == "ticket"
        && (payload.kind == "call" || payload.kind == "email")
        && let Ok(ticket) = records::find_ticket(&txn, payload.subject_pid).await
    {
        {
            let is_assignee =
                ticket.assignee_ref.is_some() && ticket.assignee_ref == payload.actor_ref;
            if is_assignee && ticket.first_responded_at.is_none() && ticket.status != "closed" {
                let ticket_pid = ticket.pid;
                let mut active: tickets::ActiveModel = ticket.into();
                active.first_responded_at = ActiveValue::set(Some(occurred_at));
                active.update(&txn).await?;
                streaming::emit_on(
                    &txn,
                    "ticket",
                    "ticket_first_response",
                    &ticket_pid.to_string(),
                    "",
                    caller.actor(),
                    None,
                )
                .await?;
            }
        }
    }
    Audit::record(&txn, "activity", row.pid, "created", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: row.pid.to_string(),
    })
}

/// `GET /api/activities?subject_kind=&subject_pid=` — one object's
/// activities, or the recent feed with no filter (CRM-R14).
#[derive(Debug, Deserialize)]
struct ActivityListParams {
    #[serde(default)]
    subject_kind: Option<String>,
    #[serde(default)]
    subject_pid: Option<Uuid>,
}

#[debug_handler]
async fn list_activities(
    State(ctx): State<AppContext>,
    Query(params): Query<ActivityListParams>,
) -> Result<Response> {
    let mut query = activities::Entity::find().filter(activities::Column::DeletedAt.is_null());
    if let Some(kind) = &params.subject_kind {
        query = query.filter(activities::Column::SubjectKind.eq(kind));
    }
    if let Some(pid) = params.subject_pid {
        query = query.filter(activities::Column::SubjectPid.eq(pid));
    }
    let rows = query
        .order_by_desc(activities::Column::OccurredAt)
        .limit(200)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

/// `PUT /api/activities/{pid}/done` — tick a task activity.
#[debug_handler]
async fn activity_done(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let pid = records::parse_pid(&pid)?;
    let row = activities::Entity::find()
        .filter(activities::Column::Pid.eq(pid))
        .filter(activities::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;
    if row.kind != "task" {
        return Err(unprocessable("only task activities can be marked done"));
    }
    let txn = ctx.db.begin().await?;
    let mut active: activities::ActiveModel = row.into();
    active.done = ActiveValue::set(true);
    let row = active.update(&txn).await?;
    Audit::record(&txn, "activity", row.pid, "updated", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(row)
}

/// The relationship routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/contacts", post(create_contact))
        .add("/contacts", get(list_contacts))
        .add("/contacts/{pid}", get(get_contact))
        .add("/contacts/{pid}/repoint", post(repoint_contact))
        .add("/accounts", post(create_account))
        .add("/accounts", get(list_accounts))
        .add("/accounts/{pid}", get(get_account))
        .add("/activities", post(create_activity))
        .add("/activities", get(list_activities))
        .add("/activities/{pid}/done", put(activity_done))
}
