//! Stakeholder-engagement / innovation-partnership / confederation
//! sub-resources — all **declared** data (a role, a grid score, a
//! partnership stage, a membership) recorded by an operator; nothing
//! here is inferred. Lifecycles live in
//! [`crate::rules::engagement`]; the derived views live in
//! [`super::insights`].

use loco_rs::prelude::*;
use sea_orm::{ActiveValue, QueryOrder, QuerySelect};
use serde::Deserialize;
use uuid::Uuid;

use super::{ensure_valid, unprocessable};
use crate::models::_entities::{
    activities, contacts, memberships, partnerships, working_group_members, working_groups,
};
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
use crate::rules::engagement as rules;
use crate::validation::Problems;

/// The auth extractor (actor stamping only).
use crate::auth::MaybeAuthUser;

/// `PUT /api/contacts/{pid}/stakeholder` body — declared typing; an
/// omitted field clears the declaration.
#[derive(Debug, Deserialize)]
struct ContactStakeholderPayload {
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    influence: Option<i32>,
    #[serde(default)]
    interest: Option<i32>,
}

/// `PUT /api/contacts/{pid}/stakeholder` — declare (or clear) a
/// contact's stakeholder role and power–interest scores (1–5).
#[debug_handler]
async fn declare_contact_stakeholder(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<ContactStakeholderPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.token_opt("role", rules::STAKEHOLDER_ROLES, payload.role.as_deref());
    for (field, score) in [("influence", payload.influence), ("interest", payload.interest)] {
        if let Some(score) = score
            && !rules::valid_grid_score(score)
        {
            problems.push(format!("{field} must be between 1 and 5"));
        }
    }
    ensure_valid(&problems.into_vec())?;
    let contact = records::find_contact(&ctx.db, records::parse_pid(&pid)?).await?;
    let contact_pid = contact.pid;
    let mut active: contacts::ActiveModel = contact.into();
    active.stakeholder_role = ActiveValue::set(payload.role.clone());
    active.influence = ActiveValue::set(payload.influence);
    active.interest = ActiveValue::set(payload.interest);
    let row = active.update(&ctx.db).await?;
    Audit::record(
        &ctx.db,
        "contact",
        contact_pid,
        "stakeholder_declared",
        caller.actor(),
        Some(serde_json::json!({
            "role": payload.role, "influence": payload.influence, "interest": payload.interest,
        })),
    )
    .await?;
    format::json(row)
}

/// `PUT /api/accounts/{pid}/stakeholder` body.
#[derive(Debug, Deserialize)]
struct AccountStakeholderPayload {
    #[serde(default)]
    role: Option<String>,
}

/// `PUT /api/accounts/{pid}/stakeholder` — declare (or clear) an
/// account's stakeholder role.
#[debug_handler]
async fn declare_account_stakeholder(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<AccountStakeholderPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.token_opt("role", rules::STAKEHOLDER_ROLES, payload.role.as_deref());
    ensure_valid(&problems.into_vec())?;
    let account = records::find_account(&ctx.db, records::parse_pid(&pid)?).await?;
    let account_pid = account.pid;
    let mut active: crate::models::_entities::accounts::ActiveModel = account.into();
    active.stakeholder_role = ActiveValue::set(payload.role.clone());
    let row = active.update(&ctx.db).await?;
    Audit::record(
        &ctx.db,
        "account",
        account_pid,
        "stakeholder_declared",
        caller.actor(),
        Some(serde_json::json!({ "role": payload.role })),
    )
    .await?;
    format::json(row)
}

/// `POST /api/accounts/{pid}/partnerships` body.
#[derive(Debug, Deserialize)]
struct PartnershipPayload {
    kind: String,
    summary: String,
    #[serde(default)]
    started_on: Option<chrono::NaiveDate>,
}

/// `POST /api/accounts/{pid}/partnerships` — open a partnership record
/// (starts at `scouting`).
#[debug_handler]
async fn create_partnership(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<PartnershipPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("kind", rules::PARTNERSHIP_KINDS, &payload.kind);
    problems.require_text("summary", &payload.summary);
    problems.cap_text("summary", &payload.summary);
    ensure_valid(&problems.into_vec())?;
    let account = records::find_account(&ctx.db, records::parse_pid(&pid)?).await?;
    let row = partnerships::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        account_pid: ActiveValue::set(account.pid),
        kind: ActiveValue::set(payload.kind.clone()),
        stage: ActiveValue::set("scouting".to_string()),
        summary: ActiveValue::set(payload.summary.clone()),
        started_on: ActiveValue::set(payload.started_on),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await?;
    Audit::record(&ctx.db, "partnership", row.pid, "created", caller.actor(), None).await?;
    format::json(row)
}

/// `GET /api/accounts/{pid}/partnerships` — the account's partnership
/// records, newest first.
#[debug_handler]
async fn list_partnerships(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
) -> Result<Response> {
    let account = records::find_account(&ctx.db, records::parse_pid(&pid)?).await?;
    let rows = partnerships::Entity::find()
        .filter(partnerships::Column::AccountPid.eq(account.pid))
        .filter(partnerships::Column::DeletedAt.is_null())
        .order_by_desc(partnerships::Column::Id)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

/// `POST /api/partnerships/{pid}/stage` body.
#[derive(Debug, Deserialize)]
struct StagePayload {
    to: String,
}

/// `POST /api/partnerships/{pid}/stage` — the lifecycle move (forward
/// one step, or retire; the pure machine refuses everything else).
#[debug_handler]
async fn partnership_stage(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<StagePayload>,
) -> Result<Response> {
    let row = partnerships::Entity::find()
        .filter(partnerships::Column::Pid.eq(records::parse_pid(&pid)?))
        .filter(partnerships::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;
    rules::partnership_transition(&row.stage, &payload.to)
        .map_err(|reason| unprocessable(&reason))?;
    let from = row.stage.clone();
    let row_pid = row.pid;
    let mut active: partnerships::ActiveModel = row.into();
    active.stage = ActiveValue::set(payload.to.clone());
    let updated = active.update(&ctx.db).await?;
    Audit::record(
        &ctx.db,
        "partnership",
        row_pid,
        "partnership_stage_changed",
        caller.actor(),
        Some(serde_json::json!({ "from": from, "to": payload.to })),
    )
    .await?;
    format::json(updated)
}

/// `PUT /api/accounts/{pid}/membership` body.
#[derive(Debug, Deserialize)]
struct MembershipPayload {
    joined_on: chrono::NaiveDate,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    renewal_on: Option<chrono::NaiveDate>,
}

/// `PUT /api/accounts/{pid}/membership` — declare or update the
/// account's confederation membership (one record per account).
#[debug_handler]
async fn upsert_membership(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<MembershipPayload>,
) -> Result<Response> {
    let status = payload.status.clone().unwrap_or_else(|| "active".to_string());
    let mut problems = Problems::new();
    problems.require_token("status", rules::MEMBERSHIP_STATUSES, &status);
    ensure_valid(&problems.into_vec())?;
    let account = records::find_account(&ctx.db, records::parse_pid(&pid)?).await?;
    let existing = memberships::Entity::find()
        .filter(memberships::Column::AccountPid.eq(account.pid))
        .one(&ctx.db)
        .await?;
    let row = match existing {
        Some(row) => {
            let mut active: memberships::ActiveModel = row.into();
            active.joined_on = ActiveValue::set(payload.joined_on);
            active.status = ActiveValue::set(status.clone());
            active.renewal_on = ActiveValue::set(payload.renewal_on);
            active.deleted_at = ActiveValue::set(None);
            active.update(&ctx.db).await?
        }
        None => {
            memberships::ActiveModel {
                pid: ActiveValue::set(Uuid::new_v4()),
                account_pid: ActiveValue::set(account.pid),
                joined_on: ActiveValue::set(payload.joined_on),
                status: ActiveValue::set(status.clone()),
                renewal_on: ActiveValue::set(payload.renewal_on),
                deleted_at: ActiveValue::set(None),
                ..Default::default()
            }
            .insert(&ctx.db)
            .await?
        }
    };
    Audit::record(
        &ctx.db,
        "membership",
        row.pid,
        "membership_declared",
        caller.actor(),
        Some(serde_json::json!({ "status": status, "renewal_on": payload.renewal_on })),
    )
    .await?;
    format::json(row)
}

/// `POST /api/groups` body.
#[derive(Debug, Deserialize)]
struct GroupPayload {
    name: String,
    #[serde(default)]
    purpose: Option<String>,
}

/// `POST /api/groups` — create a working group.
#[debug_handler]
async fn create_group(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<GroupPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("name", &payload.name);
    problems.cap_text("name", &payload.name);
    problems.cap_opt("purpose", payload.purpose.as_deref());
    ensure_valid(&problems.into_vec())?;
    let row = working_groups::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        name: ActiveValue::set(payload.name.clone()),
        purpose: ActiveValue::set(payload.purpose.clone()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await?;
    Audit::record(&ctx.db, "working_group", row.pid, "created", caller.actor(), None).await?;
    format::json(row)
}

/// `GET /api/groups` — all live working groups with member counts.
#[debug_handler]
async fn list_groups(State(ctx): State<AppContext>) -> Result<Response> {
    let groups = working_groups::Entity::find()
        .filter(working_groups::Column::DeletedAt.is_null())
        .order_by_asc(working_groups::Column::Id)
        .all(&ctx.db)
        .await?;
    let members = working_group_members::Entity::find().all(&ctx.db).await?;
    let mut counts: std::collections::BTreeMap<Uuid, usize> = std::collections::BTreeMap::new();
    for member in &members {
        *counts.entry(member.group_pid).or_default() += 1;
    }
    let view: Vec<serde_json::Value> = groups
        .iter()
        .map(|group| {
            serde_json::json!({
                "pid": group.pid,
                "name": group.name,
                "purpose": group.purpose,
                "members": counts.get(&group.pid).copied().unwrap_or(0),
            })
        })
        .collect();
    format::json(view)
}

/// `POST /api/groups/{pid}/members` body.
#[derive(Debug, Deserialize)]
struct MemberPayload {
    contact_pid: Uuid,
}

/// `POST /api/groups/{pid}/members` — add a contact to the roster
/// (`422` if already a member).
#[debug_handler]
async fn add_member(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<MemberPayload>,
) -> Result<Response> {
    let group = find_group(&ctx, &pid).await?;
    let contact = records::find_contact(&ctx.db, payload.contact_pid).await?;
    let already = working_group_members::Entity::find()
        .filter(working_group_members::Column::GroupPid.eq(group.pid))
        .filter(working_group_members::Column::ContactPid.eq(contact.pid))
        .one(&ctx.db)
        .await?;
    if already.is_some() {
        return Err(unprocessable("contact is already a member of this group"));
    }
    working_group_members::ActiveModel {
        group_pid: ActiveValue::set(group.pid),
        contact_pid: ActiveValue::set(contact.pid),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await?;
    Audit::record(
        &ctx.db,
        "working_group",
        group.pid,
        "member_added",
        caller.actor(),
        Some(serde_json::json!({ "contact_pid": contact.pid })),
    )
    .await?;
    format::json(serde_json::json!({ "group_pid": group.pid, "contact_pid": contact.pid }))
}

/// `GET /api/groups/{pid}` — the group, its roster (with each
/// member's account), and the recent activity feed derived from the
/// members' recorded activities (cap 50; derivation disclosed).
#[debug_handler]
async fn get_group(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let group = find_group(&ctx, &pid).await?;
    let member_rows = working_group_members::Entity::find()
        .filter(working_group_members::Column::GroupPid.eq(group.pid))
        .all(&ctx.db)
        .await?;
    let member_pids: Vec<Uuid> = member_rows.iter().map(|m| m.contact_pid).collect();
    let contact_rows = contacts::Entity::find()
        .filter(contacts::Column::Pid.is_in(member_pids.clone()))
        .filter(contacts::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    let roster: Vec<serde_json::Value> = contact_rows
        .iter()
        .map(|contact| {
            serde_json::json!({
                "pid": contact.pid,
                "display_name": contact.display_name,
                "account_pid": contact.account_pid,
                "stakeholder_role": contact.stakeholder_role,
            })
        })
        .collect();
    let feed = if member_pids.is_empty() {
        Vec::new()
    } else {
        activities::Entity::find()
            .filter(activities::Column::SubjectKind.eq("contact"))
            .filter(activities::Column::SubjectPid.is_in(member_pids))
            .filter(activities::Column::DeletedAt.is_null())
            .order_by_desc(activities::Column::OccurredAt)
            .limit(50)
            .all(&ctx.db)
            .await?
    };
    format::json(serde_json::json!({
        "group": group,
        "roster": roster,
        "feed": feed,
        "feed_note": "recent contact-subject activities of roster members (cap 50)",
    }))
}

/// Find one live working group by pid, or 404.
async fn find_group(ctx: &AppContext, pid: &str) -> Result<working_groups::Model> {
    working_groups::Entity::find()
        .filter(working_groups::Column::Pid.eq(records::parse_pid(pid)?))
        .filter(working_groups::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)
}

/// The engagement routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/contacts/{pid}/stakeholder", put(declare_contact_stakeholder))
        .add("/accounts/{pid}/stakeholder", put(declare_account_stakeholder))
        .add("/accounts/{pid}/partnerships", post(create_partnership))
        .add("/accounts/{pid}/partnerships", get(list_partnerships))
        .add("/partnerships/{pid}/stage", post(partnership_stage))
        .add("/accounts/{pid}/membership", put(upsert_membership))
        .add("/groups", post(create_group))
        .add("/groups", get(list_groups))
        .add("/groups/{pid}", get(get_group))
        .add("/groups/{pid}/members", post(add_member))
}
