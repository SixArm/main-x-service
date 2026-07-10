//! HL7 FHIR R5 endpoints for the `Location` resource.
//!
//! The mounted Axum surface backing `/fhir/Location` (read / create /
//! update / delete / search) plus `/fhir/metadata` (the
//! `CapabilityStatement`), per the family contract
//! ([`agents/share/fhir.md`](../../../../agents/share/fhir.md)). Handlers
//! return an Axum [`Response`] directly (not the native `ApiResponse`
//! envelope) so every response carries `application/fhir+json` and every
//! non-2xx body is a FHIR `OperationOutcome` (§5). Conversions bridge the
//! wire ⇄ stored [`crate::models::place::Place`] DTO via [`crate::fhir`];
//! writes reuse the same repository, validation, audit trail, event stream,
//! and search index as the native REST controller (§8). These routes sit
//! behind the blanket auth+ABAC guard (`/fhir/*` is not on the public
//! allow-list).

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
use crate::db::audit::AuditContext;
use crate::fhir::resources::{FhirBundle, FhirLocation, FhirOperationOutcome};
use crate::fhir::search::FhirLocationSearchParams;
use crate::fhir::{from_fhir_location, to_fhir_location};
use crate::streaming::{EventKind, PlaceEvent};
use crate::validation::{normalize_place, validate_place};

/// Max active rows scanned per FHIR search (in-memory filter). Beyond this,
/// candidates are silently missed; the handler logs a warning at the cap.
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

/// Parse the path `{id}` as a UUID, or a boxed `404 not-found` response (an
/// unparseable id is an unknown resource, never a `400`). The error is boxed
/// so the `Result` stays small (`clippy::result_large_err`).
fn parse_id(id: &str) -> Result<Uuid, Box<Response>> {
    Uuid::parse_str(id).map_err(|_| {
        Box::new(fhir_error(
            StatusCode::NOT_FOUND,
            "not-found",
            format!("Location/{id} not found"),
        ))
    })
}

/// Best-effort audit write (never fails the request), mirroring the native
/// controller's audit path. Uses a default (actor-less) context, exactly as
/// the native place handlers do.
async fn audit_create(state: &AppState, place: &crate::models::place::Place) {
    if let Ok(v) = serde_json::to_value(place)
        && let Err(err) = state
            .audit_log
            .log_create("place", place.id, v, &AuditContext::default())
            .await
    {
        tracing::warn!(error = %err, "failed to write audit log (fhir create)");
    }
}

/// `GET /fhir/Location/{id}` — render a stored place as a FHIR `Location`,
/// or a `404` `OperationOutcome` when the id is unknown / soft-deleted.
async fn read(Path(id): Path<String>, State(state): State<AppState>) -> Response {
    let uuid = match parse_id(&id) {
        Ok(u) => u,
        Err(resp) => return *resp,
    };
    match state.place_repository.get_by_id(&uuid).await {
        Ok(Some(place)) => {
            let resource = to_fhir_location(&place, Some(place.updated_at.to_rfc3339()));
            fhir_json(StatusCode::OK, &resource)
        }
        Ok(None) => fhir_error(
            StatusCode::NOT_FOUND,
            "not-found",
            format!("Location/{id} not found"),
        ),
        Err(e) => fhir_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "exception",
            e.to_string(),
        ),
    }
}

/// Parse + validate an inbound FHIR body into a normalized, valid [`Place`],
/// returning the FHIR error response on any failure.
///
/// The error response is boxed so the `Result` stays small
/// (`clippy::result_large_err`).
///
/// [`Place`]: crate::models::place::Place
fn parse_and_validate(body: &[u8]) -> Result<crate::models::place::Place, Box<Response>> {
    let fhir: FhirLocation = serde_json::from_slice(body).map_err(|e| {
        Box::new(fhir_error(
            StatusCode::BAD_REQUEST,
            "structure",
            format!("invalid FHIR JSON: {e}"),
        ))
    })?;
    let mut place = from_fhir_location(&fhir)
        .map_err(|msg| Box::new(fhir_error(StatusCode::BAD_REQUEST, "invalid", msg)))?;
    normalize_place(&mut place);
    let errors = validate_place(&place);
    if !errors.is_empty() {
        let diagnostics = errors
            .into_iter()
            .map(|e| format!("{}: {}", e.field, e.message));
        return Err(Box::new(fhir_json(
            StatusCode::UNPROCESSABLE_ENTITY,
            &FhirOperationOutcome::errors("processing", diagnostics),
        )));
    }
    Ok(place)
}

/// `POST /fhir/Location` — create from a FHIR `Location` payload. `201` with
/// the created resource + `Location`; `400` on unparseable / invalid FHIR;
/// `422` on validation failure. Indexes, audits, and emits a `PlaceCreated`
/// event like the native path.
async fn create(State(state): State<AppState>, body: axum::body::Bytes) -> Response {
    let place = match parse_and_validate(&body) {
        Ok(p) => p,
        Err(resp) => return *resp,
    };
    let stored = match state.place_repository.create(&place).await {
        Ok(s) => s,
        Err(e) => {
            return fhir_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "exception",
                e.to_string(),
            );
        }
    };
    let _ = state.search_engine.index_place(&stored);
    let _ = state
        .event_publisher
        .publish(PlaceEvent::new(
            EventKind::PlaceCreated,
            stored.id,
            serde_json::json!({ "name": stored.name }),
        ))
        .await;
    audit_create(&state, &stored).await;
    let pid = stored.id.to_string();
    let resource = to_fhir_location(&stored, Some(stored.updated_at.to_rfc3339()));
    match serde_json::to_vec(&resource) {
        Ok(bytes) => fhir_response(StatusCode::CREATED, bytes, Some(format!("Location/{pid}"))),
        Err(e) => fhir_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "exception",
            e.to_string(),
        ),
    }
}

/// `PUT /fhir/Location/{id}` — replace from a FHIR payload. `200` with the
/// updated resource; `404` unknown id; `400` invalid FHIR; `422` validation.
/// Re-indexes, audits, and emits a `PlaceUpdated` event.
async fn update(
    Path(id): Path<String>,
    State(state): State<AppState>,
    body: axum::body::Bytes,
) -> Response {
    let uuid = match parse_id(&id) {
        Ok(u) => u,
        Err(resp) => return *resp,
    };
    let mut place = match parse_and_validate(&body) {
        Ok(p) => p,
        Err(resp) => return *resp,
    };
    place.id = uuid;
    let old = match state.place_repository.get_by_id(&uuid).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return fhir_error(
                StatusCode::NOT_FOUND,
                "not-found",
                format!("Location/{id} not found"),
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
    let stored = match state.place_repository.update(&place).await {
        Ok(s) => s,
        Err(e) => {
            return fhir_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "exception",
                e.to_string(),
            );
        }
    };
    let _ = state.search_engine.delete_place(&uuid.to_string());
    let _ = state.search_engine.index_place(&stored);
    let _ = state
        .event_publisher
        .publish(PlaceEvent::new(
            EventKind::PlaceUpdated,
            stored.id,
            serde_json::json!({ "name": stored.name }),
        ))
        .await;
    if let (Ok(old_v), Ok(new_v)) = (serde_json::to_value(&old), serde_json::to_value(&stored))
        && let Err(err) = state
            .audit_log
            .log_update("place", stored.id, old_v, new_v, &AuditContext::default())
            .await
    {
        tracing::warn!(error = %err, "failed to write audit log (fhir update)");
    }
    let resource = to_fhir_location(&stored, Some(stored.updated_at.to_rfc3339()));
    fhir_json(StatusCode::OK, &resource)
}

/// `DELETE /fhir/Location/{id}` — soft-delete. `204` no body; `404` unknown
/// id. Removes from the index, audits, and emits a `PlaceDeleted` event.
async fn remove(Path(id): Path<String>, State(state): State<AppState>) -> Response {
    let uuid = match parse_id(&id) {
        Ok(u) => u,
        Err(resp) => return *resp,
    };
    let old = match state.place_repository.get_by_id(&uuid).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return fhir_error(
                StatusCode::NOT_FOUND,
                "not-found",
                format!("Location/{id} not found"),
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
    if let Err(e) = state.place_repository.soft_delete(&uuid).await {
        return fhir_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "exception",
            e.to_string(),
        );
    }
    let _ = state.search_engine.delete_place(&uuid.to_string());
    let _ = state
        .event_publisher
        .publish(PlaceEvent::new(
            EventKind::PlaceDeleted,
            uuid,
            serde_json::json!({}),
        ))
        .await;
    if let Ok(v) = serde_json::to_value(&old)
        && let Err(err) = state
            .audit_log
            .log_delete("place", uuid, v, &AuditContext::default())
            .await
    {
        tracing::warn!(error = %err, "failed to write audit log (fhir delete)");
    }
    fhir_response(StatusCode::NO_CONTENT, Vec::new(), None)
}

/// `GET /fhir/Location?<params>` — a `searchset` `Bundle` of matching
/// places. In-memory filter over active rows (capped), then the `_count`
/// page size. Supported params: see [`FhirLocationSearchParams`].
async fn search(
    Query(params): Query<FhirLocationSearchParams>,
    State(state): State<AppState>,
) -> Response {
    let rows = match state.place_repository.list(FHIR_SEARCH_SCAN_CAP, 0).await {
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
    for place in &rows {
        let pid = place.id.to_string();
        if params.matches(place, &pid) {
            resources.push(to_fhir_location(place, Some(place.updated_at.to_rfc3339())));
            if resources.len() >= limit {
                break;
            }
        }
    }
    fhir_json(StatusCode::OK, &FhirBundle::searchset(resources))
}

/// `GET /fhir/metadata` — the `CapabilityStatement` declaring exactly the
/// interactions and search parameters this service implements (§7). Kept in
/// sync with [`routes`] (a test pins the resource type + params).
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
                "type": "Location",
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

/// All FHIR routes, mounted under `/fhir`: the `Location` resource
/// interactions + the `CapabilityStatement`. Literal `/metadata` is added
/// before the `/Location/{id}` captures.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/fhir")
        .add("/metadata", get(metadata))
        .add("/Location", post(create))
        .add("/Location", get(search))
        .add("/Location/{id}", get(read))
        .add("/Location/{id}", put(update))
        .add("/Location/{id}", delete(remove))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `CapabilityStatement` advertises the `Location` resource and the
    /// documented search params (DB-free — pins the metadata contract).
    #[tokio::test]
    async fn metadata_lists_location_and_search_params() {
        let resp = metadata().await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("body");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(json["resourceType"], "CapabilityStatement");
        let resource = &json["rest"][0]["resource"][0];
        assert_eq!(resource["type"], "Location");
        let params: Vec<String> = resource["searchParam"]
            .as_array()
            .expect("searchParam array")
            .iter()
            .map(|p| p["name"].as_str().unwrap_or_default().to_string())
            .collect();
        for want in [
            "_id",
            "_lastUpdated",
            "_count",
            "identifier",
            "name",
            "address",
            "address-city",
            "address-postalcode",
        ] {
            assert!(
                params.contains(&want.to_string()),
                "missing search param {want}"
            );
        }
    }
}
