//! HL7 FHIR R5 endpoints for the **non-standard** Course `Basic` resource.
//!
//! **There is no standard FHIR R5 resource for an educational course.** These
//! endpoints expose the Course registry as a deliberately **non-standard**
//! FHIR `Basic` resource (`code = course`), per
//! [`agents/share/fhir.md`](../../../../agents/share/fhir.md) §3 (`best-effort`
//! fidelity). It is a courtesy representation for FHIR-shaped tooling, **not**
//! interoperable with a standards-only client — the `CapabilityStatement`
//! (`GET /fhir/metadata`) says so explicitly.
//!
//! The mounted Axum surface backing `/fhir/Basic` (read / create / update /
//! delete / search) plus `/fhir/metadata`. Handlers return an Axum
//! [`Response`] directly (not the native `ApiResponse` envelope) so every
//! response carries `application/fhir+json` and every non-2xx body is a FHIR
//! `OperationOutcome` (§5). Conversions bridge the wire ⇄ stored
//! [`Course`] DTO via [`crate::fhir`]; writes reuse the same repository,
//! validators, audit trail, event stream, and metrics as the native REST
//! controller (§8).

use axum::{
    body::{Body, Bytes},
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::Response,
};
use loco_rs::controller::Routes;
use loco_rs::prelude::{delete, get, post, put};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

use super::state::AppState;
use crate::db::audit::AuditContext;
use crate::fhir::resources::{
    FhirBasic, FhirBundle, FhirIssue, FhirOperationOutcome, PROFILE_URL, RESOURCE_CODE,
    RESOURCE_CODE_SYSTEM, RESOURCE_TYPE,
};
use crate::fhir::search::FhirCourseSearchParams;
use crate::fhir::{from_fhir_basic, to_fhir_basic};
use crate::models::Course;
use crate::streaming::{CourseEvent, EventKind};
use crate::validation::{ValidationError, validate_course};

/// Max active rows scanned per FHIR search (in-memory filter, mirroring the
/// native scan model; beyond this, candidates are silently missed).
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

/// Serialize `body` as a FHIR resource with the given status. A serialization
/// failure degrades to a `500` `OperationOutcome`.
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

/// A `422` `OperationOutcome` carrying one issue per native validation error
/// (same detail as the REST `422`).
fn fhir_validation_error(errs: &[ValidationError]) -> Response {
    let outcome = FhirOperationOutcome {
        resource_type: "OperationOutcome".to_string(),
        issue: errs
            .iter()
            .map(|e| FhirIssue {
                severity: "error".to_string(),
                code: "processing".to_string(),
                diagnostics: Some(format!("{}: {}", e.field, e.message)),
            })
            .collect(),
    };
    fhir_json(StatusCode::UNPROCESSABLE_ENTITY, &outcome)
}

/// Audit + event side effects for a create (mirrors the native
/// `record_create`); failures are logged and swallowed.
async fn record_create(state: &AppState, course: &Course) {
    let new_json = serde_json::to_value(course).unwrap_or(Value::Null);
    if let Err(e) = state
        .audit_log
        .log_create(
            "Course",
            course.id,
            new_json.clone(),
            &AuditContext::default(),
        )
        .await
    {
        tracing::warn!("audit_log.log_create failed (fhir): {e}");
    }
    let evt = CourseEvent::course(EventKind::CourseCreated, course.id, new_json);
    if let Err(e) = state.event_publisher.publish(evt).await {
        tracing::warn!("event_publisher.publish failed (fhir): {e}");
    }
}

/// Audit + event side effects for an update (mirrors the native
/// `record_update`); failures are logged and swallowed.
async fn record_update(state: &AppState, old: Option<&Course>, new_value: &Course) {
    let old_json = old.map_or(Value::Null, |v| {
        serde_json::to_value(v).unwrap_or(Value::Null)
    });
    let new_json = serde_json::to_value(new_value).unwrap_or(Value::Null);
    if let Err(e) = state
        .audit_log
        .log_update(
            "Course",
            new_value.id,
            old_json,
            new_json.clone(),
            &AuditContext::default(),
        )
        .await
    {
        tracing::warn!("audit_log.log_update failed (fhir): {e}");
    }
    let evt = CourseEvent::course(EventKind::CourseUpdated, new_value.id, new_json);
    if let Err(e) = state.event_publisher.publish(evt).await {
        tracing::warn!("event_publisher.publish failed (fhir): {e}");
    }
}

/// Audit + event side effects for a soft delete (mirrors the native
/// `record_delete`); failures are logged and swallowed.
async fn record_delete(state: &AppState, old: &Course) {
    let old_json = serde_json::to_value(old).unwrap_or(Value::Null);
    if let Err(e) = state
        .audit_log
        .log_delete("Course", old.id, old_json.clone(), &AuditContext::default())
        .await
    {
        tracing::warn!("audit_log.log_delete failed (fhir): {e}");
    }
    let evt = CourseEvent::course(EventKind::CourseDeleted, old.id, old_json);
    if let Err(e) = state.event_publisher.publish(evt).await {
        tracing::warn!("event_publisher.publish failed (fhir): {e}");
    }
}

/// `GET /fhir/Basic/{id}` — render a stored course as the non-standard
/// `Basic`, or a `404` `OperationOutcome` when the id is unknown.
async fn read(Path(id): Path<String>, State(state): State<AppState>) -> Response {
    let Ok(uuid) = Uuid::parse_str(&id) else {
        return fhir_error(
            StatusCode::NOT_FOUND,
            "not-found",
            format!("Basic/{id} not found"),
        );
    };
    match state.course_repository.get_by_id(&uuid).await {
        Ok(Some(course)) => fhir_json(StatusCode::OK, &to_fhir_basic(&course)),
        Ok(None) => fhir_error(
            StatusCode::NOT_FOUND,
            "not-found",
            format!("Basic/{id} not found"),
        ),
        Err(e) => fhir_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "exception",
            e.to_string(),
        ),
    }
}

/// `POST /fhir/Basic` — create from a non-standard `Basic` payload. `201`
/// with the created resource + `Location`; `400` on unparseable / nameless
/// FHIR; `422` on native validation failure. Audits + emits a `Created`
/// event + increments metrics like the native path.
async fn create(State(state): State<AppState>, body: Bytes) -> Response {
    let fhir: FhirBasic = match serde_json::from_slice(&body) {
        Ok(f) => f,
        Err(e) => {
            return fhir_error(
                StatusCode::BAD_REQUEST,
                "structure",
                format!("invalid FHIR JSON: {e}"),
            );
        }
    };
    let course = match from_fhir_basic(&fhir) {
        Ok(c) => c,
        Err(msg) => return fhir_error(StatusCode::BAD_REQUEST, "invalid", msg),
    };
    let errs = validate_course(&course);
    if !errs.is_empty() {
        return fhir_validation_error(&errs);
    }
    let created = match state.course_repository.create(&course).await {
        Ok(c) => c,
        Err(e) => {
            return fhir_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "exception",
                e.to_string(),
            );
        }
    };
    if let Err(e) = state.search_engine.index_course(&created) {
        tracing::warn!("indexing course after fhir create failed: {e}");
    }
    record_create(&state, &created).await;
    crate::metrics::Metrics::global().course_created_total.inc();
    let pid = created.id.to_string();
    let resource = to_fhir_basic(&created);
    match serde_json::to_vec(&resource) {
        Ok(bytes) => fhir_response(StatusCode::CREATED, bytes, Some(format!("Basic/{pid}"))),
        Err(e) => fhir_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "exception",
            e.to_string(),
        ),
    }
}

/// `PUT /fhir/Basic/{id}` — replace from a `Basic` payload. `200` with the
/// updated resource; `404` unknown id; `400` invalid FHIR; `422` validation.
/// Audits + emits an `Updated` event + increments metrics.
async fn update(Path(id): Path<String>, State(state): State<AppState>, body: Bytes) -> Response {
    let Ok(uuid) = Uuid::parse_str(&id) else {
        return fhir_error(
            StatusCode::NOT_FOUND,
            "not-found",
            format!("Basic/{id} not found"),
        );
    };
    let fhir: FhirBasic = match serde_json::from_slice(&body) {
        Ok(f) => f,
        Err(e) => {
            return fhir_error(
                StatusCode::BAD_REQUEST,
                "structure",
                format!("invalid FHIR JSON: {e}"),
            );
        }
    };
    let mut course = match from_fhir_basic(&fhir) {
        Ok(c) => c,
        Err(msg) => return fhir_error(StatusCode::BAD_REQUEST, "invalid", msg),
    };
    course.id = uuid;
    let errs = validate_course(&course);
    if !errs.is_empty() {
        return fhir_validation_error(&errs);
    }
    let prior = state
        .course_repository
        .get_by_id(&uuid)
        .await
        .ok()
        .flatten();
    let updated = match state.course_repository.update(&course).await {
        Ok(c) => c,
        Err(crate::Error::NotFound) => {
            return fhir_error(
                StatusCode::NOT_FOUND,
                "not-found",
                format!("Basic/{id} not found"),
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
    if let Err(e) = state.search_engine.delete_course(&uuid.to_string()) {
        tracing::warn!("removing prior course segment after fhir update failed: {e}");
    }
    if let Err(e) = state.search_engine.index_course(&updated) {
        tracing::warn!("re-indexing course after fhir update failed: {e}");
    }
    record_update(&state, prior.as_ref(), &updated).await;
    crate::metrics::Metrics::global().course_updated_total.inc();
    fhir_json(StatusCode::OK, &to_fhir_basic(&updated))
}

/// `DELETE /fhir/Basic/{id}` — soft-delete. `204` no body; `404` unknown id.
/// Audits + emits a `Deleted` event + increments metrics.
async fn remove(Path(id): Path<String>, State(state): State<AppState>) -> Response {
    let Ok(uuid) = Uuid::parse_str(&id) else {
        return fhir_error(
            StatusCode::NOT_FOUND,
            "not-found",
            format!("Basic/{id} not found"),
        );
    };
    let prior = match state.course_repository.get_by_id(&uuid).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return fhir_error(
                StatusCode::NOT_FOUND,
                "not-found",
                format!("Basic/{id} not found"),
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
    match state.course_repository.soft_delete(&uuid).await {
        Ok(()) => {}
        Err(crate::Error::NotFound) => {
            return fhir_error(
                StatusCode::NOT_FOUND,
                "not-found",
                format!("Basic/{id} not found"),
            );
        }
        Err(e) => {
            return fhir_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "exception",
                e.to_string(),
            );
        }
    }
    if let Err(e) = state.search_engine.delete_course(&uuid.to_string()) {
        tracing::warn!("removing course segment after fhir soft-delete failed: {e}");
    }
    record_delete(&state, &prior).await;
    crate::metrics::Metrics::global().course_deleted_total.inc();
    fhir_response(StatusCode::NO_CONTENT, Vec::new(), None)
}

/// `GET /fhir/Basic?<params>` — a `searchset` `Bundle` of matching courses.
/// In-memory filter over active rows (capped), then the `_count` page size.
async fn search(
    Query(params): Query<FhirCourseSearchParams>,
    State(state): State<AppState>,
) -> Response {
    let rows = match state.course_repository.list(FHIR_SEARCH_SCAN_CAP, 0).await {
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
    for course in &rows {
        let pid = course.id.to_string();
        if params.matches(course, &pid) {
            resources.push(to_fhir_basic(course));
            if resources.len() >= limit {
                break;
            }
        }
    }
    fhir_json(StatusCode::OK, &FhirBundle::searchset(resources))
}

/// `GET /fhir/metadata` — the `CapabilityStatement`. It declares, and labels
/// as **non-standard**, the `Basic` (`code = course`) surface + profile, the
/// supported interactions (§4), and the supported search parameters (§6).
/// Kept in sync with [`routes`] (a test pins the resource type + params).
async fn metadata() -> Response {
    let statement = serde_json::json!({
        "resourceType": "CapabilityStatement",
        "status": "active",
        "kind": "instance",
        "fhirVersion": "5.0.0",
        "format": ["application/fhir+json"],
        "implementationGuide": [PROFILE_URL],
        "rest": [{
            "mode": "server",
            "documentation":
                "NON-STANDARD: no FHIR R5 resource models an educational course. \
                 Courses are exposed as a best-effort `Basic` resource with \
                 code {system: 'urn:mxi:resource', code: 'course'} and the \
                 profile declared in `implementationGuide`. Not interoperable \
                 with a standards-only client.",
            "resource": [{
                "type": RESOURCE_TYPE,
                "profile": PROFILE_URL,
                "documentation": format!(
                    "Non-standard course wrapper: Basic.code = {{{RESOURCE_CODE_SYSTEM} | {RESOURCE_CODE}}}."
                ),
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
                    {"name": "code", "type": "token"},
                    {"name": "name", "type": "string"}
                ]
            }]
        }]
    });
    fhir_json(StatusCode::OK, &statement)
}

/// All FHIR routes, mounted under `/fhir`: the non-standard course `Basic`
/// resource interactions + the `CapabilityStatement`. Literal `/metadata`
/// is added before the `/Basic/{id}` captures.
#[must_use]
pub fn fhir_routes() -> Routes {
    Routes::new()
        .prefix("/fhir")
        .add("/metadata", get(metadata))
        .add("/Basic", post(create))
        .add("/Basic", get(search))
        .add("/Basic/{id}", get(read))
        .add("/Basic/{id}", put(update))
        .add("/Basic/{id}", delete(remove))
}

/// DB-free pins for the FHIR route registration + the `CapabilityStatement`.
#[cfg(test)]
mod tests {
    use super::*;

    /// The FHIR routes are mounted under `/fhir` and cover the `Basic`
    /// interactions + `/metadata`.
    #[test]
    fn fhir_routes_are_mounted_under_fhir_prefix() {
        let routes = fhir_routes();
        assert_eq!(routes.prefix.as_deref(), Some("/fhir"));
        for uri in ["/metadata", "/Basic", "/Basic/{id}"] {
            assert!(
                routes.handlers.iter().any(|h| h.uri == uri),
                "missing {uri} handler in fhir_routes()"
            );
        }
    }

    /// The `CapabilityStatement` declares exactly the non-standard `Basic`
    /// resource type and the supported search params the routes serve — and
    /// labels the surface non-standard.
    #[tokio::test]
    async fn capability_statement_matches_routes_and_is_labelled_non_standard() {
        use axum::body::to_bytes;

        let resp = metadata().await;
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let doc: Value = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(doc["resourceType"], "CapabilityStatement");
        assert_eq!(doc["fhirVersion"], "5.0.0");
        let resource = &doc["rest"][0]["resource"][0];
        assert_eq!(resource["type"], RESOURCE_TYPE);
        assert_eq!(resource["profile"], PROFILE_URL);

        let params: Vec<&str> = resource["searchParam"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["name"].as_str().unwrap())
            .collect();
        for expected in [
            "_id",
            "_lastUpdated",
            "_count",
            "identifier",
            "code",
            "name",
        ] {
            assert!(
                params.contains(&expected),
                "capability missing search param {expected}"
            );
        }

        // The non-standard nature is stated in the server documentation.
        let doc_text = doc["rest"][0]["documentation"].as_str().unwrap();
        assert!(
            doc_text.contains("NON-STANDARD"),
            "capability must label the surface non-standard"
        );
    }
}
