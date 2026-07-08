//! HL7 FHIR R5 endpoints for the `Organization` resource.
//!
//! The mounted Axum surface backing `/fhir/Organization` (read / create /
//! update / delete / search) plus `/fhir/metadata` (the
//! `CapabilityStatement`), per the family contract
//! ([`agents/share/fhir.md`](../../../../agents/share/fhir.md)). Handlers
//! return an Axum [`Response`] directly (not loco's `Result`) so every
//! response carries `application/fhir+json` and every non-2xx body is a
//! FHIR `OperationOutcome` (§5). Conversions bridge the wire ⇄ stored
//! `organization_matcher::Organization` DTO via [`crate::fhir`]; writes
//! reuse the same model helpers, audit trail, and event stream as the
//! native REST controller (§8). These routes sit behind the blanket
//! auth+ABAC guard (`/fhir/*` is not on the public allow-list).

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
use crate::fhir::resources::{FhirBundle, FhirOperationOutcome, FhirOrganization};
use crate::fhir::search::FhirOrgSearchParams;
use crate::fhir::{from_fhir_organization, to_fhir_organization};
use crate::metrics::Metrics;
use crate::models::audit_logs::Model as AuditModel;
use crate::models::organizations::Model as OrgModel;
use crate::streaming;

/// Max active rows scanned per FHIR search (in-memory filter, mirroring
/// the native `check-duplicates` scan model; beyond this, candidates are
/// silently missed — see `organizations::CHECK_DUPLICATES_SCAN_CAP`).
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

/// Best-effort audit write (never fails the request), mirroring the native
/// controller's audit path.
async fn audit(
    ctx: &AppContext,
    entity_pid: uuid::Uuid,
    action: &str,
    actor: Option<&str>,
    snapshot: Option<serde_json::Value>,
) {
    if let Err(err) = AuditModel::record(&ctx.db, entity_pid, action, actor, snapshot).await {
        tracing::warn!(error = %err, action, "failed to write audit log (fhir)");
    }
}

/// `GET /fhir/Organization/{id}` — render a stored organization as a FHIR
/// `Organization`, or a `404` `OperationOutcome` when the id is unknown.
async fn read(Path(id): Path<String>, State(ctx): State<AppContext>) -> Response {
    let Ok(model) = OrgModel::find_by_pid(&ctx.db, &id).await else {
        return fhir_error(StatusCode::NOT_FOUND, "not-found", format!("Organization/{id} not found"));
    };
    let org = match model.to_org() {
        Ok(org) => org,
        Err(e) => return fhir_error(StatusCode::INTERNAL_SERVER_ERROR, "exception", e.to_string()),
    };
    let resource = to_fhir_organization(
        &org,
        &model.pid.to_string(),
        model.active,
        Some(model.updated_at.to_rfc3339()),
    );
    fhir_json(StatusCode::OK, &resource)
}

/// `POST /fhir/Organization` — create from a FHIR `Organization` payload.
/// `201` with the created resource + `Location`; `400` on unparseable or
/// invalid FHIR. Audits + emits a `Created` event like the native path.
async fn create(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    body: axum::body::Bytes,
) -> Response {
    let fhir: FhirOrganization = match serde_json::from_slice(&body) {
        Ok(f) => f,
        Err(e) => return fhir_error(StatusCode::BAD_REQUEST, "structure", format!("invalid FHIR JSON: {e}")),
    };
    let org = match from_fhir_organization(&fhir) {
        Ok(org) => org,
        Err(msg) => return fhir_error(StatusCode::BAD_REQUEST, "invalid", msg),
    };
    // Write + `Created` event, atomic under the active transport (shares
    // the native controller's transport-aware path).
    let model = match streaming::create_and_emit(&ctx.db, &org, caller.actor()).await {
        Ok(m) => m,
        Err(e) => return fhir_error(StatusCode::INTERNAL_SERVER_ERROR, "exception", e.to_string()),
    };
    Metrics::global().organization_created_total.inc();
    audit(&ctx, model.pid, "created", caller.actor(), Some(model.data.clone())).await;
    let pid = model.pid.to_string();
    let resource = to_fhir_organization(&org, &pid, true, Some(model.updated_at.to_rfc3339()));
    match serde_json::to_vec(&resource) {
        Ok(bytes) => fhir_response(StatusCode::CREATED, bytes, Some(format!("Organization/{pid}"))),
        Err(e) => fhir_error(StatusCode::INTERNAL_SERVER_ERROR, "exception", e.to_string()),
    }
}

/// `PUT /fhir/Organization/{id}` — replace from a FHIR payload. `200` with
/// the updated resource; `404` unknown id; `400` invalid FHIR. Audits +
/// emits an `Updated` event.
async fn update(
    Path(id): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    body: axum::body::Bytes,
) -> Response {
    let fhir: FhirOrganization = match serde_json::from_slice(&body) {
        Ok(f) => f,
        Err(e) => return fhir_error(StatusCode::BAD_REQUEST, "structure", format!("invalid FHIR JSON: {e}")),
    };
    let org = match from_fhir_organization(&fhir) {
        Ok(org) => org,
        Err(msg) => return fhir_error(StatusCode::BAD_REQUEST, "invalid", msg),
    };
    let Ok(model) = OrgModel::find_by_pid(&ctx.db, &id).await else {
        return fhir_error(StatusCode::NOT_FOUND, "not-found", format!("Organization/{id} not found"));
    };
    // Replace + `Updated` event, atomic under the active transport.
    let updated = match streaming::update_and_emit(&ctx.db, model, &org, caller.actor()).await {
        Ok(m) => m,
        Err(e) => return fhir_error(StatusCode::INTERNAL_SERVER_ERROR, "exception", e.to_string()),
    };
    Metrics::global().organization_updated_total.inc();
    audit(&ctx, updated.pid, "updated", caller.actor(), Some(updated.data.clone())).await;
    let resource = to_fhir_organization(
        &org,
        &updated.pid.to_string(),
        updated.active,
        Some(updated.updated_at.to_rfc3339()),
    );
    fhir_json(StatusCode::OK, &resource)
}

/// `DELETE /fhir/Organization/{id}` — soft-delete. `204` no body; `404`
/// unknown id. Audits + emits a `Deleted` event.
async fn remove(
    Path(id): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
) -> Response {
    let Ok(model) = OrgModel::find_by_pid(&ctx.db, &id).await else {
        return fhir_error(StatusCode::NOT_FOUND, "not-found", format!("Organization/{id} not found"));
    };
    // Soft-delete + `Deleted` event, atomic under the active transport.
    let entity_pid = match streaming::delete_and_emit(&ctx.db, model, caller.actor()).await {
        Ok((pid, _name)) => pid,
        Err(e) => return fhir_error(StatusCode::INTERNAL_SERVER_ERROR, "exception", e.to_string()),
    };
    Metrics::global().organization_deleted_total.inc();
    audit(&ctx, entity_pid, "deleted", caller.actor(), None).await;
    fhir_response(StatusCode::NO_CONTENT, Vec::new(), None)
}

/// `GET /fhir/Organization?<params>` — a `searchset` `Bundle` of matching
/// organizations. In-memory filter over active rows (capped), then the
/// `_count` page size. Supported params: see [`FhirOrgSearchParams`].
async fn search(
    Query(params): Query<FhirOrgSearchParams>,
    State(ctx): State<AppContext>,
) -> Response {
    let rows = match OrgModel::list(&ctx.db, FHIR_SEARCH_SCAN_CAP).await {
        Ok(rows) => rows,
        Err(e) => return fhir_error(StatusCode::INTERNAL_SERVER_ERROR, "exception", e.to_string()),
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
        let Ok(org) = model.to_org() else { continue };
        let pid = model.pid.to_string();
        if params.matches(&org, &pid) {
            resources.push(to_fhir_organization(
                &org,
                &pid,
                model.active,
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
                "type": "Organization",
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
                    {"name": "name", "type": "string"},
                    {"name": "address", "type": "string"},
                    {"name": "address-city", "type": "string"},
                    {"name": "address-postalcode", "type": "string"}
                ]
            }]
        }]
    });
    fhir_json(StatusCode::OK, &statement)
}

/// All FHIR routes, mounted under `/fhir`: the `Organization` resource
/// interactions + the `CapabilityStatement`. Literal `/metadata` is added
/// before the `/Organization/{id}` captures.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/fhir")
        .add("/metadata", get(metadata))
        .add("/Organization", post(create))
        .add("/Organization", get(search))
        .add("/Organization/{id}", get(read))
        .add("/Organization/{id}", put(update))
        .add("/Organization/{id}", delete(remove))
}
