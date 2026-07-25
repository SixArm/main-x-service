//! Ergonomic (DSE) workstation assessments (WPM-R32 / WPM-D24) — a
//! checklist per employee + workstation, answered item by item,
//! completed only when every item is answered; open issues surface in
//! a rota-tier department report. About the workstation, never the
//! body: no symptom field exists anywhere on this surface.

use loco_rs::prelude::*;
use sea_orm::{ActiveValue, QueryOrder};
use serde::Deserialize;
use uuid::Uuid;

use super::{ensure_valid, unprocessable};
use crate::auth::MaybeAuthUser;
use crate::models::_entities::{employees, ergonomic_assessments, ergonomic_items};
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
use crate::rules::ergonomics as rules;
use crate::validation::Problems;

/// A `{pid}` reference response.
#[derive(serde::Serialize)]
struct PidRef {
    pid: String,
}

/// `POST /api/employees/{pid}/ergonomic-assessments` body. Omitted
/// `items` ⇒ the default DSE checklist.
#[derive(Debug, Deserialize)]
struct AssessmentPayload {
    workstation: String,
    #[serde(default)]
    items: Vec<String>,
}

/// `POST /api/employees/{pid}/ergonomic-assessments` — open an
/// assessment with its checklist (default DSE items when none given).
#[debug_handler]
async fn create_assessment(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<AssessmentPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("workstation", &payload.workstation);
    problems.cap_text("workstation", &payload.workstation);
    problems.cap_list("items", &payload.items);
    ensure_valid(&problems.into_vec())?;
    let employee = records::find_employee(&ctx.db, records::parse_pid(&pid)?).await?;
    let items: Vec<String> = if payload.items.is_empty() {
        rules::DSE_ITEMS.iter().map(ToString::to_string).collect()
    } else {
        payload.items.clone()
    };
    let txn = ctx.db.begin().await?;
    let assessment = ergonomic_assessments::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        employee_pid: ActiveValue::set(employee.pid),
        workstation: ActiveValue::set(payload.workstation.clone()),
        status: ActiveValue::set("open".to_string()),
        assessed_on: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    for name in &items {
        ergonomic_items::ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            assessment_pid: ActiveValue::set(assessment.pid),
            name: ActiveValue::set(name.clone()),
            ok: ActiveValue::set(None),
            note: ActiveValue::set(None),
            deleted_at: ActiveValue::set(None),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
    }
    Audit::record(
        &txn,
        "ergonomic_assessment",
        assessment.pid,
        "created",
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: assessment.pid.to_string(),
    })
}

/// `GET /api/employees/{pid}/ergonomic-assessments` — the employee's
/// assessments with their items.
#[debug_handler]
async fn list_assessments(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
) -> Result<Response> {
    let employee = records::find_employee(&ctx.db, records::parse_pid(&pid)?).await?;
    let assessments = ergonomic_assessments::Entity::find()
        .filter(ergonomic_assessments::Column::EmployeePid.eq(employee.pid))
        .filter(ergonomic_assessments::Column::DeletedAt.is_null())
        .order_by_desc(ergonomic_assessments::Column::Id)
        .all(&ctx.db)
        .await?;
    let mut view = Vec::new();
    for assessment in &assessments {
        let items = ergonomic_items::Entity::find()
            .filter(ergonomic_items::Column::AssessmentPid.eq(assessment.pid))
            .filter(ergonomic_items::Column::DeletedAt.is_null())
            .order_by_asc(ergonomic_items::Column::Id)
            .all(&ctx.db)
            .await?;
        view.push(serde_json::json!({
            "pid": assessment.pid,
            "workstation": assessment.workstation,
            "status": assessment.status,
            "assessed_on": assessment.assessed_on,
            "open_issues": rules::open_issues(
                &items.iter().map(|i| i.ok).collect::<Vec<_>>()
            ),
            "items": items,
        }));
    }
    format::json(view)
}

/// `PUT /api/ergonomic-items/{pid}` body — `ok` / `issue` + an
/// **equipment** note.
#[derive(Debug, Deserialize)]
struct ItemAnswer {
    ok: bool,
    #[serde(default)]
    note: Option<String>,
}

/// `PUT /api/ergonomic-items/{pid}` — answer one checklist item
/// (only while the assessment is open).
#[debug_handler]
async fn answer_item(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<ItemAnswer>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.cap_opt("note", payload.note.as_deref());
    ensure_valid(&problems.into_vec())?;
    let item = ergonomic_items::Entity::find()
        .filter(ergonomic_items::Column::Pid.eq(records::parse_pid(&pid)?))
        .filter(ergonomic_items::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;
    let assessment = ergonomic_assessments::Entity::find()
        .filter(ergonomic_assessments::Column::Pid.eq(item.assessment_pid))
        .filter(ergonomic_assessments::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;
    if assessment.status != "open" {
        return Err(unprocessable(
            "a completed assessment is a record; open a new one",
        ));
    }
    let item_pid = item.pid;
    let mut active: ergonomic_items::ActiveModel = item.into();
    active.ok = ActiveValue::set(Some(payload.ok));
    active.note = ActiveValue::set(payload.note.clone());
    let updated = active.update(&ctx.db).await?;
    Audit::record(
        &ctx.db,
        "ergonomic_item",
        item_pid,
        "answered",
        caller.actor(),
        Some(serde_json::json!({ "ok": payload.ok })),
    )
    .await?;
    format::json(updated)
}

/// `POST /api/ergonomic-assessments/{pid}/complete` — completing
/// requires every item answered; stamps the date.
#[debug_handler]
async fn complete_assessment(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let assessment = ergonomic_assessments::Entity::find()
        .filter(ergonomic_assessments::Column::Pid.eq(records::parse_pid(&pid)?))
        .filter(ergonomic_assessments::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;
    if assessment.status != "open" {
        return Err(unprocessable("already completed"));
    }
    let items = ergonomic_items::Entity::find()
        .filter(ergonomic_items::Column::AssessmentPid.eq(assessment.pid))
        .filter(ergonomic_items::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    let answers: Vec<Option<bool>> = items.iter().map(|i| i.ok).collect();
    rules::may_complete(&answers).map_err(|reason| unprocessable(&reason))?;
    let issues = rules::open_issues(&answers);
    let assessment_pid = assessment.pid;
    let mut active: ergonomic_assessments::ActiveModel = assessment.into();
    active.status = ActiveValue::set("completed".to_string());
    active.assessed_on = ActiveValue::set(Some(chrono::Utc::now().date_naive()));
    let updated = active.update(&ctx.db).await?;
    Audit::record(
        &ctx.db,
        "ergonomic_assessment",
        assessment_pid,
        "completed",
        caller.actor(),
        Some(serde_json::json!({ "open_issues": issues })),
    )
    .await?;
    format::json(updated)
}

/// `GET /api/ergonomics/issues` — every `issue`-flagged item on a
/// live assessment, grouped by department (rota-tier visibility,
/// WPM-R27 precedent): employee, workstation, item, note; plus
/// per-department counts.
#[debug_handler]
async fn issues(State(ctx): State<AppContext>) -> Result<Response> {
    let employee_rows = employees::Entity::find()
        .filter(employees::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    let assessments = ergonomic_assessments::Entity::find()
        .filter(ergonomic_assessments::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    let items = ergonomic_items::Entity::find()
        .filter(ergonomic_items::Column::DeletedAt.is_null())
        .filter(ergonomic_items::Column::Ok.eq(false))
        .all(&ctx.db)
        .await?;
    let mut listed = Vec::new();
    let mut by_department: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for item in &items {
        let Some(assessment) = assessments.iter().find(|a| a.pid == item.assessment_pid) else {
            continue;
        };
        let Some(employee) = employee_rows
            .iter()
            .find(|e| e.pid == assessment.employee_pid)
        else {
            continue;
        };
        *by_department
            .entry(employee.department.clone())
            .or_default() += 1;
        listed.push(serde_json::json!({
            "department": employee.department,
            "employee_pid": employee.pid,
            "display_name": employee.display_name,
            "workstation": assessment.workstation,
            "item": item.name,
            "note": item.note,
            "assessment_status": assessment.status,
        }));
    }
    format::json(serde_json::json!({
        "as_of": chrono::Utc::now(),
        "by_department": by_department,
        "issues": listed,
        "derivation": "issue-flagged checklist items on live assessments; equipment and \
                       environment facts only — no symptom field exists (WPM-D24); \
                       visibility is rota-tier",
    }))
}

/// The ergonomics routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add(
            "/employees/{pid}/ergonomic-assessments",
            post(create_assessment),
        )
        .add(
            "/employees/{pid}/ergonomic-assessments",
            get(list_assessments),
        )
        .add("/ergonomic-items/{pid}", put(answer_item))
        .add(
            "/ergonomic-assessments/{pid}/complete",
            post(complete_assessment),
        )
        .add("/ergonomics/issues", get(issues))
}
