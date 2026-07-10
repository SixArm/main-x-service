//! HL7 FHIR R5 endpoints for the `Appointment` resource.
//!
//! The mounted Axum surface backing `/fhir/Appointment` (read / create /
//! update / delete / search) plus `/fhir/metadata` (the
//! `CapabilityStatement`), per the family contract
//! ([`agents/share/fhir.md`](../../../../agents/share/fhir.md)). This is a
//! **best-effort, `low`-fidelity** mapping — schema.org/Event has no
//! clean FHIR analog (see [`crate::fhir`]); the `CapabilityStatement`
//! declares the partial surface honestly.
//!
//! Handlers return an Axum [`Response`] directly (not loco's `Result`) so
//! every response carries `application/fhir+json` and every non-2xx body
//! is a FHIR `OperationOutcome` (§5). Conversions bridge the wire ⇄
//! stored [`Event`](crate::models::Event) via [`crate::fhir`]; writes
//! reuse the **same** [`EventRepository`](crate::db::EventRepository) (its
//! audit trail + event stream) and search index as the native REST
//! handlers (§8). These routes sit behind the blanket auth+ABAC guard
//! (`/fhir/*` is not on the public allow-list).

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::Response,
};
use loco_rs::controller::Routes;
use loco_rs::prelude::{delete, get, post, put};
use serde::Serialize;
use uuid::Uuid;

use crate::api::rest::AppState;
use crate::fhir::resources::{FhirAppointment, FhirBundle, FhirOperationOutcome};
use crate::fhir::search::FhirAppointmentSearchParams;
use crate::fhir::{from_fhir_appointment, to_fhir_appointment};

/// Max active rows scanned per FHIR search (in-memory filter, mirroring
/// the native list scan; beyond this, later candidates are missed).
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

/// A `404` `OperationOutcome` for an unknown / unparseable id.
fn not_found(id: &str) -> Response {
    fhir_error(
        StatusCode::NOT_FOUND,
        "not-found",
        format!("Appointment/{id} not found"),
    )
}

/// Run the native validators over `event`, returning a `422`
/// `OperationOutcome` (one issue) when it fails, else `None`.
fn validate(event: &crate::models::Event) -> Option<Response> {
    let errors = crate::validation::validate_event(event);
    if errors.is_empty() {
        return None;
    }
    let msg = errors
        .iter()
        .map(|e| format!("{}: {}", e.field, e.message))
        .collect::<Vec<_>>()
        .join("; ");
    Some(fhir_error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "processing",
        msg,
    ))
}

/// `GET /fhir/Appointment/{id}` — render a stored event as a FHIR
/// `Appointment`, or a `404` `OperationOutcome` when the id is unknown.
async fn read(Path(id): Path<String>, State(state): State<AppState>) -> Response {
    let Ok(uuid) = Uuid::parse_str(&id) else {
        return not_found(&id);
    };
    match state.event_repository.get_by_id(&uuid).await {
        Ok(Some(event)) => fhir_json(StatusCode::OK, &to_fhir_appointment(&event)),
        Ok(None) => not_found(&id),
        Err(e) => fhir_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "exception",
            e.to_string(),
        ),
    }
}

/// `POST /fhir/Appointment` — create from a FHIR `Appointment` payload.
/// `201` with the created resource + `Location`; `400` on unparseable
/// FHIR; `422` on validation failure. Persists through the repository
/// (which audits + emits a `Created` event) exactly like the native path.
async fn create(State(state): State<AppState>, body: axum::body::Bytes) -> Response {
    let fhir: FhirAppointment = match serde_json::from_slice(&body) {
        Ok(f) => f,
        Err(e) => {
            return fhir_error(
                StatusCode::BAD_REQUEST,
                "structure",
                format!("invalid FHIR JSON: {e}"),
            );
        }
    };
    let event = match from_fhir_appointment(&fhir) {
        Ok(event) => event,
        Err(msg) => return fhir_error(StatusCode::BAD_REQUEST, "invalid", msg),
    };
    if let Some(resp) = validate(&event) {
        return resp;
    }
    match state.event_repository.create(&event).await {
        Ok(stored) => {
            if let Err(e) = state.search_engine.index_event(&stored) {
                tracing::warn!("Failed to index event (fhir): {}", e);
            }
            let pid = stored.id.to_string();
            let resource = to_fhir_appointment(&stored);
            match serde_json::to_vec(&resource) {
                Ok(bytes) => fhir_response(
                    StatusCode::CREATED,
                    bytes,
                    Some(format!("Appointment/{pid}")),
                ),
                Err(e) => fhir_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "exception",
                    e.to_string(),
                ),
            }
        }
        Err(e) => fhir_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "exception",
            e.to_string(),
        ),
    }
}

/// `PUT /fhir/Appointment/{id}` — replace from a FHIR payload. `200` with
/// the updated resource; `404` unknown id; `400` invalid FHIR; `422`
/// validation failure. Audits + emits an `Updated` event via the
/// repository.
async fn update(
    Path(id): Path<String>,
    State(state): State<AppState>,
    body: axum::body::Bytes,
) -> Response {
    let Ok(uuid) = Uuid::parse_str(&id) else {
        return not_found(&id);
    };
    let fhir: FhirAppointment = match serde_json::from_slice(&body) {
        Ok(f) => f,
        Err(e) => {
            return fhir_error(
                StatusCode::BAD_REQUEST,
                "structure",
                format!("invalid FHIR JSON: {e}"),
            );
        }
    };
    let mut event = match from_fhir_appointment(&fhir) {
        Ok(event) => event,
        Err(msg) => return fhir_error(StatusCode::BAD_REQUEST, "invalid", msg),
    };
    if let Some(resp) = validate(&event) {
        return resp;
    }
    match state.event_repository.get_by_id(&uuid).await {
        Ok(Some(_)) => {}
        Ok(None) => return not_found(&id),
        Err(e) => {
            return fhir_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "exception",
                e.to_string(),
            );
        }
    }
    // The path is authoritative: force the record id so a mismatched
    // body cannot retarget the write.
    event.id = uuid;
    match state.event_repository.update(&event).await {
        Ok(stored) => {
            if let Err(e) = state.search_engine.index_event(&stored) {
                tracing::warn!("Failed to update event in search index (fhir): {}", e);
            }
            fhir_json(StatusCode::OK, &to_fhir_appointment(&stored))
        }
        Err(e) => fhir_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "exception",
            e.to_string(),
        ),
    }
}

/// `DELETE /fhir/Appointment/{id}` — soft-delete. `204` no body; `404`
/// unknown id. Audits + emits a `Deleted` event via the repository.
async fn remove(Path(id): Path<String>, State(state): State<AppState>) -> Response {
    let Ok(uuid) = Uuid::parse_str(&id) else {
        return not_found(&id);
    };
    match state.event_repository.get_by_id(&uuid).await {
        Ok(Some(_)) => {}
        Ok(None) => return not_found(&id),
        Err(e) => {
            return fhir_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "exception",
                e.to_string(),
            );
        }
    }
    if let Err(e) = state.event_repository.delete(&uuid).await {
        return fhir_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "exception",
            e.to_string(),
        );
    }
    if let Err(e) = state.search_engine.delete_event(&uuid.to_string()) {
        tracing::warn!("Failed to delete event from search index (fhir): {}", e);
    }
    fhir_response(StatusCode::NO_CONTENT, Vec::new(), None)
}

/// `GET /fhir/Appointment?<params>` — a `searchset` `Bundle` of matching
/// appointments. In-memory filter over active rows (capped), then the
/// `_count` page size. Supported params: see
/// [`FhirAppointmentSearchParams`].
async fn search(
    Query(params): Query<FhirAppointmentSearchParams>,
    State(state): State<AppState>,
) -> Response {
    let rows = match state
        .event_repository
        .list_active(FHIR_SEARCH_SCAN_CAP, 0)
        .await
    {
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
    for event in &rows {
        let pid = event.id.to_string();
        if params.matches(event, &pid) {
            resources.push(to_fhir_appointment(event));
            if resources.len() >= limit {
                break;
            }
        }
    }
    fhir_json(StatusCode::OK, &FhirBundle::searchset(resources))
}

/// `GET /fhir/metadata` — the `CapabilityStatement` declaring exactly the
/// interactions and search parameters this service implements (§7),
/// honestly labelling the `Appointment` surface as best-effort. Kept in
/// sync with [`routes`] (a test pins the resource type + params).
async fn metadata() -> Response {
    let statement = serde_json::json!({
        "resourceType": "CapabilityStatement",
        "status": "active",
        "kind": "instance",
        "fhirVersion": "5.0.0",
        "format": ["application/fhir+json"],
        "implementationGuide": [
            "urn:mxi:note:appointment-is-a-best-effort-low-fidelity-mapping-of-schema-org-event"
        ],
        "rest": [{
            "mode": "server",
            "resource": [{
                "type": "Appointment",
                "documentation": "Best-effort, low-fidelity projection of a schema.org/Event onto FHIR R5 Appointment. Many Event fields have no Appointment home and are dropped; see the service FHIR docs.",
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
                    {"name": "date", "type": "date"}
                ]
            }]
        }]
    });
    fhir_json(StatusCode::OK, &statement)
}

/// All FHIR routes, mounted under `/fhir`: the `Appointment` resource
/// interactions + the `CapabilityStatement`. Literal `/metadata` is added
/// before the `/Appointment/{id}` captures.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/fhir")
        .add("/metadata", get(metadata))
        .add("/Appointment", post(create))
        .add("/Appointment", get(search))
        .add("/Appointment/{id}", get(read))
        .add("/Appointment/{id}", put(update))
        .add("/Appointment/{id}", delete(remove))
}

/// The same FHIR handlers as [`routes`], assembled as a plain
/// `axum::Router` (already `/fhir`-relative — nest it under `/fhir`).
///
/// Production serves FHIR via [`routes`] on the loco `App`; the
/// hand-written [`create_router`](crate::api::rest::create_router) test
/// harness is axum-native and cannot mount a loco `Routes`, so it uses
/// this builder — keeping the integration-test surface identical to
/// production instead of the old `501` stub.
pub fn axum_router(state: AppState) -> axum::Router {
    axum::Router::new()
        .route("/Appointment", axum::routing::post(create).get(search))
        .route(
            "/Appointment/{id}",
            axum::routing::get(read).put(update).delete(remove),
        )
        .route("/metadata", axum::routing::get(metadata))
        .with_state(state)
}
