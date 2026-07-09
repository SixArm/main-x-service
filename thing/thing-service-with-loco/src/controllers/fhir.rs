//! HL7 FHIR R5 endpoints for the `Device` resource.
//!
//! The mounted Axum surface backing `/fhir/Device` (read / create /
//! update / delete / search) plus `/fhir/metadata` (the
//! `CapabilityStatement`), per the family contract
//! ([`agents/share/fhir.md`](../../../../agents/share/fhir.md)). Handlers
//! return an Axum [`Response`] directly (not the native `ApiResponse`
//! envelope) so every response carries `application/fhir+json` and every
//! non-2xx body is a FHIR `OperationOutcome` (§5). Conversions bridge the
//! wire ⇄ stored [`crate::models::thing::Thing`] DTO via [`crate::fhir`];
//! writes reuse the same repository helpers, validators, audit trail, and
//! event stream as the native REST controller (§8). These routes sit
//! behind the blanket auth+ABAC guard (`/fhir/*` is guarded; only
//! `/fhir/metadata` is public).

use axum::{
    body::{Body, Bytes},
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::Response,
};
use loco_rs::controller::Routes;
use loco_rs::prelude::{delete, get, post, put};
use serde::Serialize;
use uuid::Uuid;

use crate::api::rest::AppState;
use crate::db::audit::AuditContext;
use crate::fhir::resources::{FhirBundle, FhirDevice, FhirOperationOutcome};
use crate::fhir::search::FhirDeviceSearchParams;
use crate::fhir::{from_fhir_device, to_fhir_device};
use crate::streaming::{EventKind, ThingEvent};
use crate::validation::{normalize_thing, validate_thing};

/// Max active rows scanned per FHIR search (in-memory filter, mirroring
/// the native duplicate-scan model; beyond this, candidates are silently
/// missed).
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

/// Parse a path `{id}` into a `Uuid`, or a boxed `404` `not-found`
/// response (an unparseable id can name no record). The error is boxed to
/// keep the `Result` small (`clippy::result_large_err`).
fn parse_id(id: &str) -> Result<Uuid, Box<Response>> {
    Uuid::parse_str(id).map_err(|_| {
        Box::new(fhir_error(
            StatusCode::NOT_FOUND,
            "not-found",
            format!("Device/{id} not found"),
        ))
    })
}

/// Best-effort audit write (never fails the request), mirroring the native
/// controller's audit path (no actor — `AuditContext::default()`).
async fn audit_create(state: &AppState, thing: &crate::models::thing::Thing) {
    if let Ok(v) = serde_json::to_value(thing) {
        let _ = state
            .audit_log
            .log_create("thing", thing.id, v, &AuditContext::default())
            .await;
    }
}

/// `GET /fhir/Device/{id}` — render a stored thing as a FHIR `Device`, or
/// a `404` `OperationOutcome` when the id is unknown.
async fn read(Path(id): Path<String>, State(state): State<AppState>) -> Response {
    let uuid = match parse_id(&id) {
        Ok(u) => u,
        Err(resp) => return *resp,
    };
    match state.thing_repository.get_by_id(&uuid).await {
        Ok(Some(thing)) => fhir_json(StatusCode::OK, &to_fhir_device(&thing)),
        Ok(None) => fhir_error(
            StatusCode::NOT_FOUND,
            "not-found",
            format!("Device/{id} not found"),
        ),
        Err(e) => fhir_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "exception",
            e.to_string(),
        ),
    }
}

/// Parse + validate an inbound FHIR body into a stored `Thing`. Returns
/// the normalized, validated record, or a `400`/`422` `OperationOutcome`.
fn parse_and_validate(body: &Bytes) -> Result<crate::models::thing::Thing, Box<Response>> {
    let fhir: FhirDevice = serde_json::from_slice(body).map_err(|e| {
        Box::new(fhir_error(
            StatusCode::BAD_REQUEST,
            "structure",
            format!("invalid FHIR JSON: {e}"),
        ))
    })?;
    let mut thing = from_fhir_device(&fhir)
        .map_err(|msg| Box::new(fhir_error(StatusCode::BAD_REQUEST, "invalid", msg)))?;
    normalize_thing(&mut thing);
    let errors = validate_thing(&thing);
    if !errors.is_empty() {
        return Err(Box::new(fhir_json(
            StatusCode::UNPROCESSABLE_ENTITY,
            &FhirOperationOutcome::errors(
                "processing",
                errors.iter().map(|e| format!("{}: {}", e.field, e.message)),
            ),
        )));
    }
    Ok(thing)
}

/// `POST /fhir/Device` — create from a FHIR `Device` payload. `201` with
/// the created resource + `Location`; `400` on unparseable/invalid FHIR;
/// `422` on data-quality failure. Audits + emits a `Created` event like
/// the native path.
async fn create(State(state): State<AppState>, body: Bytes) -> Response {
    let thing = match parse_and_validate(&body) {
        Ok(t) => t,
        Err(resp) => return *resp,
    };
    let stored = match state.thing_repository.create(&thing).await {
        Ok(s) => s,
        Err(e) => {
            return fhir_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "exception",
                e.to_string(),
            );
        }
    };
    let _ = state.search_engine.index_thing(&stored);
    let _ = state
        .event_publisher
        .publish(ThingEvent::new(
            EventKind::ThingCreated,
            stored.id,
            serde_json::json!({ "name": stored.name }),
        ))
        .await;
    audit_create(&state, &stored).await;
    let pid = stored.id.to_string();
    let resource = to_fhir_device(&stored);
    match serde_json::to_vec(&resource) {
        Ok(bytes) => fhir_response(StatusCode::CREATED, bytes, Some(format!("Device/{pid}"))),
        Err(e) => fhir_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "exception",
            e.to_string(),
        ),
    }
}

/// `PUT /fhir/Device/{id}` — replace from a FHIR payload. `200` with the
/// updated resource; `404` unknown id; `400`/`422` invalid FHIR. Audits +
/// emits an `Updated` event.
async fn update(Path(id): Path<String>, State(state): State<AppState>, body: Bytes) -> Response {
    let uuid = match parse_id(&id) {
        Ok(u) => u,
        Err(resp) => return *resp,
    };
    let mut thing = match parse_and_validate(&body) {
        Ok(t) => t,
        Err(resp) => return *resp,
    };
    let old = match state.thing_repository.get_by_id(&uuid).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            return fhir_error(
                StatusCode::NOT_FOUND,
                "not-found",
                format!("Device/{id} not found"),
            );
        }
        Err(e) => {
            return fhir_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "exception",
                e.to_string(),
            );
        }
    };
    thing.id = uuid;
    let stored = match state.thing_repository.update(&thing).await {
        Ok(s) => s,
        Err(e) => {
            return fhir_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "exception",
                e.to_string(),
            );
        }
    };
    let _ = state.search_engine.delete_thing(&uuid.to_string());
    let _ = state.search_engine.index_thing(&stored);
    let _ = state
        .event_publisher
        .publish(ThingEvent::new(
            EventKind::ThingUpdated,
            stored.id,
            serde_json::json!({ "name": stored.name }),
        ))
        .await;
    if let (Ok(old_v), Ok(new_v)) = (serde_json::to_value(&old), serde_json::to_value(&stored)) {
        let _ = state
            .audit_log
            .log_update("thing", stored.id, old_v, new_v, &AuditContext::default())
            .await;
    }
    fhir_json(StatusCode::OK, &to_fhir_device(&stored))
}

/// `DELETE /fhir/Device/{id}` — soft-delete. `204` no body; `404` unknown
/// id. Audits + emits a `Deleted` event.
async fn remove(Path(id): Path<String>, State(state): State<AppState>) -> Response {
    let uuid = match parse_id(&id) {
        Ok(u) => u,
        Err(resp) => return *resp,
    };
    let old = match state.thing_repository.get_by_id(&uuid).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            return fhir_error(
                StatusCode::NOT_FOUND,
                "not-found",
                format!("Device/{id} not found"),
            );
        }
        Err(e) => {
            return fhir_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "exception",
                e.to_string(),
            );
        }
    };
    if let Err(e) = state.thing_repository.soft_delete(&uuid).await {
        return fhir_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "exception",
            e.to_string(),
        );
    }
    let _ = state.search_engine.delete_thing(&uuid.to_string());
    let _ = state
        .event_publisher
        .publish(ThingEvent::new(
            EventKind::ThingDeleted,
            uuid,
            serde_json::json!({}),
        ))
        .await;
    if let Ok(v) = serde_json::to_value(&old) {
        let _ = state
            .audit_log
            .log_delete("thing", uuid, v, &AuditContext::default())
            .await;
    }
    fhir_response(StatusCode::NO_CONTENT, Vec::new(), None)
}

/// `GET /fhir/Device?<params>` — a `searchset` `Bundle` of matching
/// things. In-memory filter over active rows (capped), then the `_count`
/// page size. Supported params: see [`FhirDeviceSearchParams`].
async fn search(
    Query(params): Query<FhirDeviceSearchParams>,
    State(state): State<AppState>,
) -> Response {
    let rows = match state.thing_repository.list(FHIR_SEARCH_SCAN_CAP, 0).await {
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
    for thing in &rows {
        if params.matches(thing) {
            resources.push(to_fhir_device(thing));
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
                "type": "Device",
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
                    {"name": "type", "type": "token"},
                    {"name": "manufacturer", "type": "string"}
                ]
            }]
        }]
    });
    fhir_json(StatusCode::OK, &statement)
}

/// All FHIR routes, mounted under `/fhir`: the `Device` resource
/// interactions + the `CapabilityStatement`. Literal `/metadata` is added
/// before the `/Device/{id}` captures.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/fhir")
        .add("/metadata", get(metadata))
        .add("/Device", post(create))
        .add("/Device", get(search))
        .add("/Device/{id}", get(read))
        .add("/Device/{id}", put(update))
        .add("/Device/{id}", delete(remove))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `CapabilityStatement` advertises exactly the mounted resource
    /// type, interactions, and the supported search params (kept in sync
    /// with [`routes`]).
    #[tokio::test]
    async fn metadata_declares_device_and_search_params() {
        let response = metadata().await;
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let doc: serde_json::Value = serde_json::from_slice(&bytes).expect("valid JSON");
        assert_eq!(doc["resourceType"], "CapabilityStatement");
        let resource = &doc["rest"][0]["resource"][0];
        assert_eq!(resource["type"], "Device");
        let params: Vec<&str> = resource["searchParam"]
            .as_array()
            .expect("searchParam array")
            .iter()
            .map(|p| p["name"].as_str().expect("param name"))
            .collect();
        for expected in [
            "_id",
            "_lastUpdated",
            "_count",
            "identifier",
            "type",
            "manufacturer",
        ] {
            assert!(
                params.contains(&expected),
                "missing search param {expected}"
            );
        }
    }

    /// The mounted route group binds the `Device` interactions and the
    /// capability endpoint under `/fhir`.
    #[test]
    fn routes_bind_device_and_metadata() {
        let routes = routes();
        for uri in ["/metadata", "/Device", "/Device/{id}"] {
            assert!(
                routes.handlers.iter().any(|h| h.uri == uri),
                "routes missing {uri}"
            );
        }
    }
}
