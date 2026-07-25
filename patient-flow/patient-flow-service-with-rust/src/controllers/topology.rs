//! Topology CRUD: sites, wards, bays, beds — plus the bed
//! state-transition endpoint (`POST /api/beds/{pid}/state`).
//!
//! Every mutation runs on one transaction: the row change, its audit
//! entry, and (under the `outbox` transport) its event share a commit
//! boundary (PF-D9).

use loco_rs::prelude::*;
use sea_orm::{QueryOrder, QuerySelect, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ensure_valid, unprocessable};
use crate::auth::MaybeAuthUser;
use crate::flow::bed_state::{self, BedContext, BedState, Transition};
use crate::flow::tokens;
use crate::metrics::Metrics;
use crate::models::_entities::{bays, beds, sites, wards};
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
use crate::streaming;
use crate::validation::Problems;

/// `POST /api/sites` body.
#[derive(Debug, Deserialize)]
struct SitePayload {
    name: String,
    #[serde(default)]
    place_ref: Option<String>,
    #[serde(default)]
    organization_ref: Option<String>,
}

/// `POST /api/wards` body.
#[derive(Debug, Deserialize)]
struct WardPayload {
    site_pid: Uuid,
    name: String,
    code: String,
    kind: String,
    #[serde(default)]
    specialty: Option<String>,
    #[serde(default = "default_true")]
    open: bool,
    #[serde(default)]
    escalation: bool,
    #[serde(default)]
    closed_to_admissions: bool,
    #[serde(default)]
    place_ref: Option<String>,
}

/// `POST /api/bays` body.
#[derive(Debug, Deserialize)]
struct BayPayload {
    ward_pid: Uuid,
    name: String,
    sex_designation: String,
    #[serde(default)]
    side_room: bool,
    #[serde(default)]
    closed_to_admissions: bool,
}

/// `POST /api/beds` body.
#[allow(clippy::struct_excessive_bools)] // bed attributes are independent flags
#[derive(Debug, Deserialize)]
struct BedPayload {
    bay_pid: Uuid,
    number: String,
    #[serde(default)]
    isolation_capable: bool,
    #[serde(default)]
    oxygen: bool,
    #[serde(default)]
    bariatric: bool,
    #[serde(default)]
    r#virtual: bool,
}

/// `POST /api/beds/{pid}/state` body: one state-machine transition.
#[derive(Debug, Deserialize)]
struct TransitionPayload {
    /// `allocate` | `release` | `clean_start` | `clean_complete` |
    /// `close` | `reopen`. (`admit`/`vacate` happen only through the
    /// stay endpoints.)
    transition: String,
    /// Closure reason (required for `close`).
    #[serde(default)]
    reason: Option<String>,
    /// Whether a completed clean was the confirmed deep clean.
    #[serde(default)]
    deep_clean_done: bool,
}

/// A `{pid}` reference response.
#[derive(Debug, Serialize)]
struct PidRef {
    pid: String,
}

const fn default_true() -> bool {
    true
}

fn validate_site(p: &SitePayload) -> Vec<String> {
    let mut problems = Problems::new();
    problems.require_text("name", &p.name);
    problems.ref_opt(
        "place_ref",
        entity_ref::EntityType::Place,
        p.place_ref.as_deref(),
    );
    problems.ref_opt(
        "organization_ref",
        entity_ref::EntityType::Organization,
        p.organization_ref.as_deref(),
    );
    problems.into_vec()
}

fn validate_ward(p: &WardPayload) -> Vec<String> {
    let mut problems = Problems::new();
    problems.require_text("name", &p.name);
    problems.require_text("code", &p.code);
    problems.require_token("kind", tokens::WARD_KINDS, &p.kind);
    problems.cap_opt("specialty", p.specialty.as_deref());
    problems.ref_opt(
        "place_ref",
        entity_ref::EntityType::Place,
        p.place_ref.as_deref(),
    );
    problems.into_vec()
}

fn validate_bay(p: &BayPayload) -> Vec<String> {
    let mut problems = Problems::new();
    problems.require_text("name", &p.name);
    problems.require_token(
        "sex_designation",
        tokens::SEX_DESIGNATIONS,
        &p.sex_designation,
    );
    problems.into_vec()
}

fn validate_bed(p: &BedPayload) -> Vec<String> {
    let mut problems = Problems::new();
    problems.require_text("number", &p.number);
    problems.into_vec()
}

/// `POST /api/sites`.
#[debug_handler]
async fn create_site(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<SitePayload>,
) -> Result<Response> {
    ensure_valid(&validate_site(&payload))?;
    let txn = ctx.db.begin().await?;
    let row = sites::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        name: ActiveValue::set(payload.name.clone()),
        place_ref: ActiveValue::set(payload.place_ref.clone()),
        organization_ref: ActiveValue::set(payload.organization_ref.clone()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "site", row.pid, "created", caller.actor(), None).await?;
    streaming::emit_on(
        &txn,
        "site",
        "created",
        &row.pid.to_string(),
        &row.name,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: row.pid.to_string(),
    })
}

/// `GET /api/sites` — active sites.
#[debug_handler]
async fn list_sites(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = sites::Entity::find()
        .filter(sites::Column::DeletedAt.is_null())
        .order_by_asc(sites::Column::Id)
        .limit(200)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

/// `POST /api/wards`.
#[debug_handler]
async fn create_ward(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<WardPayload>,
) -> Result<Response> {
    ensure_valid(&validate_ward(&payload))?;
    records::find_site(&ctx.db, payload.site_pid).await?;
    let txn = ctx.db.begin().await?;
    let row = wards::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        site_pid: ActiveValue::set(payload.site_pid),
        name: ActiveValue::set(payload.name.clone()),
        code: ActiveValue::set(payload.code.clone()),
        kind: ActiveValue::set(payload.kind.clone()),
        specialty: ActiveValue::set(payload.specialty.clone()),
        open: ActiveValue::set(payload.open),
        escalation: ActiveValue::set(payload.escalation),
        closed_to_admissions: ActiveValue::set(payload.closed_to_admissions),
        place_ref: ActiveValue::set(payload.place_ref.clone()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "ward", row.pid, "created", caller.actor(), None).await?;
    streaming::emit_on(
        &txn,
        "ward",
        "created",
        &row.pid.to_string(),
        &row.name,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: row.pid.to_string(),
    })
}

/// `GET /api/wards` — active wards.
#[debug_handler]
async fn list_wards(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = wards::Entity::find()
        .filter(wards::Column::DeletedAt.is_null())
        .order_by_asc(wards::Column::Id)
        .limit(500)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

/// `GET /api/wards/{pid}`.
#[debug_handler]
async fn get_ward(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let ward = records::find_ward(&ctx.db, records::parse_pid(&pid)?).await?;
    format::json(ward)
}

/// `PUT /api/wards/{pid}` — update the mutable ward fields (open /
/// escalation / closed-to-admissions / name / specialty).
#[derive(Debug, Deserialize)]
struct WardUpdate {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    specialty: Option<String>,
    #[serde(default)]
    open: Option<bool>,
    #[serde(default)]
    escalation: Option<bool>,
    #[serde(default)]
    closed_to_admissions: Option<bool>,
}

#[debug_handler]
async fn update_ward(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<WardUpdate>,
) -> Result<Response> {
    let mut problems = Problems::new();
    if let Some(name) = payload.name.as_deref() {
        problems.require_text("name", name);
    }
    problems.cap_opt("specialty", payload.specialty.as_deref());
    ensure_valid(&problems.into_vec())?;
    let ward = records::find_ward(&ctx.db, records::parse_pid(&pid)?).await?;
    let txn = ctx.db.begin().await?;
    let before = serde_json::to_value(&ward).ok();
    let mut active: wards::ActiveModel = ward.into();
    if let Some(name) = payload.name {
        active.name = ActiveValue::set(name);
    }
    if let Some(specialty) = payload.specialty {
        active.specialty = ActiveValue::set(Some(specialty));
    }
    if let Some(open) = payload.open {
        active.open = ActiveValue::set(open);
    }
    if let Some(escalation) = payload.escalation {
        active.escalation = ActiveValue::set(escalation);
    }
    if let Some(closed) = payload.closed_to_admissions {
        active.closed_to_admissions = ActiveValue::set(closed);
    }
    let row = active.update(&txn).await?;
    let snapshot = serde_json::json!({ "before": before, "ward_pid": row.pid.to_string() });
    Audit::record(
        &txn,
        "ward",
        row.pid,
        "updated",
        caller.actor(),
        Some(snapshot),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "ward",
        "updated",
        &row.pid.to_string(),
        &row.name,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `POST /api/bays`.
#[debug_handler]
async fn create_bay(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<BayPayload>,
) -> Result<Response> {
    ensure_valid(&validate_bay(&payload))?;
    records::find_ward(&ctx.db, payload.ward_pid).await?;
    let txn = ctx.db.begin().await?;
    let row = bays::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        ward_pid: ActiveValue::set(payload.ward_pid),
        name: ActiveValue::set(payload.name.clone()),
        sex_designation: ActiveValue::set(payload.sex_designation.clone()),
        side_room: ActiveValue::set(payload.side_room),
        closed_to_admissions: ActiveValue::set(payload.closed_to_admissions),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "bay", row.pid, "created", caller.actor(), None).await?;
    streaming::emit_on(
        &txn,
        "bay",
        "created",
        &row.pid.to_string(),
        &row.name,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: row.pid.to_string(),
    })
}

/// `PUT /api/bays/{pid}` — update closure / designation.
#[derive(Debug, Deserialize)]
struct BayUpdate {
    #[serde(default)]
    sex_designation: Option<String>,
    #[serde(default)]
    closed_to_admissions: Option<bool>,
}

#[debug_handler]
async fn update_bay(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<BayUpdate>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.token_opt(
        "sex_designation",
        tokens::SEX_DESIGNATIONS,
        payload.sex_designation.as_deref(),
    );
    ensure_valid(&problems.into_vec())?;
    let bay = records::find_bay(&ctx.db, records::parse_pid(&pid)?).await?;
    let txn = ctx.db.begin().await?;
    let mut active: bays::ActiveModel = bay.into();
    if let Some(sex) = payload.sex_designation {
        active.sex_designation = ActiveValue::set(sex);
    }
    if let Some(closed) = payload.closed_to_admissions {
        active.closed_to_admissions = ActiveValue::set(closed);
    }
    let row = active.update(&txn).await?;
    Audit::record(
        &txn,
        "bay",
        row.pid,
        "updated",
        caller.actor(),
        Some(serde_json::json!({ "ward_pid": row.ward_pid.to_string() })),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "bay",
        "updated",
        &row.pid.to_string(),
        &row.name,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `POST /api/beds` — a new bed starts `available`.
#[debug_handler]
async fn create_bed(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<BedPayload>,
) -> Result<Response> {
    ensure_valid(&validate_bed(&payload))?;
    let bay = records::find_bay(&ctx.db, payload.bay_pid).await?;
    // A virtual slot must live on a virtual ward; a physical bed must not.
    let ward = records::find_ward(&ctx.db, bay.ward_pid).await?;
    if payload.r#virtual != (ward.kind == "virtual") {
        return Err(unprocessable(
            "virtual slots live on virtual wards; physical beds on physical wards",
        ));
    }
    let txn = ctx.db.begin().await?;
    let row = beds::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        bay_pid: ActiveValue::set(payload.bay_pid),
        number: ActiveValue::set(payload.number.clone()),
        state: ActiveValue::set(BedState::Available.token().to_string()),
        state_since: ActiveValue::set(chrono::Utc::now().into()),
        closure_reason: ActiveValue::set(None),
        deep_clean_required: ActiveValue::set(false),
        isolation_capable: ActiveValue::set(payload.isolation_capable),
        oxygen: ActiveValue::set(payload.oxygen),
        bariatric: ActiveValue::set(payload.bariatric),
        is_virtual: ActiveValue::set(payload.r#virtual),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "bed", row.pid, "created", caller.actor(), None).await?;
    streaming::emit_on(
        &txn,
        "bed",
        "created",
        &row.pid.to_string(),
        &row.number,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: row.pid.to_string(),
    })
}

/// `GET /api/beds/{pid}`.
#[debug_handler]
async fn get_bed(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let bed = records::find_bed(&ctx.db, records::parse_pid(&pid)?).await?;
    format::json(bed)
}

/// `POST /api/beds/{pid}/state` — apply one bed state-machine
/// transition (spec `bed-management.md`). `admit` and `vacate` are
/// driven by the stay endpoints, not here.
#[debug_handler]
async fn bed_state(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<TransitionPayload>,
) -> Result<Response> {
    let transition = match payload.transition.as_str() {
        "allocate" => Transition::Allocate,
        "release" => Transition::Release,
        "clean_start" => Transition::CleanStart,
        "clean_complete" => Transition::CleanComplete {
            deep_clean_done: payload.deep_clean_done,
        },
        "close" => Transition::Close {
            reason: payload.reason.clone().unwrap_or_default(),
        },
        "reopen" => Transition::Reopen,
        "admit" | "vacate" => {
            return Err(unprocessable(
                "admit/vacate are driven by the stay endpoints",
            ));
        }
        other => return Err(unprocessable(&format!("unknown transition {other:?}"))),
    };
    let txn = ctx.db.begin().await?;
    // Lock the bed row for the read-check-write (PF-D9).
    let bed = beds::Entity::find()
        .filter(beds::Column::Pid.eq(records::parse_pid(&pid)?))
        .filter(beds::Column::DeletedAt.is_null())
        .lock_exclusive()
        .one(&txn)
        .await?
        .ok_or_else(|| Error::NotFound)?;
    let outcome = apply_transition(&bed, &transition)?;
    let row = persist_outcome(&txn, bed, &outcome, caller.actor()).await?;
    txn.commit().await?;
    Metrics::global().bed_state_changed_total.inc();
    format::json(row)
}

/// Run the pure state machine over a bed row.
pub(crate) fn apply_transition(
    bed: &beds::Model,
    transition: &Transition,
) -> Result<bed_state::Outcome> {
    let state = BedState::parse(&bed.state).map_err(|e| unprocessable(&e.to_string()))?;
    let ctx = BedContext {
        is_virtual: bed.is_virtual,
        deep_clean_required: bed.deep_clean_required,
    };
    bed_state::apply(state, transition, ctx).map_err(|e| unprocessable(&e.to_string()))
}

/// Persist a transition outcome: update the bed row, audit (with
/// old/new state), and emit `bed_state_changed` — all on `txn`.
pub(crate) async fn persist_outcome(
    txn: &sea_orm::DatabaseTransaction,
    bed: beds::Model,
    outcome: &bed_state::Outcome,
    actor: Option<&str>,
) -> Result<beds::Model> {
    let old_state = bed.state.clone();
    let pid = bed.pid;
    let number = bed.number.clone();
    let mut active: beds::ActiveModel = bed.into();
    active.state = ActiveValue::set(outcome.state.token().to_string());
    active.state_since = ActiveValue::set(chrono::Utc::now().into());
    active.deep_clean_required = ActiveValue::set(outcome.deep_clean_required);
    active.closure_reason = ActiveValue::set(outcome.closure_reason.clone());
    let row = active.update(txn).await?;
    let detail = serde_json::json!({
        "from": old_state,
        "to": outcome.state.token(),
        "deep_clean_required": outcome.deep_clean_required,
        "closure_reason": outcome.closure_reason,
    });
    Audit::record(
        txn,
        "bed",
        pid,
        "bed_state_changed",
        actor,
        Some(detail.clone()),
    )
    .await?;
    streaming::emit_on(
        txn,
        "bed",
        "bed_state_changed",
        &pid.to_string(),
        &number,
        actor,
        Some(detail),
    )
    .await?;
    Ok(row)
}

/// Soft-delete helper shared by the topology DELETE handlers.
macro_rules! soft_delete_handler {
    ($fn_name:ident, $module:ident, $finder:ident, $entity:literal) => {
        #[debug_handler]
        async fn $fn_name(
            State(ctx): State<AppContext>,
            caller: MaybeAuthUser,
            Path(pid): Path<String>,
        ) -> Result<Response> {
            let row = records::$finder(&ctx.db, records::parse_pid(&pid)?).await?;
            let txn = ctx.db.begin().await?;
            let pid = row.pid;
            let mut active: $module::ActiveModel = row.into();
            active.deleted_at = ActiveValue::set(Some(chrono::Utc::now().into()));
            active.update(&txn).await?;
            Audit::record(&txn, $entity, pid, "deleted", caller.actor(), None).await?;
            streaming::emit_on(
                &txn,
                $entity,
                "deleted",
                &pid.to_string(),
                "",
                caller.actor(),
                None,
            )
            .await?;
            txn.commit().await?;
            format::empty_json()
        }
    };
}

soft_delete_handler!(delete_site, sites, find_site, "site");
soft_delete_handler!(delete_ward, wards, find_ward, "ward");
soft_delete_handler!(delete_bay, bays, find_bay, "bay");
soft_delete_handler!(delete_bed, beds, find_bed, "bed");

/// The topology routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/sites", post(create_site))
        .add("/sites", get(list_sites))
        .add("/sites/{pid}", delete(delete_site))
        .add("/wards", post(create_ward))
        .add("/wards", get(list_wards))
        .add("/wards/{pid}", get(get_ward))
        .add("/wards/{pid}", put(update_ward))
        .add("/wards/{pid}", delete(delete_ward))
        .add("/bays", post(create_bay))
        .add("/bays/{pid}", put(update_bay))
        .add("/bays/{pid}", delete(delete_bay))
        .add("/beds", post(create_bed))
        .add("/beds/{pid}", get(get_bed))
        .add("/beds/{pid}", delete(delete_bed))
        .add("/beds/{pid}/state", post(bed_state))
}
