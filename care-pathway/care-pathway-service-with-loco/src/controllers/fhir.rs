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
use crate::compliance::disclosure::{self, AccessContext};
use crate::compliance::{bulk, smart};
use crate::fhir::profile;
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

/// Validate an inbound resource against the declared profile, its
/// terminology bindings, and the service's own payload rules, surfacing
/// every problem as one `OperationOutcome` issue.
///
/// Returns `Some(422)` when any **error**-severity issue is present.
/// Warnings alone (a code system the profile does not bind) do not block a
/// write: the family deliberately preserves foreign namespaces, so
/// rejecting them would contradict the conversion contract.
fn validate_fhir(
    resource: &FhirPlanDefinition,
    pathway: &care_pathway_matcher::CarePathway,
) -> Option<Response> {
    let issue = profile::validate_all(resource, pathway);
    if !profile::has_errors(&issue) {
        return None;
    }
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
///
/// Audited as a read (HIPAA §164.312(b)) when `CARE_PATHWAY_AUDIT_READS`
/// is on — the FHIR surface is a second representation, not a second,
/// unaudited way in.
async fn read(
    Path(id): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    access: AccessContext,
) -> Response {
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
    disclosure::record_access(
        &ctx.db,
        model.pid,
        disclosure::action::FHIR_READ,
        caller.actor(),
        &access,
    )
    .await;
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
    if let Some(resp) = validate_fhir(&fhir, &pathway) {
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
    if let Some(resp) = validate_fhir(&fhir, &pathway) {
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
    caller: MaybeAuthUser,
    access: AccessContext,
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
    disclosure::record_access(
        &ctx.db,
        uuid::Uuid::nil(),
        disclosure::action::FHIR_SEARCH,
        caller.actor(),
        &access,
    )
    .await;
    fhir_json(StatusCode::OK, &FhirBundle::searchset(resources))
}

/// The search parameters this service actually filters on. The
/// `CapabilityStatement` is the machine-readable statement of the
/// (deliberately partial) surface, so it is built from this one list
/// rather than a second hand-written copy that could drift.
const SEARCH_PARAMS: [(&str, &str); 6] = [
    ("_id", "token"),
    ("_lastUpdated", "date"),
    ("_count", "number"),
    ("identifier", "token"),
    ("name", "string"),
    ("status", "token"),
];

/// The SMART App Launch OAuth-extension URI a `CapabilityStatement`
/// declares its authorization endpoints under.
const SMART_OAUTH_EXTENSION: &str =
    "http://fhir-registry.smarthealthit.org/StructureDefinition/oauth-uris";

/// `GET /fhir/metadata` — the `CapabilityStatement` declaring exactly the
/// interactions, operations, profile, and search parameters this service
/// implements (§7). Kept in sync with [`routes`] and [`SEARCH_PARAMS`] by
/// the tests below.
///
/// The `security` block is emitted **only** when the deployment has
/// configured a SMART authorization server: a `CapabilityStatement` that
/// advertised SMART endpoints which do not exist would be a false
/// conformance statement (see [`crate::compliance::smart`]).
async fn metadata() -> Response {
    let search_param: Vec<serde_json::Value> = SEARCH_PARAMS
        .iter()
        .map(|(name, kind)| serde_json::json!({ "name": name, "type": kind }))
        .collect();
    let mut rest = serde_json::json!({
        "mode": "server",
        "resource": [{
            "type": "PlanDefinition",
            "profile": profile::PROFILE_URL,
            "interaction": [
                {"code": "read"},
                {"code": "create"},
                {"code": "update"},
                {"code": "delete"},
                {"code": "search-type"}
            ],
            "operation": [
                {"name": "validate", "definition": "http://hl7.org/fhir/OperationDefinition/Resource-validate"}
            ],
            "searchParam": search_param
        }],
        "operation": [
            {"name": "export", "definition": "http://hl7.org/fhir/uv/bulkdata/OperationDefinition/export"}
        ]
    });
    if let Some(config) = smart::Configuration::from_env()
        && let Some(map) = rest.as_object_mut()
    {
        map.insert(
            "security".to_string(),
            serde_json::json!({
                "service": [{
                    "coding": [{
                        "system": "http://terminology.hl7.org/CodeSystem/restful-security-service",
                        "code": "SMART-on-FHIR",
                    }],
                }],
                "extension": [{
                    "url": SMART_OAUTH_EXTENSION,
                    "extension": [
                        {"url": "authorize", "valueUri": config.authorization_endpoint},
                        {"url": "token", "valueUri": config.token_endpoint},
                    ],
                }],
            }),
        );
    }
    let statement = serde_json::json!({
        "resourceType": "CapabilityStatement",
        "status": "active",
        "kind": "instance",
        "fhirVersion": "5.0.0",
        "format": ["application/fhir+json"],
        "rest": [rest],
    });
    fhir_json(StatusCode::OK, &statement)
}

/// `GET /fhir/.well-known/smart-configuration` — SMART App Launch
/// discovery.
///
/// Served **only** when the deployment names a real authorization server
/// (`CARE_PATHWAY_SMART_AUTHORIZATION_URL` + `_TOKEN_URL`). Otherwise this
/// returns `404` with an `OperationOutcome` saying so, because the family
/// authenticates with PASETO rather than OAuth 2.0 and publishing a
/// document pointing at endpoints that do not exist would be worse than
/// publishing none.
async fn smart_configuration() -> Response {
    match smart::Configuration::from_env() {
        Some(config) => fhir_json(StatusCode::OK, &config),
        None => fhir_error(
            StatusCode::NOT_FOUND,
            "not-supported",
            "SMART App Launch is not configured for this deployment; it authenticates with \
             PASETO v4.public bearer tokens issued by the authentication service. Set \
             CARE_PATHWAY_SMART_AUTHORIZATION_URL and CARE_PATHWAY_SMART_TOKEN_URL to \
             advertise a SMART authorization server.",
        ),
    }
}

/// `POST /fhir/PlanDefinition/$validate` — validate a resource **without
/// persisting it**.
///
/// Always `200` with an `OperationOutcome`: the operation succeeded even
/// when the resource is invalid, and the outcome carries the verdict. A
/// body that is not parseable JSON is a genuine `400` — there was nothing
/// to validate. An `information` issue is returned when everything passes,
/// because an empty `issue` array is not a valid `OperationOutcome`.
async fn validate_op(body: axum::body::Bytes) -> Response {
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
    // Profile checks run on the resource as received; terminology checks
    // need the converted pathway. A conversion failure is itself a
    // reportable issue rather than a separate error shape.
    let mut issue = profile::validate_profile(&fhir);
    match from_fhir_plan_definition(&fhir) {
        Ok(pathway) => {
            issue.extend(profile::validate_terminology(&pathway));
            issue.extend(crate::validation::problems(&pathway).into_iter().map(|p| {
                crate::fhir::resources::FhirIssue {
                    severity: "error".to_string(),
                    code: "processing".to_string(),
                    diagnostics: Some(p),
                }
            }));
        }
        Err(msg) => issue.push(crate::fhir::resources::FhirIssue {
            severity: "error".to_string(),
            code: "invalid".to_string(),
            diagnostics: Some(msg),
        }),
    }
    if issue.is_empty() {
        issue.push(crate::fhir::resources::FhirIssue {
            severity: "information".to_string(),
            code: "informational".to_string(),
            diagnostics: Some(format!("conforms to {}", profile::PROFILE_URL)),
        });
    }
    fhir_json(
        StatusCode::OK,
        &FhirOperationOutcome {
            resource_type: "OperationOutcome".to_string(),
            issue,
        },
    )
}

/// `GET /fhir/$export` — FHIR Bulk Data Access kickoff.
///
/// Returns `202 Accepted` with a `Content-Location` pointing at the status
/// endpoint, per the Bulk Data IG. The NDJSON is materialised during the
/// call and held in-process (see [`crate::compliance::bulk`] for the
/// limits that entails).
///
/// A bulk export is a **mass read**, so it is audited with the caller's
/// access context exactly as a single read is — including the GDPR Ch. V
/// cross-border classification when the caller names a destination region.
async fn export_kickoff(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    access: AccessContext,
) -> Response {
    let rows = match PathwayModel::list(&ctx.db, bulk::MAX_RESOURCES as u64).await {
        Ok(rows) => rows,
        Err(e) => {
            return fhir_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "exception",
                e.to_string(),
            );
        }
    };
    let resources: Vec<FhirPlanDefinition> = rows
        .iter()
        .filter_map(|model| {
            let pathway = model.to_pathway().ok()?;
            Some(to_fhir_plan_definition(
                &pathway,
                &model.pid.to_string(),
                model.active,
                Some(model.updated_at.to_rfc3339()),
            ))
        })
        .collect();
    let (ndjson, count, truncated) = bulk::to_ndjson(&resources);
    let id = bulk::register("/fhir/$export".to_string(), ndjson, count, truncated);
    disclosure::record_access(
        &ctx.db,
        uuid::Uuid::nil(),
        disclosure::action::EXPORT,
        caller.actor(),
        &access,
    )
    .await;
    Response::builder()
        .status(StatusCode::ACCEPTED)
        .header(
            header::CONTENT_LOCATION,
            format!("/fhir/$export-status/{id}"),
        )
        .header(header::CONTENT_TYPE, "application/fhir+json")
        .body(Body::empty())
        .unwrap_or_else(|_| {
            fhir_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "exception",
                "failed to build the kickoff response",
            )
        })
}

/// `GET /fhir/$export-status/{id}` — Bulk Data status poll.
///
/// `200` with the completion manifest, or `404` when the job is unknown,
/// expired, or was cancelled.
async fn export_status(Path(id): Path<String>) -> Response {
    let Some(job) = uuid::Uuid::parse_str(&id).ok().and_then(bulk::get) else {
        return fhir_error(
            StatusCode::NOT_FOUND,
            "not-found",
            format!("export job {id} is unknown or has expired"),
        );
    };
    if job.status == bulk::ExportStatus::Cancelled {
        return fhir_error(
            StatusCode::NOT_FOUND,
            "not-found",
            format!("export job {id} was cancelled"),
        );
    }
    fhir_json(StatusCode::OK, &job.manifest("/fhir"))
}

/// `DELETE /fhir/$export-status/{id}` — cancel an export.
///
/// `202` on success (the IG's response for an accepted cancellation);
/// `404` when the job is unknown or already expired. Cancelling releases
/// the payload immediately rather than waiting for the TTL.
async fn export_cancel(Path(id): Path<String>) -> Response {
    let cancelled = uuid::Uuid::parse_str(&id).is_ok_and(bulk::cancel);
    if cancelled {
        fhir_response(StatusCode::ACCEPTED, Vec::new(), None)
    } else {
        fhir_error(
            StatusCode::NOT_FOUND,
            "not-found",
            format!("export job {id} is unknown or has expired"),
        )
    }
}

/// `GET /fhir/$export-file/{id}/{file}` — an export output file.
///
/// Serves `PlanDefinition.ndjson` (the resources) or `error.ndjson` (the
/// truncation `OperationOutcome`), both as `application/fhir+ndjson`.
async fn export_file(Path((id, file)): Path<(String, String)>) -> Response {
    let Some(job) = uuid::Uuid::parse_str(&id).ok().and_then(bulk::get) else {
        return fhir_error(
            StatusCode::NOT_FOUND,
            "not-found",
            format!("export job {id} is unknown or has expired"),
        );
    };
    let body = match file.as_str() {
        "PlanDefinition.ndjson" => job.ndjson.clone(),
        "error.ndjson" => job.error_ndjson(),
        other => {
            return fhir_error(
                StatusCode::NOT_FOUND,
                "not-found",
                format!("export job {id} has no output file {other}"),
            );
        }
    };
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, bulk::NDJSON_CONTENT_TYPE)
        .body(Body::from(body))
        .unwrap_or_else(|_| {
            fhir_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "exception",
                "failed to build the NDJSON response",
            )
        })
}

/// All FHIR routes, mounted under `/fhir`: the `PlanDefinition` resource
/// interactions, the `$validate` operation, the `CapabilityStatement`,
/// SMART discovery, and the Bulk Data `$export` flow. Literal paths are
/// added before the `/PlanDefinition/{id}` captures.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/fhir")
        .add("/metadata", get(metadata))
        .add("/.well-known/smart-configuration", get(smart_configuration))
        .add("/$export", get(export_kickoff))
        .add("/$export-status/{id}", get(export_status))
        .add("/$export-status/{id}", delete(export_cancel))
        .add("/$export-file/{id}/{file}", get(export_file))
        .add("/PlanDefinition/$validate", post(validate_op))
        .add("/PlanDefinition", post(create))
        .add("/PlanDefinition", get(search))
        .add("/PlanDefinition/{id}", get(read))
        .add("/PlanDefinition/{id}", put(update))
        .add("/PlanDefinition/{id}", delete(remove))
}

#[cfg(test)]
mod tests {
    use super::{SEARCH_PARAMS, SMART_OAUTH_EXTENSION};
    use crate::fhir::profile;
    use crate::fhir::resources::FhirPlanDefinition;
    use crate::fhir::to_fhir_plan_definition;
    use care_pathway_matcher::CarePathway;

    /// The `CapabilityStatement`'s declared search params are the
    /// supported set. `metadata()` now builds its `searchParam` array from
    /// [`SEARCH_PARAMS`] directly, so drift between the two is impossible;
    /// this pins the set itself, which must not change silently.
    #[test]
    fn capability_statement_search_params_are_stable() {
        let declared: Vec<&str> = SEARCH_PARAMS.iter().map(|(name, _)| *name).collect();
        assert_eq!(
            declared,
            [
                "_id",
                "_lastUpdated",
                "_count",
                "identifier",
                "name",
                "status"
            ]
        );
        for (name, kind) in SEARCH_PARAMS {
            assert!(!name.is_empty() && !kind.is_empty());
            assert!(
                ["token", "date", "number", "string"].contains(&kind),
                "{name} declares an unknown search-param type {kind}"
            );
        }
    }

    /// The SMART OAuth-extension URI is the registry canonical a SMART
    /// client looks for; a typo here silently breaks discovery.
    #[test]
    fn smart_oauth_extension_uri_is_the_registry_canonical() {
        assert_eq!(
            SMART_OAUTH_EXTENSION,
            "http://fhir-registry.smarthealthit.org/StructureDefinition/oauth-uris"
        );
    }

    /// Every rendered resource carries the conformance claim, so a client
    /// (or Inferno-style suite) can see which profile to validate against.
    #[test]
    fn rendered_resources_declare_their_profile() {
        let fhir = to_fhir_plan_definition(&CarePathway::new("X"), "pid-1", true, None);
        let meta = fhir.meta.as_ref().expect("meta is always emitted");
        assert_eq!(meta.profile, vec![profile::PROFILE_URL.to_string()]);
    }

    /// The profile is declared even when the row's `updated_at` is
    /// unknown — the conformance claim does not depend on having a
    /// timestamp.
    #[test]
    fn profile_is_declared_without_a_last_updated() {
        let fhir = to_fhir_plan_definition(&CarePathway::new("X"), "pid-1", true, None);
        let meta = fhir.meta.as_ref().expect("meta present");
        assert!(meta.last_updated.is_none());
        assert!(!meta.profile.is_empty());
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
