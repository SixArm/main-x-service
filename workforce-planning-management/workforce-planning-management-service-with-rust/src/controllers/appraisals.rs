//! 360° appraisals (WPM-R29) — multi-rater feedback around a subject
//! employee: nominations by group (`self | manager | peer | report`),
//! once-per-rater responses while collecting, and a group-floored
//! report once shared. Rater anonymity is **procedural** (WPM-D21):
//! the store links a response to its nomination so once-per-rater and
//! completion tracking hold, but no endpoint ever serves rater-level
//! content — the detail view shows *who* responded, the report shows
//! group aggregates only, with `peer`/`report` cells withheld below a
//! floor of 3 (count included).

use std::collections::BTreeMap;

use loco_rs::prelude::*;
use sea_orm::{ActiveValue, QueryOrder};
use serde::Deserialize;
use uuid::Uuid;

use super::{ensure_valid, record_rejection, unprocessable};
use crate::auth::{self, MaybeAuthUser};
use crate::models::_entities::{appraisal_nominations, appraisal_responses, appraisals, employees};
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
use crate::rules::appraisal as rules;
use crate::validation::Problems;

/// A `{pid}` reference response.
#[derive(serde::Serialize)]
struct PidRef {
    pid: String,
}

/// `POST /api/employees/{pid}/appraisals` body.
#[derive(Debug, Deserialize)]
struct AppraisalPayload {
    competencies: Vec<String>,
}

/// `POST /api/employees/{pid}/appraisals` — open a draft 360° for the
/// subject; the subject's `self` nomination is created automatically.
#[debug_handler]
async fn create_appraisal(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<AppraisalPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    if payload.competencies.is_empty() {
        problems.push("at least one competency is required");
    }
    if payload.competencies.len() > rules::MAX_COMPETENCIES {
        problems.push(format!("at most {} competencies", rules::MAX_COMPETENCIES));
    }
    problems.cap_list("competencies", &payload.competencies);
    let mut seen = std::collections::HashSet::new();
    if payload.competencies.iter().any(|c| !seen.insert(c.trim().to_lowercase())) {
        problems.push("competencies must be unique");
    }
    ensure_valid(&problems.into_vec())?;
    let subject = records::find_employee(&ctx.db, records::parse_pid(&pid)?).await?;
    let txn = ctx.db.begin().await?;
    let row = appraisals::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        employee_pid: ActiveValue::set(subject.pid),
        competencies: ActiveValue::set(serde_json::json!(payload.competencies)),
        status: ActiveValue::set("draft".to_string()),
        shared_on: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    appraisal_nominations::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        appraisal_pid: ActiveValue::set(row.pid),
        rater_pid: ActiveValue::set(subject.pid),
        rater_group: ActiveValue::set("self".to_string()),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "appraisal", row.pid, "created", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef { pid: row.pid.to_string() })
}

/// `GET /api/employees/{pid}/appraisals` — the subject's appraisals
/// with nomination/response counts (never content).
#[debug_handler]
async fn list_appraisals(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let subject = records::find_employee(&ctx.db, records::parse_pid(&pid)?).await?;
    auth::authorize_record(
        &caller,
        authentication_verifier::Action::Read,
        &auth::employee_resource_attrs(&subject),
    )
    .map_err(record_rejection)?;
    let rows = appraisals::Entity::find()
        .filter(appraisals::Column::EmployeePid.eq(subject.pid))
        .filter(appraisals::Column::DeletedAt.is_null())
        .order_by_desc(appraisals::Column::Id)
        .all(&ctx.db)
        .await?;
    let mut view = Vec::new();
    for appraisal in &rows {
        let nominated = appraisal_nominations::Entity::find()
            .filter(appraisal_nominations::Column::AppraisalPid.eq(appraisal.pid))
            .all(&ctx.db)
            .await?
            .len();
        let responded = appraisal_responses::Entity::find()
            .filter(appraisal_responses::Column::AppraisalPid.eq(appraisal.pid))
            .all(&ctx.db)
            .await?
            .len();
        view.push(serde_json::json!({
            "pid": appraisal.pid,
            "status": appraisal.status,
            "competencies": appraisal.competencies,
            "shared_on": appraisal.shared_on,
            "nominated": nominated,
            "responded": responded,
        }));
    }
    format::json(view)
}

/// `POST /api/appraisals/{pid}/nominations` body.
#[derive(Debug, Deserialize)]
struct NominationPayload {
    rater_pid: Uuid,
    group: String,
}

/// `POST /api/appraisals/{pid}/nominations` — invite a rater (draft
/// only; `self` is automatic and cannot be nominated; one nomination
/// per rater; at most [`rules::MAX_RATERS`]).
#[debug_handler]
async fn nominate(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<NominationPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("group", rules::GROUPS, &payload.group);
    if payload.group == "self" {
        problems.push("the self nomination is automatic");
    }
    ensure_valid(&problems.into_vec())?;
    let appraisal = find_appraisal(&ctx, &pid).await?;
    if appraisal.status != "draft" {
        return Err(unprocessable("nominations are frozen once collecting"));
    }
    let rater = records::find_employee(&ctx.db, payload.rater_pid).await?;
    if rater.pid == appraisal.employee_pid {
        return Err(unprocessable("the subject rates only as `self`"));
    }
    let existing = appraisal_nominations::Entity::find()
        .filter(appraisal_nominations::Column::AppraisalPid.eq(appraisal.pid))
        .all(&ctx.db)
        .await?;
    if existing.iter().any(|n| n.rater_pid == rater.pid) {
        return Err(unprocessable("this rater is already nominated"));
    }
    if existing.len() >= rules::MAX_RATERS {
        return Err(unprocessable("at most 12 raters"));
    }
    let row = appraisal_nominations::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        appraisal_pid: ActiveValue::set(appraisal.pid),
        rater_pid: ActiveValue::set(rater.pid),
        rater_group: ActiveValue::set(payload.group.clone()),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await?;
    Audit::record(&ctx.db, "appraisal_nomination", row.pid, "created", caller.actor(), None)
        .await?;
    format::json(PidRef { pid: row.pid.to_string() })
}

/// `POST /api/appraisals/{pid}/status` body.
#[derive(Debug, Deserialize)]
struct StatusPayload {
    to: String,
}

/// `POST /api/appraisals/{pid}/status` — the lifecycle move (pure
/// machine); `collecting` requires ≥ 3 non-self nominations; sharing
/// stamps `shared_on`.
#[debug_handler]
async fn appraisal_status(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<StatusPayload>,
) -> Result<Response> {
    let appraisal = find_appraisal(&ctx, &pid).await?;
    rules::transition(&appraisal.status, &payload.to).map_err(|reason| unprocessable(&reason))?;
    if payload.to == "collecting" {
        let non_self = appraisal_nominations::Entity::find()
            .filter(appraisal_nominations::Column::AppraisalPid.eq(appraisal.pid))
            .filter(appraisal_nominations::Column::RaterGroup.ne("self"))
            .all(&ctx.db)
            .await?
            .len();
        if non_self < rules::MIN_NON_SELF_RATERS {
            return Err(unprocessable(&format!(
                "collecting requires at least {} non-self raters, have {non_self}",
                rules::MIN_NON_SELF_RATERS
            )));
        }
    }
    let from = appraisal.status.clone();
    let row_pid = appraisal.pid;
    let mut active: appraisals::ActiveModel = appraisal.into();
    active.status = ActiveValue::set(payload.to.clone());
    if payload.to == "shared" {
        active.shared_on = ActiveValue::set(Some(chrono::Utc::now().date_naive()));
    }
    let updated = active.update(&ctx.db).await?;
    Audit::record(
        &ctx.db,
        "appraisal",
        row_pid,
        "status_changed",
        caller.actor(),
        Some(serde_json::json!({ "from": from, "to": payload.to })),
    )
    .await?;
    format::json(updated)
}

/// `GET /api/appraisals/{pid}` — the appraisal + its nominations with
/// display names and a responded flag. **Who** responded, never what
/// (WPM-D21).
#[debug_handler]
async fn get_appraisal(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let appraisal = find_appraisal(&ctx, &pid).await?;
    let nominations = appraisal_nominations::Entity::find()
        .filter(appraisal_nominations::Column::AppraisalPid.eq(appraisal.pid))
        .order_by_asc(appraisal_nominations::Column::Id)
        .all(&ctx.db)
        .await?;
    let responses = appraisal_responses::Entity::find()
        .filter(appraisal_responses::Column::AppraisalPid.eq(appraisal.pid))
        .all(&ctx.db)
        .await?;
    let responded: std::collections::HashSet<Uuid> =
        responses.iter().map(|r| r.nomination_pid).collect();
    let mut nomination_view = Vec::new();
    for nomination in &nominations {
        let rater = employees::Entity::find()
            .filter(employees::Column::Pid.eq(nomination.rater_pid))
            .one(&ctx.db)
            .await?;
        nomination_view.push(serde_json::json!({
            "pid": nomination.pid,
            "rater_pid": nomination.rater_pid,
            "display_name": rater.map(|r| r.display_name),
            "group": nomination.rater_group,
            "responded": responded.contains(&nomination.pid),
        }));
    }
    format::json(serde_json::json!({
        "pid": appraisal.pid,
        "employee_pid": appraisal.employee_pid,
        "status": appraisal.status,
        "competencies": appraisal.competencies,
        "shared_on": appraisal.shared_on,
        "nominations": nomination_view,
    }))
}

/// `POST /api/appraisals/{pid}/responses` body.
#[derive(Debug, Deserialize)]
struct ResponsePayload {
    rater_pid: Uuid,
    scores: BTreeMap<String, i32>,
    #[serde(default)]
    comment: Option<String>,
}

/// `POST /api/appraisals/{pid}/responses` — one rater's response:
/// collecting only, nominated raters only, once per rater, every
/// declared competency scored 1–5. `$sub` ownership applies to the
/// rater. The audit row carries the actor (procedural anonymity —
/// accountability without content disclosure: no endpoint serves
/// rater-level answers).
#[debug_handler]
async fn respond(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<ResponsePayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.cap_opt("comment", payload.comment.as_deref());
    ensure_valid(&problems.into_vec())?;
    let appraisal = find_appraisal(&ctx, &pid).await?;
    if appraisal.status != "collecting" {
        return Err(unprocessable("responses are accepted only while collecting"));
    }
    let declared: Vec<String> =
        serde_json::from_value(appraisal.competencies.clone()).unwrap_or_default();
    rules::check_scores(&declared, &payload.scores).map_err(|reason| unprocessable(&reason))?;
    let rater = records::find_employee(&ctx.db, payload.rater_pid).await?;
    auth::authorize_record(
        &caller,
        authentication_verifier::Action::Write,
        &auth::employee_resource_attrs(&rater),
    )
    .map_err(record_rejection)?;
    let nomination = appraisal_nominations::Entity::find()
        .filter(appraisal_nominations::Column::AppraisalPid.eq(appraisal.pid))
        .filter(appraisal_nominations::Column::RaterPid.eq(rater.pid))
        .one(&ctx.db)
        .await?
        .ok_or_else(|| unprocessable("this employee is not a nominated rater"))?;
    let already = appraisal_responses::Entity::find()
        .filter(appraisal_responses::Column::NominationPid.eq(nomination.pid))
        .one(&ctx.db)
        .await?;
    if already.is_some() {
        return Err(unprocessable("this rater has already responded"));
    }
    let row = appraisal_responses::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        appraisal_pid: ActiveValue::set(appraisal.pid),
        nomination_pid: ActiveValue::set(nomination.pid),
        rater_group: ActiveValue::set(nomination.rater_group.clone()),
        scores: ActiveValue::set(serde_json::json!(payload.scores)),
        comment: ActiveValue::set(payload.comment.clone()),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await?;
    Audit::record(&ctx.db, "appraisal_response", row.pid, "submitted", caller.actor(), None)
        .await?;
    format::json(serde_json::json!({ "submitted": true }))
}

/// `GET /api/appraisals/{pid}/report` — the group-floored report,
/// readable once `shared` (`422` before). Per group × competency
/// count + mean, and group-pooled comments sorted alphabetically (so
/// ordering reveals no submission sequence); `peer`/`report` cells
/// below the floor of 3 are withheld — count included (WPM-D21).
/// Reads are audited (review-content sensitivity, WPM-R10 posture).
#[debug_handler]
async fn report(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let appraisal = find_appraisal(&ctx, &pid).await?;
    if appraisal.status != "shared" {
        return Err(unprocessable("the report is readable once shared"));
    }
    let subject = records::find_employee(&ctx.db, appraisal.employee_pid).await?;
    auth::authorize_record(
        &caller,
        authentication_verifier::Action::Read,
        &auth::employee_resource_attrs(&subject),
    )
    .map_err(record_rejection)?;
    let declared: Vec<String> =
        serde_json::from_value(appraisal.competencies.clone()).unwrap_or_default();
    let responses = appraisal_responses::Entity::find()
        .filter(appraisal_responses::Column::AppraisalPid.eq(appraisal.pid))
        .all(&ctx.db)
        .await?;
    let mut groups = Vec::new();
    for group in rules::GROUPS {
        let group_responses: Vec<&appraisal_responses::Model> =
            responses.iter().filter(|r| r.rater_group == *group).collect();
        if group_responses.is_empty() {
            continue;
        }
        if !rules::group_discloses(group, group_responses.len()) {
            groups.push(serde_json::json!({ "group": group, "withheld": true }));
            continue;
        }
        let mut competencies = serde_json::Map::new();
        for competency in &declared {
            let scores: Vec<i32> = group_responses
                .iter()
                .filter_map(|r| {
                    r.scores.get(competency).and_then(serde_json::Value::as_i64)
                })
                .filter_map(|s| i32::try_from(s).ok())
                .collect();
            if let Some((count, mean)) = rules::competency_mean(&scores) {
                competencies.insert(
                    competency.clone(),
                    serde_json::json!({ "count": count, "mean": mean }),
                );
            }
        }
        let mut comments: Vec<&str> = group_responses
            .iter()
            .filter_map(|r| r.comment.as_deref())
            .filter(|c| !c.trim().is_empty())
            .collect();
        comments.sort_unstable();
        groups.push(serde_json::json!({
            "group": group,
            "withheld": false,
            "responses": group_responses.len(),
            "competencies": competencies,
            "comments": comments,
        }));
    }
    Audit::record(&ctx.db, "appraisal", appraisal.pid, "report_read", caller.actor(), None)
        .await?;
    format::json(serde_json::json!({
        "appraisal": {
            "pid": appraisal.pid,
            "employee_pid": appraisal.employee_pid,
            "competencies": declared,
            "shared_on": appraisal.shared_on,
        },
        "groups": groups,
        "derivation": "group aggregates only — no rater-level content exists on any \
                       endpoint; peer/report cells with fewer than 3 responses are \
                       withheld (count included); manager/self disclose at 1 by \
                       convention; comments are pooled per group and sorted \
                       alphabetically; development-facing, not a payroll input",
    }))
}

/// `GET /api/employees/{pid}/appraisal-requests` — the rater's own
/// pending requests: `collecting` appraisals where they are nominated
/// and have not yet responded, with the subject, group, and declared
/// competencies. `$sub`-owned; discloses only what the rater already
/// knows (that they were invited).
#[debug_handler]
async fn rater_requests(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let rater = records::find_employee(&ctx.db, records::parse_pid(&pid)?).await?;
    auth::authorize_record(
        &caller,
        authentication_verifier::Action::Read,
        &auth::employee_resource_attrs(&rater),
    )
    .map_err(record_rejection)?;
    let nominations = appraisal_nominations::Entity::find()
        .filter(appraisal_nominations::Column::RaterPid.eq(rater.pid))
        .order_by_asc(appraisal_nominations::Column::Id)
        .all(&ctx.db)
        .await?;
    let mut requests = Vec::new();
    for nomination in &nominations {
        let appraisal = appraisals::Entity::find()
            .filter(appraisals::Column::Pid.eq(nomination.appraisal_pid))
            .filter(appraisals::Column::DeletedAt.is_null())
            .filter(appraisals::Column::Status.eq("collecting"))
            .one(&ctx.db)
            .await?;
        let Some(appraisal) = appraisal else { continue };
        let responded = appraisal_responses::Entity::find()
            .filter(appraisal_responses::Column::NominationPid.eq(nomination.pid))
            .one(&ctx.db)
            .await?
            .is_some();
        if responded {
            continue;
        }
        let subject = employees::Entity::find()
            .filter(employees::Column::Pid.eq(appraisal.employee_pid))
            .one(&ctx.db)
            .await?;
        requests.push(serde_json::json!({
            "appraisal_pid": appraisal.pid,
            "subject_pid": appraisal.employee_pid,
            "subject": subject.map(|s| s.display_name),
            "group": nomination.rater_group,
            "competencies": appraisal.competencies,
        }));
    }
    format::json(requests)
}

/// Find one live appraisal by pid, or 404.
async fn find_appraisal(ctx: &AppContext, pid: &str) -> Result<appraisals::Model> {
    appraisals::Entity::find()
        .filter(appraisals::Column::Pid.eq(records::parse_pid(pid)?))
        .filter(appraisals::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)
}

/// The appraisal routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/employees/{pid}/appraisals", post(create_appraisal))
        .add("/employees/{pid}/appraisals", get(list_appraisals))
        .add("/employees/{pid}/appraisal-requests", get(rater_requests))
        .add("/appraisals/{pid}", get(get_appraisal))
        .add("/appraisals/{pid}/nominations", post(nominate))
        .add("/appraisals/{pid}/status", post(appraisal_status))
        .add("/appraisals/{pid}/responses", post(respond))
        .add("/appraisals/{pid}/report", get(report))
}
