//! Care-pathway **instance layer** — a patient enrolled on a pathway
//! template, with a status/urgency lifecycle, a step-completion log, a
//! care-team roster, recorded events, and a scheduled next review.
//! Operational state that references a `person:` URN and the template
//! pid; it is **not** part of the matcher payload (the registry owns
//! pathway identities). The derived views (caseload, overdue reviews,
//! chronic cohorts, care-team load) live at the end.

use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use sea_orm::{ActiveValue, QueryOrder};
use uuid::Uuid;

use crate::auth::MaybeAuthUser;
use crate::instances as rules;
use crate::models::_entities::{
    instance_events, instance_steps, instance_team, pathway_instances,
};
use crate::models::audit_logs::Model as Audit;
use crate::models::care_pathways::Model as PathwayModel;

/// `422` with a reason.
fn refuse(reason: &str) -> Error {
    Error::CustomError(
        axum::http::StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("unprocessable_entity", reason),
    )
}

/// Parse a pid or 404.
fn pid(raw: &str) -> Result<Uuid> {
    Uuid::parse_str(raw).map_err(|_| Error::NotFound)
}

/// Validate a `person:<uuid>` subject URN.
fn valid_subject(value: &str) -> bool {
    value
        .split_once(':')
        .is_some_and(|(scheme, id)| scheme == "person" && Uuid::parse_str(id).is_ok())
}

/// Validate a `worker:`/`person:`/`organization:` team-member URN.
fn valid_member(value: &str) -> bool {
    value.split_once(':').is_some_and(|(scheme, id)| {
        matches!(scheme, "worker" | "person" | "organization") && Uuid::parse_str(id).is_ok()
    })
}

/// Find one live instance, or 404.
async fn find_instance(ctx: &AppContext, raw: &str) -> Result<pathway_instances::Model> {
    pathway_instances::Entity::find()
        .filter(pathway_instances::Column::Pid.eq(pid(raw)?))
        .filter(pathway_instances::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)
}

/// `POST /api/care-pathways/{pathway}/instances` body.
#[derive(Debug, serde::Deserialize)]
struct EnrollPayload {
    subject_ref: String,
    #[serde(default)]
    urgency: Option<String>,
    #[serde(default)]
    next_review_on: Option<chrono::NaiveDate>,
    /// Optional initial steps (ordered).
    #[serde(default)]
    steps: Vec<String>,
}

/// `POST /api/care-pathways/{pathway}/instances` — enrol a subject on a
/// pathway template. Copies the declared steps into the instance's
/// checklist.
#[debug_handler]
async fn enroll(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pathway): Path<String>,
    Json(payload): Json<EnrollPayload>,
) -> Result<Response> {
    if !valid_subject(&payload.subject_ref) {
        return Err(refuse("subject_ref must be a person:<uuid> URN"));
    }
    let urgency = payload.urgency.clone().unwrap_or_else(|| "routine".to_string());
    if !rules::URGENCY_LEVELS.contains(&urgency.as_str()) {
        return Err(refuse(&format!("urgency must be one of {:?}", rules::URGENCY_LEVELS)));
    }
    // The template must exist (registry ownership; 404 otherwise).
    let template = PathwayModel::find_by_pid(&ctx.db, &pathway)
        .await
        .map_err(|_| Error::NotFound)?;
    let txn = ctx.db.begin().await?;
    let instance = pathway_instances::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        pathway_pid: ActiveValue::set(template.pid),
        subject_ref: ActiveValue::set(payload.subject_ref.clone()),
        status: ActiveValue::set("active".to_string()),
        urgency: ActiveValue::set(urgency.clone()),
        enrolled_on: ActiveValue::set(chrono::Utc::now().date_naive()),
        next_review_on: ActiveValue::set(payload.next_review_on),
        closed_on: ActiveValue::set(None),
        closure_reason: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    for (index, label) in payload.steps.iter().enumerate() {
        if label.trim().is_empty() {
            continue;
        }
        instance_steps::ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            instance_pid: ActiveValue::set(instance.pid),
            label: ActiveValue::set(label.clone()),
            done: ActiveValue::set(false),
            done_on: ActiveValue::set(None),
            position: ActiveValue::set(i32::try_from(index).unwrap_or(i32::MAX)),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
    }
    Audit::record(
        &txn,
        instance.pid,
        "instance_enrolled",
        caller.actor(),
        Some(serde_json::json!({ "pathway_pid": template.pid, "urgency": urgency })),
    )
    .await
    .map_err(Error::Model)?;
    txn.commit().await?;
    format::json(instance)
}

/// `GET /api/care-pathways/{pathway}/instances` — the template's live
/// instances (newest first).
#[debug_handler]
async fn list_for_pathway(
    State(ctx): State<AppContext>,
    Path(pathway): Path<String>,
) -> Result<Response> {
    let template = PathwayModel::find_by_pid(&ctx.db, &pathway)
        .await
        .map_err(|_| Error::NotFound)?;
    let rows = pathway_instances::Entity::find()
        .filter(pathway_instances::Column::PathwayPid.eq(template.pid))
        .filter(pathway_instances::Column::DeletedAt.is_null())
        .order_by_desc(pathway_instances::Column::Id)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

/// `GET /api/instances/{pid}` — one instance with its steps, care team,
/// and event log.
#[debug_handler]
async fn get_instance(State(ctx): State<AppContext>, Path(raw): Path<String>) -> Result<Response> {
    let instance = find_instance(&ctx, &raw).await?;
    let mut steps = instance_steps::Entity::find()
        .filter(instance_steps::Column::InstancePid.eq(instance.pid))
        .all(&ctx.db)
        .await?;
    steps.sort_by_key(|s| s.position);
    let team = instance_team::Entity::find()
        .filter(instance_team::Column::InstancePid.eq(instance.pid))
        .all(&ctx.db)
        .await?;
    let events = instance_events::Entity::find()
        .filter(instance_events::Column::InstancePid.eq(instance.pid))
        .order_by_desc(instance_events::Column::OccurredAt)
        .all(&ctx.db)
        .await?;
    format::json(serde_json::json!({
        "instance": instance, "steps": steps, "team": team, "events": events,
    }))
}

/// `POST /api/instances/{pid}/status` body.
#[derive(Debug, serde::Deserialize)]
struct StatusPayload {
    to: String,
    #[serde(default)]
    reason: Option<String>,
}

/// `POST /api/instances/{pid}/status` — the enrolment lifecycle move
/// (pure machine); closing stamps `closed_on` + `closure_reason`.
#[debug_handler]
async fn set_status(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(raw): Path<String>,
    Json(payload): Json<StatusPayload>,
) -> Result<Response> {
    let instance = find_instance(&ctx, &raw).await?;
    rules::instance_transition(&instance.status, &payload.to).map_err(|r| refuse(&r))?;
    let from = instance.status.clone();
    let instance_pid = instance.pid;
    let today = chrono::Utc::now().date_naive();
    let mut active: pathway_instances::ActiveModel = instance.into();
    active.status = ActiveValue::set(payload.to.clone());
    if rules::is_terminal(&payload.to) {
        active.closed_on = ActiveValue::set(Some(today));
        active.closure_reason = ActiveValue::set(payload.reason.clone());
        active.next_review_on = ActiveValue::set(None);
    }
    let updated = active.update(&ctx.db).await?;
    Audit::record(
        &ctx.db,
        instance_pid,
        "instance_status_changed",
        caller.actor(),
        Some(serde_json::json!({ "from": from, "to": payload.to })),
    )
    .await
    .map_err(Error::Model)?;
    format::json(updated)
}

/// `POST /api/instances/{pid}/review` body.
#[derive(Debug, serde::Deserialize)]
struct ReviewPayload {
    #[serde(default)]
    note: Option<String>,
    /// The next review date to schedule (chronic cadence).
    next_review_on: chrono::NaiveDate,
}

/// `POST /api/instances/{pid}/review` — record a review event and
/// reschedule the next review (the chronic-management cadence).
#[debug_handler]
async fn record_review(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(raw): Path<String>,
    Json(payload): Json<ReviewPayload>,
) -> Result<Response> {
    let instance = find_instance(&ctx, &raw).await?;
    if rules::is_terminal(&instance.status) {
        return Err(refuse("a closed instance is not reviewed"));
    }
    let instance_pid = instance.pid;
    let mut active: pathway_instances::ActiveModel = instance.into();
    active.next_review_on = ActiveValue::set(Some(payload.next_review_on));
    let updated = active.update(&ctx.db).await?;
    instance_events::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        instance_pid: ActiveValue::set(instance_pid),
        kind: ActiveValue::set("review".to_string()),
        occurred_at: ActiveValue::set(chrono::Utc::now().into()),
        note: ActiveValue::set(payload.note.clone()),
        actor: ActiveValue::set(caller.actor().map(ToString::to_string)),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await?;
    Audit::record(&ctx.db, instance_pid, "instance_reviewed", caller.actor(), None)
        .await
        .map_err(Error::Model)?;
    format::json(updated)
}

/// `POST /api/instances/{pid}/urgency` body.
#[derive(Debug, serde::Deserialize)]
struct UrgencyPayload {
    to: String,
    #[serde(default)]
    note: Option<String>,
}

/// `POST /api/instances/{pid}/urgency` — change urgency; records an
/// `escalation` / `de_escalation` event by direction.
#[debug_handler]
async fn set_urgency(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(raw): Path<String>,
    Json(payload): Json<UrgencyPayload>,
) -> Result<Response> {
    if !rules::URGENCY_LEVELS.contains(&payload.to.as_str()) {
        return Err(refuse(&format!("urgency must be one of {:?}", rules::URGENCY_LEVELS)));
    }
    let instance = find_instance(&ctx, &raw).await?;
    let from_rank = rules::URGENCY_LEVELS.iter().position(|u| *u == instance.urgency);
    let to_rank = rules::URGENCY_LEVELS.iter().position(|u| *u == payload.to);
    let kind = match (from_rank, to_rank) {
        (Some(f), Some(t)) if t > f => "escalation",
        (Some(f), Some(t)) if t < f => "de_escalation",
        _ => "note",
    };
    let from = instance.urgency.clone();
    let instance_pid = instance.pid;
    let mut active: pathway_instances::ActiveModel = instance.into();
    active.urgency = ActiveValue::set(payload.to.clone());
    let updated = active.update(&ctx.db).await?;
    instance_events::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        instance_pid: ActiveValue::set(instance_pid),
        kind: ActiveValue::set(kind.to_string()),
        occurred_at: ActiveValue::set(chrono::Utc::now().into()),
        note: ActiveValue::set(payload.note.clone()),
        actor: ActiveValue::set(caller.actor().map(ToString::to_string)),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await?;
    Audit::record(
        &ctx.db,
        instance_pid,
        "instance_urgency_changed",
        caller.actor(),
        Some(serde_json::json!({ "from": from, "to": payload.to })),
    )
    .await
    .map_err(Error::Model)?;
    format::json(updated)
}

/// `POST /api/instances/{pid}/steps/{step}/complete` — mark a step
/// done (stamps `done_on`; idempotent).
#[debug_handler]
async fn complete_step(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path((raw, step_raw)): Path<(String, String)>,
) -> Result<Response> {
    let instance = find_instance(&ctx, &raw).await?;
    let step = instance_steps::Entity::find()
        .filter(instance_steps::Column::Pid.eq(pid(&step_raw)?))
        .one(&ctx.db)
        .await?
        .filter(|s| s.instance_pid == instance.pid)
        .ok_or(Error::NotFound)?;
    if step.done {
        return format::json(step);
    }
    let step_pid = step.pid;
    let mut active: instance_steps::ActiveModel = step.into();
    active.done = ActiveValue::set(true);
    active.done_on = ActiveValue::set(Some(chrono::Utc::now().date_naive()));
    let updated = active.update(&ctx.db).await?;
    Audit::record(&ctx.db, step_pid, "instance_step_completed", caller.actor(), None)
        .await
        .map_err(Error::Model)?;
    format::json(updated)
}

/// `POST /api/instances/{pid}/team` body.
#[derive(Debug, serde::Deserialize)]
struct TeamPayload {
    member_ref: String,
    role: String,
}

/// `POST /api/instances/{pid}/team` — add a care-team member
/// (`422` if the (member, role) pair is already on the team).
#[debug_handler]
async fn add_team_member(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(raw): Path<String>,
    Json(payload): Json<TeamPayload>,
) -> Result<Response> {
    if !valid_member(&payload.member_ref) {
        return Err(refuse("member_ref must be a worker:/person:/organization: URN"));
    }
    if !rules::TEAM_ROLES.contains(&payload.role.as_str()) {
        return Err(refuse(&format!("role must be one of {:?}", rules::TEAM_ROLES)));
    }
    let instance = find_instance(&ctx, &raw).await?;
    let existing = instance_team::Entity::find()
        .filter(instance_team::Column::InstancePid.eq(instance.pid))
        .filter(instance_team::Column::MemberRef.eq(payload.member_ref.clone()))
        .filter(instance_team::Column::Role.eq(payload.role.clone()))
        .one(&ctx.db)
        .await?;
    if existing.is_some() {
        return Err(refuse("that member already holds that role on this instance"));
    }
    let row = instance_team::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        instance_pid: ActiveValue::set(instance.pid),
        member_ref: ActiveValue::set(payload.member_ref.clone()),
        role: ActiveValue::set(payload.role.clone()),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await?;
    Audit::record(
        &ctx.db,
        instance.pid,
        "instance_team_added",
        caller.actor(),
        Some(serde_json::json!({ "member_ref": payload.member_ref, "role": payload.role })),
    )
    .await
    .map_err(Error::Model)?;
    format::json(row)
}

/// `POST /api/instances/{pid}/events` body.
#[derive(Debug, serde::Deserialize)]
struct EventPayload {
    kind: String,
    #[serde(default)]
    note: Option<String>,
}

/// `POST /api/instances/{pid}/events` — record a free event (note /
/// referral / …). Reviews and escalations have their own endpoints.
#[debug_handler]
async fn add_event(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(raw): Path<String>,
    Json(payload): Json<EventPayload>,
) -> Result<Response> {
    if !rules::EVENT_KINDS.contains(&payload.kind.as_str()) {
        return Err(refuse(&format!("kind must be one of {:?}", rules::EVENT_KINDS)));
    }
    let instance = find_instance(&ctx, &raw).await?;
    let row = instance_events::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        instance_pid: ActiveValue::set(instance.pid),
        kind: ActiveValue::set(payload.kind.clone()),
        occurred_at: ActiveValue::set(chrono::Utc::now().into()),
        note: ActiveValue::set(payload.note.clone()),
        actor: ActiveValue::set(caller.actor().map(ToString::to_string)),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await?;
    format::json(row)
}

// ─── Derived views ──────────────────────────────────────────────────────────

/// `GET /api/instances/caseload` — active instances by care setting and
/// urgency (setting comes from each instance's pathway template), plus
/// the emergency/urgent count.
#[debug_handler]
async fn caseload(State(ctx): State<AppContext>) -> Result<Response> {
    let instances = pathway_instances::Entity::find()
        .filter(pathway_instances::Column::DeletedAt.is_null())
        .filter(pathway_instances::Column::Status.is_in(["active", "on_hold"]))
        .all(&ctx.db)
        .await?;
    let templates = PathwayModel::list(&ctx.db, 1000).await?;
    let mut setting_of: std::collections::BTreeMap<Uuid, String> = std::collections::BTreeMap::new();
    for template in &templates {
        let setting = template
            .to_pathway()
            .ok()
            .and_then(|p| p.care_setting.map(|s| format!("{s:?}")))
            .unwrap_or_else(|| "unspecified".to_string());
        setting_of.insert(template.pid, setting);
    }
    let mut by_setting: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut by_urgency: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for instance in &instances {
        let setting = setting_of
            .get(&instance.pathway_pid)
            .cloned()
            .unwrap_or_else(|| "unspecified".to_string());
        *by_setting.entry(setting).or_default() += 1;
        *by_urgency.entry(instance.urgency.clone()).or_default() += 1;
    }
    let urgent = instances
        .iter()
        .filter(|i| matches!(i.urgency.as_str(), "urgent" | "emergency"))
        .count();
    format::json(serde_json::json!({
        "as_of": chrono::Utc::now(),
        "note": "open instances (active + on_hold); setting from each instance's pathway template",
        "open": instances.len(),
        "by_setting": by_setting,
        "by_urgency": by_urgency,
        "urgent_or_emergency": urgent,
    }))
}

/// `GET /api/instances/overdue-reviews` — open instances whose
/// `next_review_on` has passed (or is unset), most overdue first — the
/// chronic-management review register.
#[debug_handler]
async fn overdue_reviews(State(ctx): State<AppContext>) -> Result<Response> {
    let today = chrono::Utc::now().date_naive();
    let instances = pathway_instances::Entity::find()
        .filter(pathway_instances::Column::DeletedAt.is_null())
        .filter(pathway_instances::Column::Status.is_in(["active", "on_hold"]))
        .all(&ctx.db)
        .await?;
    let mut rows: Vec<(i64, serde_json::Value)> = instances
        .iter()
        .filter_map(|instance| {
            let overdue_days = instance
                .next_review_on
                .map_or(i64::MAX, |due| (today - due).num_days());
            (overdue_days > 0).then(|| {
                (overdue_days, serde_json::json!({
                    "pid": instance.pid,
                    "subject_ref": instance.subject_ref,
                    "urgency": instance.urgency,
                    "next_review_on": instance.next_review_on,
                    "overdue_days": if overdue_days == i64::MAX {
                        serde_json::Value::Null
                    } else {
                        serde_json::json!(overdue_days)
                    },
                    "unscheduled": instance.next_review_on.is_none(),
                }))
            })
        })
        .collect();
    rows.sort_by_key(|(days, _)| std::cmp::Reverse(*days));
    format::json(serde_json::json!({
        "as_of": today,
        "note": "open instances past next_review_on (or with none scheduled); \
                 unscheduled sort first",
        "overdue": rows.into_iter().map(|(_, v)| v).collect::<Vec<_>>(),
    }))
}

/// `GET /api/care-pathways/{pathway}/cohort` — the chronic cohort on
/// one pathway: instance counts by status + urgency, and step
/// completion across the active cohort.
#[debug_handler]
async fn cohort(State(ctx): State<AppContext>, Path(pathway): Path<String>) -> Result<Response> {
    let template = PathwayModel::find_by_pid(&ctx.db, &pathway)
        .await
        .map_err(|_| Error::NotFound)?;
    let instances = pathway_instances::Entity::find()
        .filter(pathway_instances::Column::PathwayPid.eq(template.pid))
        .filter(pathway_instances::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    let mut by_status: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut by_urgency: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for instance in &instances {
        *by_status.entry(instance.status.clone()).or_default() += 1;
        *by_urgency.entry(instance.urgency.clone()).or_default() += 1;
    }
    let instance_pids: Vec<Uuid> = instances.iter().map(|i| i.pid).collect();
    let (mut steps_done, mut steps_total) = (0usize, 0usize);
    if !instance_pids.is_empty() {
        let steps = instance_steps::Entity::find()
            .filter(instance_steps::Column::InstancePid.is_in(instance_pids))
            .all(&ctx.db)
            .await?;
        steps_total = steps.len();
        steps_done = steps.iter().filter(|s| s.done).count();
    }
    format::json(serde_json::json!({
        "as_of": chrono::Utc::now(),
        "pathway": { "pid": template.pid, "name": template.name },
        "instances": instances.len(),
        "by_status": by_status,
        "by_urgency": by_urgency,
        "step_completion": { "done": steps_done, "total": steps_total },
    }))
}

/// `GET /api/instances/care-team-load` — active-instance count per care
/// team member (with their roles), the per-provider caseload lens.
#[debug_handler]
async fn care_team_load(State(ctx): State<AppContext>) -> Result<Response> {
    let open: std::collections::HashSet<Uuid> = pathway_instances::Entity::find()
        .filter(pathway_instances::Column::DeletedAt.is_null())
        .filter(pathway_instances::Column::Status.is_in(["active", "on_hold"]))
        .all(&ctx.db)
        .await?
        .into_iter()
        .map(|i| i.pid)
        .collect();
    let team = instance_team::Entity::find().all(&ctx.db).await?;
    // member → (open instance set, roles)
    let mut per_member: std::collections::BTreeMap<
        String,
        (std::collections::HashSet<Uuid>, std::collections::BTreeSet<String>),
    > = std::collections::BTreeMap::new();
    for row in &team {
        if !open.contains(&row.instance_pid) {
            continue;
        }
        let entry = per_member.entry(row.member_ref.clone()).or_default();
        entry.0.insert(row.instance_pid);
        entry.1.insert(row.role.clone());
    }
    let mut members: Vec<serde_json::Value> = per_member
        .iter()
        .map(|(member, (instances, roles))| {
            serde_json::json!({
                "member_ref": member,
                "open_instances": instances.len(),
                "roles": roles,
            })
        })
        .collect();
    members.sort_by_key(|m| std::cmp::Reverse(m["open_instances"].as_u64().unwrap_or(0)));
    format::json(serde_json::json!({
        "as_of": chrono::Utc::now(),
        "note": "open-instance load per care-team member (active + on_hold)",
        "members": members,
    }))
}

/// Instance-scoped routes (prefix `/api/instances`).
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/instances")
        .add("/caseload", get(caseload))
        .add("/overdue-reviews", get(overdue_reviews))
        .add("/care-team-load", get(care_team_load))
        .add("/{pid}", get(get_instance))
        .add("/{pid}/status", post(set_status))
        .add("/{pid}/review", post(record_review))
        .add("/{pid}/urgency", post(set_urgency))
        .add("/{pid}/team", post(add_team_member))
        .add("/{pid}/events", post(add_event))
        .add("/{pid}/steps/{step}/complete", post(complete_step))
}

/// Pathway-scoped instance routes (prefix `/api/care-pathways`), added
/// before the registry's `/{pid}` capture so the literal sub-paths win.
pub fn pathway_routes() -> Routes {
    Routes::new()
        .prefix("/api/care-pathways")
        .add("/{pathway}/instances", post(enroll))
        .add("/{pathway}/instances", get(list_for_pathway))
        .add("/{pathway}/cohort", get(cohort))
}
