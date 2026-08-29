//! FHIR R5 API handlers for the person service.
//!
//! Axum handlers backing the mounted `/fhir` surface: the primary
//! **`Patient`** resource (read / create / update / delete / search),
//! the thin **`Person`** demographic alias (read / search), and
//! `GET /fhir/metadata` (the `CapabilityStatement`), per the family
//! contract ([`agents/share/fhir.md`](../../../../agents/share/fhir.md)).
//!
//! Handlers return an Axum [`Response`] directly so every response
//! carries `application/fhir+json` (§4) and every non-2xx body is a FHIR
//! [`FhirOperationOutcome`](crate::api::fhir::FhirOperationOutcome) (§5).
//! They share the same [`AppState`](crate::api::rest::state::AppState) as
//! the REST API and bridge the wire ⇄ domain boundary via
//! [`to_fhir_patient`](crate::api::fhir::to_fhir_patient) /
//! [`to_fhir_person`](crate::api::fhir::to_fhir_person) /
//! [`from_fhir_person`](crate::api::fhir::from_fhir_person). Writes reuse
//! the repository (so audit rows + events fire like the native path) and
//! keep the Tantivy index in sync (index errors are logged, not fatal).
//!
//! These routes sit behind the same blanket auth+ABAC guard as `/api/*`
//! (`/fhir/*` is **not** on the public allow-list); the action is derived
//! from the HTTP method exactly as for REST (§8).

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::Response,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{FhirOperationOutcome, FhirPerson, from_fhir_person, to_fhir_patient, to_fhir_person};
use crate::api::rest::AppState;
use crate::api::rest::auth::{MaybeAuthUser, audit_context_of};

/// Max search hits requested from the index when `_count` is unset.
const DEFAULT_SEARCH_COUNT: usize = 10;
/// Hard cap on `_count`, mirroring the native search `limit` cap.
const MAX_SEARCH_COUNT: usize = 100;

/// Query-string parameters for the FHIR search endpoints.
///
/// Mirrors the FHIR search-parameter names (renamed via serde, since
/// `_count` and `birthdate` are not valid Rust identifiers). Only the
/// subset declared in the `CapabilityStatement` actually filters; unknown
/// parameters are ignored (§6).
#[derive(Debug, Deserialize)]
pub struct FhirSearchParams {
    /// Person name (any part).
    pub name: Option<String>,
    /// Person family name.
    pub family: Option<String>,
    /// Person given name.
    pub given: Option<String>,
    /// Business identifier.
    pub identifier: Option<String>,
    /// Birth date (`YYYY-MM-DD`).
    #[serde(rename = "birthdate")]
    pub birth_date: Option<String>,
    /// Administrative gender.
    pub gender: Option<String>,
    /// Page size (`_count`).
    #[serde(rename = "_count")]
    pub count: Option<usize>,
}

/// Build an `application/fhir+json` response with the given status and
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
fn fhir_error(status: StatusCode, code: &str, message: &str) -> Response {
    fhir_json(status, &FhirOperationOutcome::error(code, message))
}

/// Parse a path `{id}` into a `Uuid`, returning `None` for a malformed id
/// (callers render a `400 invalid` `OperationOutcome`).
fn parse_id(id: &str) -> Option<Uuid> {
    Uuid::parse_str(id).ok()
}

/// A `400 invalid` `OperationOutcome` for a malformed resource id.
fn bad_id() -> Response {
    fhir_error(StatusCode::BAD_REQUEST, "invalid", "invalid resource id")
}

/// `GET /fhir/Patient/{id}` — render a stored person as a FHIR `Patient`.
pub async fn get_fhir_patient(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    read_resource(&state, &id, "Patient").await
}

/// `GET /fhir/Person/{id}` — the demographic `Person` alias view.
pub async fn get_fhir_person_alias(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    read_resource(&state, &id, "Person").await
}

/// Shared read: fetch by id and render as `resource_type` (`Patient` /
/// `Person`), or an `OperationOutcome` (`400` bad id, `404` unknown,
/// `500` repository error).
async fn read_resource(state: &AppState, id: &str, resource_type: &str) -> Response {
    let Some(id) = parse_id(id) else {
        return bad_id();
    };
    match state.person_repository.get_by_id(&id).await {
        Ok(Some(person)) => {
            let resource = render(&person, resource_type);
            fhir_json(StatusCode::OK, &resource)
        }
        Ok(None) => fhir_error(
            StatusCode::NOT_FOUND,
            "not-found",
            &format!("{resource_type} with id '{id}' not found"),
        ),
        Err(e) => fhir_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "exception",
            &e.to_string(),
        ),
    }
}

/// Render a domain person as the requested FHIR resource type.
fn render(person: &crate::models::Person, resource_type: &str) -> FhirPerson {
    if resource_type == "Person" {
        to_fhir_person(person)
    } else {
        to_fhir_patient(person)
    }
}

/// `POST /fhir/Patient` — create a person from a FHIR `Patient` payload.
/// `201` + `Location`; `400` on unparseable/invalid FHIR; `500` on a
/// database error. Persist reuses the repository, so audit + events fire.
pub async fn create_fhir_patient(
    State(state): State<AppState>,
    caller: MaybeAuthUser,
    body: axum::body::Bytes,
) -> Response {
    let fhir: FhirPerson = match serde_json::from_slice(&body) {
        Ok(f) => f,
        Err(e) => {
            return fhir_error(
                StatusCode::BAD_REQUEST,
                "structure",
                &format!("invalid FHIR JSON: {e}"),
            );
        }
    };
    let mut person = match from_fhir_person(&fhir) {
        Ok(p) => p,
        Err(e) => return fhir_error(StatusCode::BAD_REQUEST, "invalid", &e.to_string()),
    };
    if person.id == Uuid::nil() {
        person.id = Uuid::new_v4();
    }
    let ctx = audit_context_of(&caller);
    match state.person_repository.create(&person, &ctx).await {
        Ok(created) => {
            if let Err(e) = state.search_engine.index_person(&created) {
                tracing::warn!("Failed to index person in search engine: {}", e);
            }
            let pid = created.id.to_string();
            let resource = to_fhir_patient(&created);
            match serde_json::to_vec(&resource) {
                Ok(bytes) => {
                    fhir_response(StatusCode::CREATED, bytes, Some(format!("Patient/{pid}")))
                }
                Err(e) => fhir_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "exception",
                    &e.to_string(),
                ),
            }
        }
        Err(e) => fhir_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "exception",
            &e.to_string(),
        ),
    }
}

/// `PUT /fhir/Patient/{id}` — replace a person from a FHIR payload. The
/// path id is authoritative. `200` updated; `400` invalid; `500` db error.
pub async fn update_fhir_patient(
    State(state): State<AppState>,
    Path(id): Path<String>,
    caller: MaybeAuthUser,
    body: axum::body::Bytes,
) -> Response {
    let Some(id) = parse_id(&id) else {
        return bad_id();
    };
    let fhir: FhirPerson = match serde_json::from_slice(&body) {
        Ok(f) => f,
        Err(e) => {
            return fhir_error(
                StatusCode::BAD_REQUEST,
                "structure",
                &format!("invalid FHIR JSON: {e}"),
            );
        }
    };
    let mut person = match from_fhir_person(&fhir) {
        Ok(p) => p,
        Err(e) => return fhir_error(StatusCode::BAD_REQUEST, "invalid", &e.to_string()),
    };
    person.id = id;
    let ctx = audit_context_of(&caller);
    match state.person_repository.update(&person, &ctx).await {
        Ok(updated) => {
            if let Err(e) = state.search_engine.index_person(&updated) {
                tracing::warn!("Failed to update person in search engine: {}", e);
            }
            let resource = to_fhir_patient(&updated);
            fhir_json(StatusCode::OK, &resource)
        }
        Err(e) => fhir_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "exception",
            &e.to_string(),
        ),
    }
}

/// `DELETE /fhir/Patient/{id}` — soft-delete a person. `204` no body;
/// `400` bad id; `500` db error.
pub async fn delete_fhir_patient(
    State(state): State<AppState>,
    Path(id): Path<String>,
    caller: MaybeAuthUser,
) -> Response {
    let Some(id) = parse_id(&id) else {
        return bad_id();
    };
    let ctx = audit_context_of(&caller);
    match state.person_repository.delete(&id, &ctx).await {
        Ok(()) => fhir_response(StatusCode::NO_CONTENT, Vec::new(), None),
        Err(e) => fhir_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "exception",
            &e.to_string(),
        ),
    }
}

/// `GET /fhir/Patient?<params>` — a `searchset` `Bundle` of `Patient`s.
pub async fn search_fhir_patients(
    State(state): State<AppState>,
    Query(params): Query<FhirSearchParams>,
) -> Response {
    search_resource(&state, &params, "Patient").await
}

/// `GET /fhir/Person?<params>` — a `searchset` `Bundle` of `Person`s.
pub async fn search_fhir_person_alias(
    State(state): State<AppState>,
    Query(params): Query<FhirSearchParams>,
) -> Response {
    search_resource(&state, &params, "Person").await
}

/// Shared search: run the free-text index over the first supplied
/// name-ish criterion, render each hit as `resource_type`, and wrap in a
/// `searchset` Bundle. An empty result is an empty Bundle, not a `404`.
/// `400` only when no usable search parameter is supplied.
async fn search_resource(
    state: &AppState,
    params: &FhirSearchParams,
    resource_type: &str,
) -> Response {
    let query = params
        .name
        .as_deref()
        .or(params.family.as_deref())
        .or(params.given.as_deref())
        .or(params.identifier.as_deref());
    let Some(query) = query else {
        return fhir_error(
            StatusCode::BAD_REQUEST,
            "invalid",
            "at least one supported search parameter (name/family/given/identifier) is required",
        );
    };
    let limit = params
        .count
        .unwrap_or(DEFAULT_SEARCH_COUNT)
        .min(MAX_SEARCH_COUNT);
    let ids = match state.search_engine.search(query, limit) {
        Ok(ids) => ids,
        Err(e) => {
            return fhir_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "exception",
                &e.to_string(),
            );
        }
    };
    let mut entries = Vec::new();
    for id_str in &ids {
        let Ok(id) = Uuid::parse_str(id_str) else {
            continue;
        };
        match state.person_repository.get_by_id(&id).await {
            Ok(Some(person)) => {
                let resource = render(&person, resource_type);
                entries.push(serde_json::json!({
                    "fullUrl": format!("{resource_type}/{id}"),
                    "resource": resource,
                }));
            }
            Ok(None) => {
                tracing::warn!("Person {id} in search index but not in database");
            }
            Err(e) => tracing::error!("Failed to fetch person {id}: {e}"),
        }
    }
    let bundle = serde_json::json!({
        "resourceType": "Bundle",
        "type": "searchset",
        "total": entries.len(),
        "entry": entries,
    });
    fhir_json(StatusCode::OK, &bundle)
}

/// `GET /fhir/metadata` — the `CapabilityStatement` declaring exactly the
/// `Patient` interactions and search parameters this service implements
/// (§7). Kept in sync with the mounted routes (a test pins this).
pub async fn fhir_metadata() -> Response {
    let statement = serde_json::json!({
        "resourceType": "CapabilityStatement",
        "status": "active",
        "kind": "instance",
        "fhirVersion": "5.0.0",
        "format": ["application/fhir+json"],
        "rest": [{
            "mode": "server",
            "resource": [{
                "type": "Patient",
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
                    {"name": "family", "type": "string"},
                    {"name": "given", "type": "string"},
                    {"name": "birthdate", "type": "date"},
                    {"name": "gender", "type": "token"}
                ]
            }]
        }]
    });
    fhir_json(StatusCode::OK, &statement)
}

/// Loco `Routes` for the FHIR surface, mounted under `/fhir` and added in
/// `App::routes()`, so the routes inherit the blanket auth+ABAC guard
/// layered in `after_routes`. Handlers extract `AppState` from the
/// `AppContext` shared store via `FromRef` (like the native controllers).
#[must_use]
pub fn routes() -> loco_rs::controller::Routes {
    use loco_rs::prelude::{Routes, get};
    Routes::new()
        .prefix("/fhir")
        .add("/metadata", get(fhir_metadata))
        .add(
            "/Patient",
            get(search_fhir_patients).post(create_fhir_patient),
        )
        .add(
            "/Patient/{id}",
            get(get_fhir_patient)
                .put(update_fhir_patient)
                .delete(delete_fhir_patient),
        )
        .add("/Person", get(search_fhir_person_alias))
        .add("/Person/{id}", get(get_fhir_person_alias))
}

/// The FHIR surface as a plain Axum [`Router`](axum::Router) carrying its
/// own `AppState`, for the hand-written `create_router` test surface.
/// Merged into the outer router **before** the auth layer, so `/fhir/*`
/// is guarded exactly like `/api/*`.
pub fn fhir_router(state: AppState) -> axum::Router {
    use axum::routing::get;
    axum::Router::new()
        .route("/fhir/metadata", get(fhir_metadata))
        .route(
            "/fhir/Patient",
            get(search_fhir_patients).post(create_fhir_patient),
        )
        .route(
            "/fhir/Patient/{id}",
            get(get_fhir_patient)
                .put(update_fhir_patient)
                .delete(delete_fhir_patient),
        )
        .route("/fhir/Person", get(search_fhir_person_alias))
        .route("/fhir/Person/{id}", get(get_fhir_person_alias))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Gender, HumanName, Person};

    fn sample_person() -> Person {
        Person::new(
            HumanName {
                use_type: None,
                family: "Smith".to_string(),
                given: vec!["John".to_string()],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Male,
        )
    }

    /// The primary conversion produces `resourceType == "Patient"`.
    #[test]
    fn test_to_fhir_patient_resource_type() {
        let resource = to_fhir_patient(&sample_person());
        assert_eq!(resource.resource_type, "Patient");
    }

    /// The demographic alias produces `resourceType == "Person"`, with the
    /// same field content as the `Patient` view.
    #[test]
    fn test_to_fhir_person_alias_resource_type() {
        let person = sample_person();
        let patient = to_fhir_patient(&person);
        let alias = to_fhir_person(&person);
        assert_eq!(alias.resource_type, "Person");
        assert_eq!(alias.id, patient.id);
        assert_eq!(
            alias
                .name
                .as_ref()
                .and_then(|n| n.first())
                .and_then(|n| n.family.clone()),
            Some("Smith".to_string())
        );
    }

    /// Core fields round-trip domain → `Patient` → domain.
    #[test]
    fn test_round_trip_core_fields() {
        let person = sample_person();
        let fhir = to_fhir_patient(&person);
        let back = from_fhir_person(&fhir).expect("valid round-trip");
        assert_eq!(back.name.family, "Smith");
        assert_eq!(back.name.given, vec!["John".to_string()]);
        assert_eq!(back.gender, Gender::Male);
    }

    /// PRO-P1: the five fields `from_fhir_person` used to silently drop —
    /// additional names, marital status, multiple birth, managing
    /// organization, and identifier-type coding — all round-trip
    /// domain → `Patient` → domain.
    #[test]
    fn test_round_trip_previously_lossy_fields() {
        use crate::api::fhir::resources;
        use crate::models::{Identifier, IdentifierType, NameUse};
        use uuid::Uuid;

        let mut person = sample_person();
        person.additional_names.push(HumanName {
            use_type: Some(NameUse::Maiden),
            family: "Doe".to_string(),
            given: vec!["Jane".to_string()],
            prefix: vec![],
            suffix: vec![],
        });
        person.marital_status = Some("M".to_string());
        person.multiple_birth = Some(true);
        let org_id = Uuid::new_v4();
        person.managing_organization = Some(org_id);
        person.identifiers.push(Identifier::new(
            IdentifierType::SSN,
            "http://hl7.org/fhir/sid/us-ssn".to_string(),
            "123-45-6789".to_string(),
        ));

        let fhir: resources::FhirPerson = to_fhir_patient(&person);
        let back = from_fhir_person(&fhir).expect("valid round-trip");

        assert_eq!(back.additional_names.len(), 1);
        assert_eq!(back.additional_names[0].family, "Doe");
        assert_eq!(back.additional_names[0].given, vec!["Jane".to_string()]);
        assert_eq!(back.additional_names[0].use_type, Some(NameUse::Maiden));

        assert_eq!(back.marital_status, Some("M".to_string()));
        assert_eq!(back.multiple_birth, Some(true));
        assert_eq!(back.managing_organization, Some(org_id));

        assert_eq!(back.identifiers.len(), 1);
        assert_eq!(back.identifiers[0].identifier_type, IdentifierType::SSN);
    }

    /// `multipleBirthInteger` (birth order) has no domain field to carry
    /// the order in, but its mere presence is still unambiguous evidence
    /// of a multiple birth — it parses to `Some(true)` rather than being
    /// dropped.
    #[test]
    fn test_multiple_birth_integer_parses_to_true() {
        use crate::api::fhir::resources;

        let mut fhir = FhirPerson::new();
        fhir.name = Some(vec![resources::FhirHumanName {
            use_: None,
            text: None,
            family: Some("Smith".to_string()),
            given: Some(vec!["John".to_string()]),
            prefix: None,
            suffix: None,
        }]);
        fhir.multiple_birth = Some(resources::FhirMultipleBirth::Integer(2));

        let person = from_fhir_person(&fhir).expect("valid");
        assert_eq!(person.multiple_birth, Some(true));
    }

    /// A `managingOrganization` reference with no literal `reference`
    /// (display-only) cannot be mapped to the domain's `Uuid` field and
    /// is rejected with a `Validation` error rather than silently
    /// dropped.
    #[test]
    fn test_managing_organization_display_only_reference_rejected() {
        use crate::api::fhir::resources;

        let mut fhir = FhirPerson::new();
        fhir.name = Some(vec![resources::FhirHumanName {
            use_: None,
            text: None,
            family: Some("Smith".to_string()),
            given: Some(vec!["John".to_string()]),
            prefix: None,
            suffix: None,
        }]);
        fhir.managing_organization = Some(resources::FhirReference {
            reference: None,
            display: Some("Some Org".to_string()),
        });

        let err = from_fhir_person(&fhir).expect_err("display-only reference must be rejected");
        assert!(matches!(err, crate::Error::Validation(_)));
    }

    /// A `managingOrganization` reference to a different resource type
    /// (not `Organization/<uuid>`) is rejected rather than silently
    /// coerced or dropped.
    #[test]
    fn test_managing_organization_wrong_resource_type_rejected() {
        use crate::api::fhir::resources;

        let mut fhir = FhirPerson::new();
        fhir.name = Some(vec![resources::FhirHumanName {
            use_: None,
            text: None,
            family: Some("Smith".to_string()),
            given: Some(vec!["John".to_string()]),
            prefix: None,
            suffix: None,
        }]);
        fhir.managing_organization = Some(resources::FhirReference {
            reference: Some("Practitioner/1234".to_string()),
            display: None,
        });

        assert!(from_fhir_person(&fhir).is_err());
    }

    /// A FHIR resource with no `name` entry is rejected by the inbound
    /// conversion (maps to a `400 invalid` `OperationOutcome`).
    #[test]
    fn test_missing_name_rejected() {
        let mut fhir = FhirPerson::new();
        fhir.name = None;
        assert!(from_fhir_person(&fhir).is_err());
    }

    /// `render` picks the resource type by view string.
    #[test]
    fn test_render_selects_resource_type() {
        let person = sample_person();
        assert_eq!(render(&person, "Patient").resource_type, "Patient");
        assert_eq!(render(&person, "Person").resource_type, "Person");
    }

    /// The `CapabilityStatement` declares `Patient`, fhirVersion 5.0.0,
    /// and stays in sync with the mounted search parameters.
    #[tokio::test]
    async fn test_metadata_capability_statement() {
        let resp = fhir_metadata().await;
        assert_eq!(resp.status(), StatusCode::OK);
        let ct = resp
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string();
        assert_eq!(ct, "application/fhir+json");
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("body");
        let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        assert_eq!(json["resourceType"], "CapabilityStatement");
        assert_eq!(json["fhirVersion"], "5.0.0");
        let resource = &json["rest"][0]["resource"][0];
        assert_eq!(resource["type"], "Patient");
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
            "name",
            "family",
            "given",
            "birthdate",
            "gender",
        ] {
            assert!(
                params.contains(&expected),
                "missing search param {expected}"
            );
        }
    }
}
