//! Wellbeing — configurable health-entitlement rules (e.g. NHS
//! vaccination cohorts) and self-service prompts (WPM-R25).
//!
//! The rules are **configuration, not code** (cohorts change year to
//! year) and can predicate only on non-clinical facts — an age band
//! (birth date resolved best-effort via the upstream person client),
//! department, job title — per WPM-D17. WPM prompts and records the
//! employee's acknowledgement; it does not book appointments and
//! stores no vaccination status. Acknowledgements are employee-owned
//! (`$sub` ownership rules apply on the employee record); HR sees
//! **aggregate counts only** (WPM-D16 terms), and no manager view
//! exists at all.

use std::str::FromStr;

use loco_rs::prelude::*;
use sea_orm::{ActiveValue, QueryOrder};
use serde::Deserialize;
use uuid::Uuid;

use super::{ensure_valid, record_rejection};
use crate::auth::{self, MaybeAuthUser};
use crate::models::_entities::{entitlement_acknowledgements, wellbeing_entitlements};
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
use crate::rules::wellbeing as rules;
use crate::validation::Problems;

/// A `{pid}` reference response.
#[derive(serde::Serialize)]
struct PidRef {
    pid: String,
}

impl PidRef {
    fn of(pid: Uuid) -> Self {
        Self { pid: pid.to_string() }
    }
}

// ─── Entitlement rules (HR configuration) ───────────────────────────────────

/// `POST /api/wellbeing-entitlements` / `PUT …/{pid}` body. The only
/// predicates the shape can carry are non-clinical (WPM-D17): there is
/// deliberately no field a health-status cohort could be expressed in.
#[derive(Debug, Deserialize)]
struct EntitlementPayload {
    name: String,
    description: String,
    #[serde(default)]
    info_url: Option<String>,
    #[serde(default)]
    min_age: Option<i32>,
    #[serde(default)]
    max_age: Option<i32>,
    #[serde(default)]
    departments: Vec<String>,
    #[serde(default)]
    job_titles: Vec<String>,
    #[serde(default = "one_dose")]
    doses: i32,
    #[serde(default)]
    active_from: Option<chrono::NaiveDate>,
    #[serde(default)]
    active_until: Option<chrono::NaiveDate>,
}

fn one_dose() -> i32 {
    1
}

impl EntitlementPayload {
    fn validate(&self) -> Vec<String> {
        let mut problems = Problems::new();
        problems.require_text("name", &self.name);
        problems.require_text("description", &self.description);
        problems.cap_opt("info_url", self.info_url.as_deref());
        problems.cap_list("departments", &self.departments);
        problems.cap_list("job_titles", &self.job_titles);
        if !rules::valid_doses(self.doses) {
            problems.push(format!("doses must be between 1 and {}", rules::MAX_DOSES));
        }
        if !rules::valid_age_band(self.min_age, self.max_age) {
            problems.push(format!(
                "age band must satisfy 0 <= min_age <= max_age <= {}",
                rules::MAX_AGE
            ));
        }
        if let (Some(from), Some(until)) = (self.active_from, self.active_until)
            && from > until
        {
            problems.push("active_from must not be after active_until");
        }
        problems.into_vec()
    }
}

/// `POST /api/wellbeing-entitlements` — add a rule (HR configuration).
#[debug_handler]
async fn create_entitlement(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<EntitlementPayload>,
) -> Result<Response> {
    ensure_valid(&payload.validate())?;
    let row = wellbeing_entitlements::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        name: ActiveValue::set(payload.name.clone()),
        description: ActiveValue::set(payload.description.clone()),
        info_url: ActiveValue::set(payload.info_url.clone()),
        min_age: ActiveValue::set(payload.min_age),
        max_age: ActiveValue::set(payload.max_age),
        departments: ActiveValue::set(serde_json::json!(payload.departments)),
        job_titles: ActiveValue::set(serde_json::json!(payload.job_titles)),
        doses: ActiveValue::set(payload.doses),
        active_from: ActiveValue::set(payload.active_from),
        active_until: ActiveValue::set(payload.active_until),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await?;
    Audit::record(&ctx.db, "wellbeing_entitlement", row.pid, "created", caller.actor(), None)
        .await?;
    format::json(PidRef::of(row.pid))
}

/// `GET /api/wellbeing-entitlements` — the configured rules.
#[debug_handler]
async fn list_entitlements(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = wellbeing_entitlements::Entity::find()
        .filter(wellbeing_entitlements::Column::DeletedAt.is_null())
        .order_by_asc(wellbeing_entitlements::Column::Name)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

/// `PUT /api/wellbeing-entitlements/{pid}` — restate a rule (cohorts
/// change year to year; existing acknowledgements are untouched).
#[debug_handler]
async fn update_entitlement(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<EntitlementPayload>,
) -> Result<Response> {
    ensure_valid(&payload.validate())?;
    let row = find_entitlement(&ctx, &pid).await?;
    let row_pid = row.pid;
    let mut active: wellbeing_entitlements::ActiveModel = row.into();
    active.name = ActiveValue::set(payload.name.clone());
    active.description = ActiveValue::set(payload.description.clone());
    active.info_url = ActiveValue::set(payload.info_url.clone());
    active.min_age = ActiveValue::set(payload.min_age);
    active.max_age = ActiveValue::set(payload.max_age);
    active.departments = ActiveValue::set(serde_json::json!(payload.departments));
    active.job_titles = ActiveValue::set(serde_json::json!(payload.job_titles));
    active.doses = ActiveValue::set(payload.doses);
    active.active_from = ActiveValue::set(payload.active_from);
    active.active_until = ActiveValue::set(payload.active_until);
    let updated = active.update(&ctx.db).await?;
    Audit::record(&ctx.db, "wellbeing_entitlement", row_pid, "updated", caller.actor(), None)
        .await?;
    format::json(updated)
}

/// `DELETE /api/wellbeing-entitlements/{pid}` — soft-close a rule
/// (acknowledgement history is kept).
#[debug_handler]
async fn delete_entitlement(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let row = find_entitlement(&ctx, &pid).await?;
    let row_pid = row.pid;
    let mut active: wellbeing_entitlements::ActiveModel = row.into();
    active.deleted_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    active.update(&ctx.db).await?;
    Audit::record(&ctx.db, "wellbeing_entitlement", row_pid, "deleted", caller.actor(), None)
        .await?;
    format::json(serde_json::json!({ "deleted": row_pid }))
}

// ─── Self-service prompts ───────────────────────────────────────────────────

/// The department / job-title list off a stored rule's JSONB column.
fn string_list(value: &serde_json::Value) -> Vec<String> {
    serde_json::from_value(value.clone()).unwrap_or_default()
}

/// `GET /api/employees/{pid}/wellbeing-prompts` — the employee's live
/// prompts: every active rule they are eligible for and have not
/// acknowledged, plus at most **one** reminder per multi-dose course
/// they acknowledged `booked`/`done` (serving a reminder stamps it, so
/// it appears exactly once). Eligibility runs over age (resolved
/// best-effort from the person service — unknown age fails an
/// age-banded rule), department, and job title; the payload names that
/// derivation. Employee-owned: `$sub` ownership policies on the
/// employee record apply.
#[debug_handler]
async fn employee_prompts(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let employee = records::find_employee(&ctx.db, records::parse_pid(&pid)?).await?;
    auth::authorize_record(
        &caller,
        authentication_verifier::Action::Read,
        &auth::employee_resource_attrs(&employee),
    )
    .map_err(record_rejection)?;
    let today = chrono::Utc::now().date_naive();
    let age = match entity_ref::EntityRef::from_str(&employee.person_ref) {
        Ok(person) => crate::clients::birth_date(&person)
            .await
            .and_then(|born| rules::age_on(born, today)),
        Err(_) => None,
    };
    let entitlements = wellbeing_entitlements::Entity::find()
        .filter(wellbeing_entitlements::Column::DeletedAt.is_null())
        .order_by_asc(wellbeing_entitlements::Column::Name)
        .all(&ctx.db)
        .await?;
    let acks = entitlement_acknowledgements::Entity::find()
        .filter(entitlement_acknowledgements::Column::EmployeePid.eq(employee.pid))
        .all(&ctx.db)
        .await?;
    let mut prompts = Vec::new();
    for entitlement in &entitlements {
        let departments = string_list(&entitlement.departments);
        let job_titles = string_list(&entitlement.job_titles);
        let predicates = rules::Predicates {
            min_age: entitlement.min_age,
            max_age: entitlement.max_age,
            departments: &departments,
            job_titles: &job_titles,
            active_from: entitlement.active_from,
            active_until: entitlement.active_until,
        };
        if !rules::eligible(&predicates, age, &employee.department, &employee.job_title, today) {
            continue;
        }
        let ack = acks.iter().find(|a| a.entitlement_pid == entitlement.pid);
        let state = rules::prompt_state(
            entitlement.doses,
            ack.map(|a| a.response.as_str()),
            ack.is_some_and(|a| a.reminded_on.is_some()),
        );
        let kind = match state {
            rules::PromptState::Prompt => "prompt",
            rules::PromptState::Reminder => {
                // Serving the reminder is what "one reminder" means:
                // stamp it so it never appears again.
                let mut active: entitlement_acknowledgements::ActiveModel =
                    ack.expect("reminder implies an acknowledgement").clone().into();
                active.reminded_on = ActiveValue::set(Some(today));
                active.update(&ctx.db).await?;
                "reminder"
            }
            rules::PromptState::Quiet => continue,
        };
        prompts.push(serde_json::json!({
            "kind": kind,
            "entitlement_pid": entitlement.pid,
            "name": entitlement.name,
            "description": entitlement.description,
            "info_url": entitlement.info_url,
            "doses": entitlement.doses,
            "response": ack.map(|a| a.response.clone()),
        }));
    }
    format::json(serde_json::json!({
        "as_of": today,
        "age_known": age.is_some(),
        "derivation": "eligibility over age band (unknown age fails a banded rule), \
                       department, and job title only; informational — no response \
                       is required and declining has no recorded consequence",
        "prompts": prompts,
    }))
}

/// `POST /api/employees/{pid}/wellbeing-acknowledgements` body.
#[derive(Debug, Deserialize)]
struct AcknowledgePayload {
    entitlement_pid: Uuid,
    response: String,
}

/// `POST /api/employees/{pid}/wellbeing-acknowledgements` — record (or
/// restate) the employee's response to a prompt: one row per
/// employee + entitlement, upserted. The stored fact is the
/// acknowledgement of a prompt — never a vaccination status (WPM-D17).
/// Employee-owned (`$sub` ownership policies apply); audited.
#[debug_handler]
async fn acknowledge(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<AcknowledgePayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("response", rules::RESPONSES, &payload.response);
    ensure_valid(&problems.into_vec())?;
    let employee = records::find_employee(&ctx.db, records::parse_pid(&pid)?).await?;
    auth::authorize_record(
        &caller,
        authentication_verifier::Action::Write,
        &auth::employee_resource_attrs(&employee),
    )
    .map_err(record_rejection)?;
    let entitlement = wellbeing_entitlements::Entity::find()
        .filter(wellbeing_entitlements::Column::Pid.eq(payload.entitlement_pid))
        .filter(wellbeing_entitlements::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;
    let today = chrono::Utc::now().date_naive();
    let existing = entitlement_acknowledgements::Entity::find()
        .filter(entitlement_acknowledgements::Column::EntitlementPid.eq(entitlement.pid))
        .filter(entitlement_acknowledgements::Column::EmployeePid.eq(employee.pid))
        .one(&ctx.db)
        .await?;
    let row = match existing {
        Some(row) => {
            let mut active: entitlement_acknowledgements::ActiveModel = row.into();
            active.response = ActiveValue::set(payload.response.clone());
            active.responded_on = ActiveValue::set(today);
            active.update(&ctx.db).await?
        }
        None => {
            entitlement_acknowledgements::ActiveModel {
                pid: ActiveValue::set(Uuid::new_v4()),
                entitlement_pid: ActiveValue::set(entitlement.pid),
                employee_pid: ActiveValue::set(employee.pid),
                response: ActiveValue::set(payload.response.clone()),
                responded_on: ActiveValue::set(today),
                reminded_on: ActiveValue::set(None),
                ..Default::default()
            }
            .insert(&ctx.db)
            .await?
        }
    };
    Audit::record(
        &ctx.db,
        "entitlement_acknowledgement",
        row.pid,
        "acknowledged",
        caller.actor(),
        Some(serde_json::json!({ "response": payload.response })),
    )
    .await?;
    format::json(row)
}

// ─── HR aggregate view ──────────────────────────────────────────────────────

/// `GET /api/wellbeing/uptake` — HR's view is **aggregate counts
/// only** (WPM-R25): per entitlement, the acknowledgement counts by
/// response and the uptake rate with its terms (WPM-D16). No employee
/// appears in the payload, and there is deliberately no per-manager or
/// per-employee variant of this view.
#[debug_handler]
async fn uptake(State(ctx): State<AppContext>) -> Result<Response> {
    let entitlements = wellbeing_entitlements::Entity::find()
        .filter(wellbeing_entitlements::Column::DeletedAt.is_null())
        .order_by_asc(wellbeing_entitlements::Column::Name)
        .all(&ctx.db)
        .await?;
    let acks = entitlement_acknowledgements::Entity::find().all(&ctx.db).await?;
    let view: Vec<serde_json::Value> = entitlements
        .iter()
        .map(|entitlement| {
            let mut by_response: std::collections::BTreeMap<&str, usize> =
                rules::RESPONSES.iter().map(|r| (*r, 0)).collect();
            for ack in acks.iter().filter(|a| a.entitlement_pid == entitlement.pid) {
                if let Some(count) = by_response.get_mut(ack.response.as_str()) {
                    *count += 1;
                }
            }
            let uptaken = by_response["booked"] + by_response["done"];
            let responded: usize = by_response.values().sum();
            #[allow(clippy::cast_precision_loss)] // display ratio
            let value = if responded == 0 {
                serde_json::Value::Null
            } else {
                serde_json::json!(uptaken as f64 / responded as f64)
            };
            serde_json::json!({
                "entitlement_pid": entitlement.pid,
                "name": entitlement.name,
                "by_response": by_response,
                "uptake_rate": {
                    "numerator": uptaken, "denominator": responded, "value": value,
                },
            })
        })
        .collect();
    format::json(serde_json::json!({
        "as_of": chrono::Utc::now(),
        "derivation": "uptake = (booked + done) / all acknowledgements; counts only — \
                       responses are self-reported workflow facts, not clinical records, \
                       and no individual appears in this view",
        "entitlements": view,
    }))
}

/// Find one live entitlement rule by pid, or 404.
async fn find_entitlement(
    ctx: &AppContext,
    pid: &str,
) -> Result<wellbeing_entitlements::Model> {
    wellbeing_entitlements::Entity::find()
        .filter(wellbeing_entitlements::Column::Pid.eq(records::parse_pid(pid)?))
        .filter(wellbeing_entitlements::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)
}

/// The wellbeing routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/wellbeing-entitlements", post(create_entitlement))
        .add("/wellbeing-entitlements", get(list_entitlements))
        .add("/wellbeing-entitlements/{pid}", put(update_entitlement))
        .add("/wellbeing-entitlements/{pid}", delete(delete_entitlement))
        .add("/employees/{pid}/wellbeing-prompts", get(employee_prompts))
        .add("/employees/{pid}/wellbeing-acknowledgements", post(acknowledge))
        .add("/wellbeing/uptake", get(uptake))
}
