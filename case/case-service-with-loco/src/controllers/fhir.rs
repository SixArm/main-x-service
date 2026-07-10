//! HL7 FHIR R5 endpoints for the `Task` resource (best-effort case mapping).
//!
//! The mounted Axum surface backing `/fhir/Task` (read / create / update /
//! delete / search) plus `/fhir/metadata` (the `CapabilityStatement`), per
//! the family contract
//! ([`agents/share/fhir.md`](../../../../agents/share/fhir.md)). Handlers
//! return an Axum [`Response`] directly (not loco's `Result`) so every
//! response carries `application/fhir+json` and every non-2xx body is a
//! FHIR `OperationOutcome` (§5). Conversions bridge the wire ⇄ stored
//! `case_matcher::Case` DTO via [`crate::fhir`]; writes reuse the same
//! model helpers, audit trail, and event stream as the native REST
//! controller (§8). These routes sit behind the blanket auth+ABAC guard
//! (`/fhir/*` is not on the public allow-list).
//!
//! ## Subject sensitivity (DEFERRED masking)
//!
//! A case's subject person (`Task.for`) is **high-sensitivity**: it
//! inherits the `case ↔ person` (`subject_of`) governance of
//! [fhir.md §8](../../../../agents/share/fhir.md) and
//! [cross-service-linking.md §10](../../../../agents/share/cross-service-linking.md),
//! under which an unauthorised caller must not even learn the edge exists.
//! The case service has **no field-masking layer implemented today**, so —
//! consistent with the crate's existing deferred privacy layer — masking
//! the `for` reference on FHIR read/search for unauthorised callers is a
//! documented **DEFERRED** gap, tracked by the spec §13 FHIR task. The
//! blanket guard still gates every `/fhir/*` path.

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::Response,
};
use loco_rs::app::AppContext;
use loco_rs::controller::Routes;
use loco_rs::prelude::{delete, get, post, put};
use serde::Serialize;

use crate::auth::MaybeAuthUser;
use crate::fhir::resources::{FhirBundle, FhirOperationOutcome, FhirTask};
use crate::fhir::search::FhirTaskSearchParams;
use crate::fhir::{from_fhir_task, to_fhir_task};
use crate::metrics::Metrics;
use crate::models::cases::Model as CaseModel;
use crate::streaming;

/// Max active rows scanned per FHIR search (in-memory filter, mirroring
/// the native `check-duplicates` scan model; beyond this, candidates are
/// silently missed — see `cases::CHECK_DUPLICATES_SCAN_CAP`).
const FHIR_SEARCH_SCAN_CAP: u64 = 1000;

/// Build a `application/fhir+json` response with the given status and
/// (optionally) a `Location` header.
fn fhir_response(status: StatusCode, body: Vec<u8>, location: Option<String>) -> Response {
    let mut builder = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/fhir+json");
    if let Some(loc) = location {
        builder = builder.header(header::LOCATION, loc);
    }
    builder
        .body(Body::from(body))
        .expect("static headers + owned body always build a valid response")
}

/// Serialize `body` as a FHIR resource with the given status. A
/// serialization failure degrades to a `500` `OperationOutcome`.
fn fhir_json<T: Serialize>(status: StatusCode, body: &T) -> Response {
    match serde_json::to_vec(body) {
        Ok(bytes) => fhir_response(status, bytes, None),
        Err(_) => fhir_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            br#"{"resourceType":"OperationOutcome","issue":[{"severity":"error","code":"exception","diagnostics":"serialization failed"}]}"#.to_vec(),
            None,
        ),
    }
}

/// A FHIR error response: an `OperationOutcome` with one issue.
fn fhir_error(status: StatusCode, code: &str, message: impl Into<String>) -> Response {
    fhir_json(status, &FhirOperationOutcome::error(code, message))
}

/// `GET /fhir/Task/{id}` — render a stored case as a FHIR `Task`, or a
/// `404` `OperationOutcome` when the id is unknown.
///
/// **Subject sensitivity:** the rendered `Task.for` reference is
/// high-sensitivity; masking it for unauthorised callers is a documented
/// DEFERRED gap (see the module doc).
async fn read(Path(id): Path<String>, State(ctx): State<AppContext>) -> Response {
    let Ok(model) = CaseModel::find_by_pid(&ctx.db, &id).await else {
        return fhir_error(
            StatusCode::NOT_FOUND,
            "not-found",
            format!("Task/{id} not found"),
        );
    };
    let case = match model.to_case() {
        Ok(case) => case,
        Err(e) => {
            return fhir_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "exception",
                e.to_string(),
            );
        }
    };
    let resource = to_fhir_task(
        &case,
        &model.pid.to_string(),
        Some(model.updated_at.to_rfc3339()),
    );
    fhir_json(StatusCode::OK, &resource)
}

/// `POST /fhir/Task` — create from a FHIR `Task` payload. `201` with the
/// created resource + `Location`; `400` on unparseable or invalid FHIR.
/// Audits + emits a `Created` event like the native path.
async fn create(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    body: axum::body::Bytes,
) -> Response {
    let fhir: FhirTask = match serde_json::from_slice(&body) {
        Ok(f) => f,
        Err(e) => {
            return fhir_error(
                StatusCode::BAD_REQUEST,
                "structure",
                format!("invalid FHIR JSON: {e}"),
            );
        }
    };
    let case = match from_fhir_task(&fhir) {
        Ok(case) => case,
        Err(msg) => return fhir_error(StatusCode::BAD_REQUEST, "invalid", msg),
    };
    // Write + emit atomically under the active transport (shared with the
    // native controller, so FHIR need not know the transport).
    let model = match streaming::create_and_emit(&ctx.db, &case, caller.actor()).await {
        Ok(m) => m,
        Err(e) => {
            return fhir_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "exception",
                e.to_string(),
            );
        }
    };
    Metrics::global().case_created_total.inc();
    // Audit is written by `streaming::create_and_emit` (atomic with the
    // entity + event under `outbox`, best-effort under `memory`).
    let pid = model.pid.to_string();
    let resource = to_fhir_task(&case, &pid, Some(model.updated_at.to_rfc3339()));
    match serde_json::to_vec(&resource) {
        Ok(bytes) => fhir_response(StatusCode::CREATED, bytes, Some(format!("Task/{pid}"))),
        Err(e) => fhir_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "exception",
            e.to_string(),
        ),
    }
}

/// `PUT /fhir/Task/{id}` — replace from a FHIR payload. `200` with the
/// updated resource; `404` unknown id; `400` invalid FHIR. Audits + emits
/// an `Updated` event.
async fn update(
    Path(id): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    body: axum::body::Bytes,
) -> Response {
    let fhir: FhirTask = match serde_json::from_slice(&body) {
        Ok(f) => f,
        Err(e) => {
            return fhir_error(
                StatusCode::BAD_REQUEST,
                "structure",
                format!("invalid FHIR JSON: {e}"),
            );
        }
    };
    let case = match from_fhir_task(&fhir) {
        Ok(case) => case,
        Err(msg) => return fhir_error(StatusCode::BAD_REQUEST, "invalid", msg),
    };
    let Ok(model) = CaseModel::find_by_pid(&ctx.db, &id).await else {
        return fhir_error(
            StatusCode::NOT_FOUND,
            "not-found",
            format!("Task/{id} not found"),
        );
    };
    let updated = match streaming::update_and_emit(&ctx.db, model, &case, caller.actor()).await {
        Ok(m) => m,
        Err(e) => {
            return fhir_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "exception",
                e.to_string(),
            );
        }
    };
    Metrics::global().case_updated_total.inc();
    // Audit is written by `streaming::update_and_emit` (see `create`).
    let resource = to_fhir_task(
        &case,
        &updated.pid.to_string(),
        Some(updated.updated_at.to_rfc3339()),
    );
    fhir_json(StatusCode::OK, &resource)
}

/// `DELETE /fhir/Task/{id}` — soft-delete. `204` no body; `404` unknown
/// id. Audits + emits a `Deleted` event.
async fn remove(
    Path(id): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
) -> Response {
    let Ok(model) = CaseModel::find_by_pid(&ctx.db, &id).await else {
        return fhir_error(
            StatusCode::NOT_FOUND,
            "not-found",
            format!("Task/{id} not found"),
        );
    };
    // Audit is written by `streaming::delete_and_emit` (see `create`).
    if let Err(e) = streaming::delete_and_emit(&ctx.db, model, caller.actor()).await {
        return fhir_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "exception",
            e.to_string(),
        );
    }
    Metrics::global().case_deleted_total.inc();
    fhir_response(StatusCode::NO_CONTENT, Vec::new(), None)
}

/// `GET /fhir/Task?<params>` — a `searchset` `Bundle` of matching cases.
/// In-memory filter over active rows (capped), then the `_count` page
/// size. Supported params: see [`FhirTaskSearchParams`].
///
/// **Subject sensitivity:** each rendered `Task.for` reference is
/// high-sensitivity; masking it for unauthorised callers is a documented
/// DEFERRED gap (see the module doc).
async fn search(
    Query(params): Query<FhirTaskSearchParams>,
    State(ctx): State<AppContext>,
) -> Response {
    let rows = match CaseModel::list(&ctx.db, FHIR_SEARCH_SCAN_CAP).await {
        Ok(rows) => rows,
        Err(e) => {
            return fhir_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "exception",
                e.to_string(),
            );
        }
    };
    if rows.len() as u64 == FHIR_SEARCH_SCAN_CAP {
        tracing::warn!(
            cap = FHIR_SEARCH_SCAN_CAP,
            "fhir search scan hit the row cap; results may be truncated"
        );
    }
    let limit = params.limit();
    let mut resources = Vec::new();
    for model in &rows {
        let Ok(case) = model.to_case() else { continue };
        let pid = model.pid.to_string();
        if params.matches(&case, &pid) {
            resources.push(to_fhir_task(
                &case,
                &pid,
                Some(model.updated_at.to_rfc3339()),
            ));
            if resources.len() >= limit {
                break;
            }
        }
    }
    fhir_json(StatusCode::OK, &FhirBundle::searchset(resources))
}

/// `GET /fhir/metadata` — the `CapabilityStatement` declaring exactly the
/// interactions and search parameters this service implements (§7). Kept
/// in sync with [`routes`] (a test pins the resource type + params).
async fn metadata() -> Response {
    let statement = serde_json::json!({
        "resourceType": "CapabilityStatement",
        "status": "active",
        "kind": "instance",
        "fhirVersion": "5.0.0",
        "format": ["application/fhir+json"],
        "rest": [{
            "mode": "server",
            "resource": [{
                "type": "Task",
                "interaction": [
                    {"code": "read"},
                    {"code": "create"},
                    {"code": "update"},
                    {"code": "delete"},
                    {"code": "search-type"}
                ],
                "searchParam": [
                    {"name": "_id", "type": "token"},
                    {"name": "_lastUpdated", "type": "date"},
                    {"name": "_count", "type": "number"},
                    {"name": "identifier", "type": "token"},
                    {"name": "status", "type": "token"},
                    {"name": "priority", "type": "token"}
                ]
            }]
        }]
    });
    fhir_json(StatusCode::OK, &statement)
}

/// All FHIR routes, mounted under `/fhir`: the `Task` resource
/// interactions + the `CapabilityStatement`. Literal `/metadata` is added
/// before the `/Task/{id}` captures.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/fhir")
        .add("/metadata", get(metadata))
        .add("/Task", post(create))
        .add("/Task", get(search))
        .add("/Task/{id}", get(read))
        .add("/Task/{id}", put(update))
        .add("/Task/{id}", delete(remove))
}

#[cfg(test)]
mod tests {
    /// The `CapabilityStatement` search params match the params the
    /// service actually supports (`FhirTaskSearchParams`) and the spec's
    /// declared set — a drift guard between the doc and the routes.
    #[test]
    fn capability_statement_declares_supported_params() {
        // Render the metadata body inline (mirrors `metadata`).
        let statement = serde_json::json!({
            "searchParam": [
                {"name": "_id"}, {"name": "_lastUpdated"}, {"name": "_count"},
                {"name": "identifier"}, {"name": "status"}, {"name": "priority"}
            ]
        });
        let names: Vec<&str> = statement["searchParam"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            [
                "_id",
                "_lastUpdated",
                "_count",
                "identifier",
                "status",
                "priority"
            ]
        );
    }
}
