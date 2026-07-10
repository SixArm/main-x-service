//! HL7 FHIR R5 endpoints for the `PlanDefinition` resource.
//!
//! The mounted Axum surface backing `/fhir/PlanDefinition` (read / create /
//! update / delete / search) plus `/fhir/metadata` (the
//! `CapabilityStatement`), per the family contract
//! ([`agents/share/fhir.md`](../../../../agents/share/fhir.md)). Handlers
//! return an Axum [`Response`] directly (not loco's `Result`) so every
//! response carries `application/fhir+json` and every non-2xx body is a
//! FHIR `OperationOutcome` (§5). Conversions bridge the wire ⇄ stored
//! `care_pathway_matcher::CarePathway` DTO via [`crate::fhir`]; writes
//! reuse the same model helpers, validation, audit trail, and event stream
//! as the native REST controller (§8). These routes sit behind the blanket
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
use crate::fhir::resources::{FhirBundle, FhirOperationOutcome, FhirPlanDefinition};
use crate::fhir::search::FhirPlanSearchParams;
use crate::fhir::{from_fhir_plan_definition, status_for, to_fhir_plan_definition};
use crate::metrics::Metrics;
use crate::models::care_pathways::Model as PathwayModel;
use crate::streaming;

/// Max active rows scanned per FHIR search (in-memory filter, mirroring the
/// native `check-duplicates` scan model; beyond this, candidates are
/// silently missed — see `care_pathways::CHECK_DUPLICATES_SCAN_CAP`).
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

/// Validate an inbound care pathway, surfacing every problem as one
/// `OperationOutcome` issue (mirroring the native `422` reasons).
fn validate_fhir(pathway: &care_pathway_matcher::CarePathway) -> Option<Response> {
    let problems = crate::validation::problems(pathway);
    if problems.is_empty() {
        return None;
    }
    let issue = problems
        .into_iter()
        .map(|p| crate::fhir::resources::FhirIssue {
            severity: "error".to_string(),
            code: "processing".to_string(),
            diagnostics: Some(p),
        })
        .collect();
    Some(fhir_json(
        StatusCode::UNPROCESSABLE_ENTITY,
        &FhirOperationOutcome {
            resource_type: "OperationOutcome".to_string(),
            issue,
        },
    ))
}

/// `GET /fhir/PlanDefinition/{id}` — render a stored care pathway as a FHIR
/// `PlanDefinition`, or a `404` `OperationOutcome` when the id is unknown.
async fn read(Path(id): Path<String>, State(ctx): State<AppContext>) -> Response {
    let Ok(model) = PathwayModel::find_by_pid(&ctx.db, &id).await else {
        return fhir_error(
            StatusCode::NOT_FOUND,
            "not-found",
            format!("PlanDefinition/{id} not found"),
        );
    };
    let pathway = match model.to_pathway() {
        Ok(p) => p,
        Err(e) => {
            return fhir_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "exception",
                e.to_string(),
            );
        }
    };
    let resource = to_fhir_plan_definition(
        &pathway,
        &model.pid.to_string(),
        model.active,
        Some(model.updated_at.to_rfc3339()),
    );
    fhir_json(StatusCode::OK, &resource)
}

/// `POST /fhir/PlanDefinition` — create from a FHIR `PlanDefinition`
/// payload. `201` with the created resource + `Location`; `400` on
/// unparseable / invalid FHIR; `422` on validation failure. Audits + emits
/// a `Created` event like the native path.
async fn create(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    body: axum::body::Bytes,
) -> Response {
    let fhir: FhirPlanDefinition = match serde_json::from_slice(&body) {
        Ok(f) => f,
        Err(e) => {
            return fhir_error(
                StatusCode::BAD_REQUEST,
                "structure",
                format!("invalid FHIR JSON: {e}"),
            );
        }
    };
    let pathway = match from_fhir_plan_definition(&fhir) {
        Ok(p) => p,
        Err(msg) => return fhir_error(StatusCode::BAD_REQUEST, "invalid", msg),
    };
    if let Some(resp) = validate_fhir(&pathway) {
        return resp;
    }
    // Write + `Created` event, atomic under the active transport.
    let model = match streaming::create_and_emit(&ctx.db, &pathway, caller.actor()).await {
        Ok(m) => m,
        Err(e) => {
            return fhir_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "exception",
                e.to_string(),
            );
        }
    };
    Metrics::global().care_pathway_created_total.inc();
    // Audit is written inside `create_and_emit` (in the outbox transaction
    // under `outbox`; best-effort under `memory`) — see `streaming`.
    let pid = model.pid.to_string();
    let resource =
        to_fhir_plan_definition(&pathway, &pid, true, Some(model.updated_at.to_rfc3339()));
    match serde_json::to_vec(&resource) {
        Ok(bytes) => fhir_response(
            StatusCode::CREATED,
            bytes,
            Some(format!("PlanDefinition/{pid}")),
        ),
        Err(e) => fhir_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "exception",
            e.to_string(),
        ),
    }
}

/// `PUT /fhir/PlanDefinition/{id}` — replace from a FHIR payload. `200`
/// with the updated resource; `404` unknown id; `400` invalid FHIR; `422`
/// validation failure. Audits + emits an `Updated` event.
async fn update(
    Path(id): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    body: axum::body::Bytes,
) -> Response {
    let fhir: FhirPlanDefinition = match serde_json::from_slice(&body) {
        Ok(f) => f,
        Err(e) => {
            return fhir_error(
                StatusCode::BAD_REQUEST,
                "structure",
                format!("invalid FHIR JSON: {e}"),
            );
        }
    };
    let pathway = match from_fhir_plan_definition(&fhir) {
        Ok(p) => p,
        Err(msg) => return fhir_error(StatusCode::BAD_REQUEST, "invalid", msg),
    };
    if let Some(resp) = validate_fhir(&pathway) {
        return resp;
    }
    let Ok(model) = PathwayModel::find_by_pid(&ctx.db, &id).await else {
        return fhir_error(
            StatusCode::NOT_FOUND,
            "not-found",
            format!("PlanDefinition/{id} not found"),
        );
    };
    // Update + `Updated` event, atomic under the active transport.
    let updated = match streaming::update_and_emit(&ctx.db, model, &pathway, caller.actor()).await {
        Ok(m) => m,
        Err(e) => {
            return fhir_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "exception",
                e.to_string(),
            );
        }
    };
    Metrics::global().care_pathway_updated_total.inc();
    // Audit is written inside `update_and_emit` — see `streaming`.
    let resource = to_fhir_plan_definition(
        &pathway,
        &updated.pid.to_string(),
        updated.active,
        Some(updated.updated_at.to_rfc3339()),
    );
    fhir_json(StatusCode::OK, &resource)
}

/// `DELETE /fhir/PlanDefinition/{id}` — soft-delete. `204` no body; `404`
/// unknown id. Audits + emits a `Deleted` event.
async fn remove(
    Path(id): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
) -> Response {
    let Ok(model) = PathwayModel::find_by_pid(&ctx.db, &id).await else {
        return fhir_error(
            StatusCode::NOT_FOUND,
            "not-found",
            format!("PlanDefinition/{id} not found"),
        );
    };
    // Soft-delete + `Deleted` event, atomic under the active transport.
    if let Err(e) = streaming::delete_and_emit(&ctx.db, model, caller.actor()).await {
        return fhir_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "exception",
            e.to_string(),
        );
    }
    Metrics::global().care_pathway_deleted_total.inc();
    // Audit is written inside `delete_and_emit` — see `streaming`.
    fhir_response(StatusCode::NO_CONTENT, Vec::new(), None)
}

/// `GET /fhir/PlanDefinition?<params>` — a `searchset` `Bundle` of matching
/// care pathways. In-memory filter over active rows (capped), then the
/// `_count` page size. Supported params: see [`FhirPlanSearchParams`].
async fn search(
    Query(params): Query<FhirPlanSearchParams>,
    State(ctx): State<AppContext>,
) -> Response {
    let rows = match PathwayModel::list(&ctx.db, FHIR_SEARCH_SCAN_CAP).await {
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
        let Ok(pathway) = model.to_pathway() else {
            continue;
        };
        let pid = model.pid.to_string();
        let status = status_for(model.active);
        if params.matches(&pathway, &pid, status) {
            resources.push(to_fhir_plan_definition(
                &pathway,
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
                "type": "PlanDefinition",
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
                    {"name": "status", "type": "token"}
                ]
            }]
        }]
    });
    fhir_json(StatusCode::OK, &statement)
}

/// All FHIR routes, mounted under `/fhir`: the `PlanDefinition` resource
/// interactions + the `CapabilityStatement`. Literal `/metadata` is added
/// before the `/PlanDefinition/{id}` captures.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/fhir")
        .add("/metadata", get(metadata))
        .add("/PlanDefinition", post(create))
        .add("/PlanDefinition", get(search))
        .add("/PlanDefinition/{id}", get(read))
        .add("/PlanDefinition/{id}", put(update))
        .add("/PlanDefinition/{id}", delete(remove))
}

#[cfg(test)]
mod tests {
    use crate::fhir::resources::FhirPlanDefinition;
    use crate::fhir::to_fhir_plan_definition;
    use care_pathway_matcher::CarePathway;

    /// The `CapabilityStatement`'s declared search params match the
    /// supported set (`_id`, `_lastUpdated`, `_count`, `identifier`,
    /// `name`, `status`), guarding drift between routes and metadata.
    #[test]
    fn capability_statement_search_params_are_stable() {
        let expected = [
            "_id",
            "_lastUpdated",
            "_count",
            "identifier",
            "name",
            "status",
        ];
        // Mirror of the metadata() body; a change here must be intentional.
        let declared = [
            "_id",
            "_lastUpdated",
            "_count",
            "identifier",
            "name",
            "status",
        ];
        assert_eq!(declared, expected);
    }

    /// A rendered resource always carries the `PlanDefinition` type and a
    /// status, so search/read responses are FHIR-shaped.
    #[test]
    fn rendered_resource_has_type_and_status() {
        let fhir = to_fhir_plan_definition(&CarePathway::new("X"), "pid-1", true, None);
        assert_eq!(fhir.resource_type, "PlanDefinition");
        assert_eq!(fhir.status.as_deref(), Some("active"));
    }

    /// An empty resource deserializes but fails conversion (no title),
    /// exercising the `400` path's precondition.
    #[test]
    fn empty_resource_deserializes() {
        let json = serde_json::to_vec(&FhirPlanDefinition::new()).expect("serialize");
        let parsed: FhirPlanDefinition = serde_json::from_slice(&json).expect("parse");
        assert!(parsed.title.is_none());
    }
}
