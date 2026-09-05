//! [`PersonGrpcService`]: the `PersonService` trait implementation.
//!
//! Every RPC delegates to [`AppState`]'s `person_repository` /
//! `search_engine` / `matcher` — the same handles
//! [`crate::api::rest::handlers`] calls — and to the shared validation
//! ([`crate::validation::validate_person`]), duplicate-detection
//! ([`crate::api::rest::handlers::check_duplicates_internal`]), and
//! auth (`crate::api::rest::auth`) modules. Nothing here reimplements a
//! REST business rule; this file is wire conversion (`proto::Person` ⇄
//! [`crate::models::Person`]) plus RPC-shaped error mapping
//! (`tonic::Status` codes instead of HTTP status codes) around calls
//! into that shared code.
//!
//! ## Auth parity with REST — what matches, what does not (yet)
//!
//! [`grpc_enforce`] is the gRPC counterpart of
//! [`crate::api::rest::auth::enforce`] (REST's blanket-guard
//! middleware): `PERSON_REQUIRE_AUTH` off ⇒ every RPC is open, exactly
//! today's REST default; on ⇒ a missing/invalid bearer (in the
//! `authorization` gRPC metadata entry, `Bearer <paseto>`) is
//! `UNAUTHENTICATED` and a denied ABAC policy decision is
//! `PERMISSION_DENIED` — the same 401/403 split REST makes, in gRPC's
//! terms. `GetPerson` and `DeletePerson` additionally run
//! [`crate::api::rest::auth::authorize_record`] against the loaded
//! record (REST does the same before a single-record read/write), and
//! `GetPerson` honours a `mask` obligation exactly as
//! `crate::api::rest::handlers::get_person` does. `GetPerson` also
//! writes the HIPAA §164.528 disclosure-accounting audit row REST
//! writes on every read (T-36): [`grpc_access_context`] is the
//! gRPC-metadata counterpart of the Axum
//! [`crate::compliance::disclosure::AccessContext`] extractor — client
//! IP / user-agent still have nowhere to come from on this transport,
//! same as REST, so both stay `None` there too. `ListPersons` now also
//! applies REST's SEC-G3 per-record read-visibility filtering/masking
//! (T-37), via the shared, `pub(crate)`
//! [`crate::api::rest::handlers::search_result_disposition`] — a denied
//! record is omitted (never revealed to exist) and a `mask`-obligated
//! one is redacted, exactly as REST's `list_persons`/`search_persons`
//! do; this RPC's proto has no client-requested `mask_sensitive` field,
//! so only the ABAC obligation can trigger masking here.
//!
//! What is **not** carried over to this slice, and is tracked rather
//! than silently absent (this crate's spec `T-6`): `UpdatePerson`
//! entirely (no RPC for it yet).

use uuid::Uuid;

use authentication_verifier::{Action, Claims};
use tonic::metadata::MetadataMap;
use tonic::{Request, Response, Status};

use super::proto;
use crate::api::rest::AppState;
use crate::api::rest::auth::{self, MaybeAuthUser};
use crate::api::rest::handlers::{
    ResultDisposition, check_duplicates_internal, search_result_disposition,
};
use crate::compliance::disclosure;
use crate::models::{Gender, HumanName, Person};

/// Same offset ceiling as REST's SEC-G7 bound
/// (`crate::api::rest::handlers::MAX_SEARCH_OFFSET`, private to that
/// module) — declared again here rather than exported, since the two
/// call sites otherwise share no code to factor the constant into.
/// Unbounded paging materialises arbitrarily many rows only to skip
/// them, which is a cheap denial of service regardless of transport.
const MAX_LIST_OFFSET: u32 = 10_000;

/// Default `ListPersons` page size when the caller sends `limit: 0`,
/// and the ceiling any larger request is clamped to — matching REST's
/// `GET /api/persons` defaults exactly.
const DEFAULT_LIST_LIMIT: u32 = 10;
const MAX_LIST_LIMIT: u32 = 100;

/// The `PersonService` gRPC handler. Thin — just the shared
/// [`AppState`], cloned in from the same value `App::after_routes`
/// builds for the REST router (`src/app.rs`), so both surfaces share
/// one database pool, one search index, one matcher, one event
/// publisher.
pub struct PersonGrpcService {
    state: AppState,
}

impl PersonGrpcService {
    /// Wrap an [`AppState`] as a gRPC service.
    #[must_use]
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

/// Parse a proto `id` string as a [`Uuid`], `INVALID_ARGUMENT` on
/// failure rather than a REST-style `404` — the id is malformed, not
/// merely absent.
// `tonic::Status` is 176 bytes — large for an `Err` variant per
// `clippy::result_large_err`, but it is the RPC error type every
// `PersonService` trait method (fixed by the generated trait, not by
// us) must ultimately return, so every helper here funnels into it
// too; boxing it would only relocate the cost to `?`-propagation at
// each call site.
#[allow(clippy::result_large_err)]
fn parse_uuid(raw: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(raw).map_err(|_| Status::invalid_argument(format!("'{raw}' is not a UUID")))
}

/// Extract and verify the bearer token from gRPC request metadata, if
/// any was sent.
///
/// `Ok(None)` when no `authorization` entry is present at all (an
/// anonymous call). `Err` when one **was** sent but failed verification
/// — a presented-but-bad credential is never silently downgraded to
/// anonymous, on either transport (mirrors
/// [`crate::api::rest::auth::bearer_claims`]'s contract, just fed from
/// [`MetadataMap`] rather than an HTTP [`axum::http::HeaderMap`]).
#[allow(clippy::result_large_err)]
fn grpc_bearer_claims(metadata: &MetadataMap) -> Result<Option<Claims>, Status> {
    let Some(value) = metadata.get("authorization") else {
        return Ok(None);
    };
    let header = value
        .to_str()
        .map_err(|_| Status::unauthenticated("malformed authorization metadata"))?;
    let token = header
        .strip_prefix("Bearer ")
        .or_else(|| header.strip_prefix("bearer "))
        .ok_or_else(|| Status::unauthenticated("expected a bearer token"))?;
    auth::verifier()
        .current()
        .verify(token.trim())
        .map(Some)
        .map_err(|e| Status::unauthenticated(e.to_string()))
}

/// The blanket-enforcement decision for one RPC call — the gRPC
/// counterpart of [`crate::api::rest::auth::enforce`]. See the module
/// docs' "Auth parity with REST" section for the exact contract.
/// Returns the verified claims, if any, so the caller can build an
/// [`crate::db::AuditContext`] from the same identity REST would stamp
/// (via [`auth::audit_context_of`]).
#[allow(clippy::result_large_err)]
fn grpc_enforce(metadata: &MetadataMap, action: Action) -> Result<Option<Claims>, Status> {
    if !auth::require_auth_from_env() {
        // Enforcement off: still surface a presented-but-invalid token
        // as an error instead of quietly treating the call as
        // anonymous — a caller that sent a bad credential almost
        // certainly meant to authenticate.
        return grpc_bearer_claims(metadata);
    }
    let Some(claims) = grpc_bearer_claims(metadata)? else {
        return Err(Status::unauthenticated("missing authorization metadata"));
    };
    let decision = auth::policy()
        .current()
        .evaluate(&claims, action, auth::ENTITY);
    if decision.allowed {
        Ok(Some(claims))
    } else {
        Err(Status::permission_denied(decision.reason))
    }
}

/// Build a [`disclosure::AccessContext`] from gRPC request metadata —
/// this transport's counterpart of the REST `AccessContext`'s
/// `FromRequestParts` extractor, which reads the same three headers off
/// an Axum [`axum::http::HeaderMap`] instead (T-36). Extraction is
/// infallible here too: a missing or malformed metadata entry degrades
/// to an absent header, never a rejected call.
fn grpc_access_context(metadata: &MetadataMap) -> disclosure::AccessContext {
    let header = |name: &str| metadata.get(name).and_then(|v| v.to_str().ok());
    disclosure::AccessContext::from_parts(
        header(disclosure::PURPOSE_HEADER),
        header(disclosure::RECIPIENT_HEADER),
        header(disclosure::DESTINATION_HEADER),
    )
}

/// Map [`crate::api::rest::auth::authorize_record`]'s `(StatusCode,
/// String)` rejection onto the two `tonic::Status` codes it can
/// actually produce (`401`→`UNAUTHENTICATED`, `403`→
/// `PERMISSION_DENIED`; see that function's doc comment for why no
/// other status is possible).
fn map_record_authz_error((status, reason): (axum::http::StatusCode, String)) -> Status {
    if status == axum::http::StatusCode::UNAUTHORIZED {
        Status::unauthenticated(reason)
    } else {
        Status::permission_denied(reason)
    }
}

fn gender_to_proto(g: Gender) -> proto::Gender {
    match g {
        Gender::Male => proto::Gender::Male,
        Gender::Female => proto::Gender::Female,
        Gender::Other => proto::Gender::Other,
        Gender::Unknown => proto::Gender::Unknown,
    }
}

/// `GENDER_UNSPECIFIED` (proto3's required zero value) and an
/// unrecognised wire value both map to [`Gender::Unknown`] — the same
/// "safe default when source data is absent or unparseable" the domain
/// type's own doc comment already states.
fn gender_from_proto(raw: i32) -> Gender {
    match proto::Gender::try_from(raw).unwrap_or(proto::Gender::Unspecified) {
        proto::Gender::Male => Gender::Male,
        proto::Gender::Female => Gender::Female,
        proto::Gender::Other => Gender::Other,
        proto::Gender::Unspecified | proto::Gender::Unknown => Gender::Unknown,
    }
}

/// Project the persisted domain record onto the (deliberately partial —
/// see the module docs) proto message.
fn person_to_proto(p: &Person) -> proto::Person {
    proto::Person {
        id: p.id.to_string(),
        active: p.active,
        family_name: p.name.family.clone(),
        given_names: p.name.given.clone(),
        gender: gender_to_proto(p.gender) as i32,
        birth_date: p.birth_date.map(|d| d.to_string()),
        tax_id: p.tax_id.clone(),
        created_at: p.created_at.to_rfc3339(),
        updated_at: p.updated_at.to_rfc3339(),
    }
}

/// Build a new domain [`Person`] from a create-request's proto message.
/// `id`/`created_at`/`updated_at` are server-set — [`Person::new`]
/// fills them, and any value the caller sent for them is ignored.
#[allow(clippy::result_large_err)]
fn person_from_proto(p: proto::Person) -> Result<Person, Status> {
    let birth_date = p
        .birth_date
        .map(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d"))
        .transpose()
        .map_err(|_| {
            Status::invalid_argument("birth_date must be an ISO 8601 date, e.g. \"1990-01-15\"")
        })?;
    let mut person = Person::new(
        HumanName {
            use_type: None,
            family: p.family_name,
            given: p.given_names,
            prefix: vec![],
            suffix: vec![],
        },
        gender_from_proto(p.gender),
    );
    person.birth_date = birth_date;
    person.tax_id = p.tax_id;
    Ok(person)
}

#[tonic::async_trait]
impl proto::person_service_server::PersonService for PersonGrpcService {
    /// Validates ([`crate::validation::validate_person`]) and real-time
    /// duplicate-checks ([`check_duplicates_internal`]) exactly as
    /// `POST /api/persons` does — the same functions, not
    /// reimplementations of them.
    async fn create_person(
        &self,
        request: Request<proto::CreatePersonRequest>,
    ) -> Result<Response<proto::Person>, Status> {
        let claims = grpc_enforce(request.metadata(), Action::Write)?;

        let proto_person = request
            .into_inner()
            .person
            .ok_or_else(|| Status::invalid_argument("person is required"))?;
        let mut person = person_from_proto(proto_person)?;
        person.id = Uuid::new_v4();

        let validation_errors = crate::validation::validate_person(&person);
        if !validation_errors.is_empty() {
            let message = validation_errors
                .iter()
                .map(|e| format!("{}: {}", e.field, e.message))
                .collect::<Vec<_>>()
                .join("; ");
            return Err(Status::invalid_argument(format!(
                "validation failed: {message}"
            )));
        }

        let duplicates = check_duplicates_internal(&self.state, &person).await;
        if !duplicates.is_empty() {
            return Err(Status::already_exists(format!(
                "{} potential duplicate(s) found; review before creating",
                duplicates.len()
            )));
        }

        let ctx = auth::audit_context_of(&MaybeAuthUser(claims));
        let created = self
            .state
            .person_repository
            .create(&person, &ctx)
            .await
            .map_err(|e| Status::internal(format!("failed to create person: {e}")))?;
        if let Err(e) = self.state.search_engine.index_person(&created) {
            tracing::warn!("gRPC CreatePerson: failed to index in search engine: {e}");
        }
        Ok(Response::new(person_to_proto(&created)))
    }

    /// Record-level ABAC + `mask` obligation, exactly as
    /// `GET /api/persons/{id}` (`crate::api::rest::handlers::get_person`)
    /// applies them — same [`auth::authorize_record`] call, same
    /// [`crate::privacy::mask_person`] on a `mask` obligation, and now
    /// (T-36) the same HIPAA §164.528 disclosure-accounting audit row,
    /// via [`grpc_access_context`] in place of the Axum extractor.
    async fn get_person(
        &self,
        request: Request<proto::GetPersonRequest>,
    ) -> Result<Response<proto::Person>, Status> {
        let claims = grpc_enforce(request.metadata(), Action::Read)?;
        let access = grpc_access_context(request.metadata());
        let caller = MaybeAuthUser(claims);
        let id = parse_uuid(&request.into_inner().id)?;

        let person = self
            .state
            .person_repository
            .get_by_id(&id)
            .await
            .map_err(|e| Status::internal(format!("failed to retrieve person: {e}")))?
            .ok_or_else(|| Status::not_found(format!("person '{id}' not found")))?;

        let obligations =
            auth::authorize_record(&caller, Action::Read, &auth::person_resource_attrs(&person))
                .map_err(map_record_authz_error)?;

        // Audited only once authorization has allowed the read: a denied
        // request disclosed nothing, and recording it would pollute the
        // §164.528 accounting with accesses that never happened. Mirrors
        // `crate::api::rest::handlers::get_person` exactly.
        if disclosure::record_access(
            &self.state.audit_log,
            "Person",
            id,
            disclosure::action::READ,
            caller.claims().map(|c| c.sub.as_str()),
            &access,
        )
        .await
        .is_err()
        {
            return Err(Status::unavailable(
                "the access could not be recorded in the audit trail, so the read was refused",
            ));
        }

        let body = if obligations.iter().any(|o| o == "mask") {
            crate::privacy::mask_person(&person)
        } else {
            person
        };
        Ok(Response::new(person_to_proto(&body)))
    }

    /// SEC-G3 per-record read-visibility filtering/masking, exactly as
    /// REST's `list_persons`/`search_persons` apply it via the shared
    /// [`search_result_disposition`] (T-37, module docs): a record the
    /// caller may not read is omitted entirely (concealment, not just a
    /// denial the caller could infer existence from), and a
    /// `mask`-obligated one is redacted.
    async fn list_persons(
        &self,
        request: Request<proto::ListPersonsRequest>,
    ) -> Result<Response<proto::ListPersonsResponse>, Status> {
        let claims = grpc_enforce(request.metadata(), Action::Read)?;
        let caller = MaybeAuthUser(claims);
        let req = request.into_inner();

        if req.offset > MAX_LIST_OFFSET {
            return Err(Status::invalid_argument(format!(
                "offset must not exceed {MAX_LIST_OFFSET}; narrow the query instead"
            )));
        }
        let limit = if req.limit == 0 {
            DEFAULT_LIST_LIMIT
        } else {
            req.limit.min(MAX_LIST_LIMIT)
        };

        let rows = self
            .state
            .person_repository
            .list_active(u64::from(limit), u64::from(req.offset))
            .await
            .map_err(|e| Status::internal(format!("failed to list persons: {e}")))?;

        let mut persons = Vec::new();
        for person in &rows {
            let visibility = auth::read_visibility(&caller, person);
            // No client-requested `mask_sensitive` on this proto (unlike
            // REST's `ListQuery`), so only an ABAC `mask` obligation can
            // trigger masking here.
            match search_result_disposition(visibility.as_deref(), false) {
                ResultDisposition::Omit => {}
                ResultDisposition::Masked => {
                    persons.push(person_to_proto(&crate::privacy::mask_person(person)));
                }
                ResultDisposition::Full => persons.push(person_to_proto(person)),
            }
        }
        Ok(Response::new(proto::ListPersonsResponse { persons }))
    }

    /// Record-level ABAC exactly as `DELETE /api/persons/{id}`
    /// (`crate::api::rest::handlers::delete_person`) applies it, then a
    /// soft delete (the row is retained with `deleted_at` set) and a
    /// best-effort search-index removal, same as REST.
    async fn delete_person(
        &self,
        request: Request<proto::DeletePersonRequest>,
    ) -> Result<Response<proto::DeletePersonResponse>, Status> {
        let claims = grpc_enforce(request.metadata(), Action::Delete)?;
        let id = parse_uuid(&request.into_inner().id)?;

        let person = self
            .state
            .person_repository
            .get_by_id(&id)
            .await
            .map_err(|e| Status::internal(format!("failed to retrieve person: {e}")))?
            .ok_or_else(|| Status::not_found(format!("person '{id}' not found")))?;

        auth::authorize_record(
            &MaybeAuthUser(claims.clone()),
            Action::Delete,
            &auth::person_resource_attrs(&person),
        )
        .map_err(map_record_authz_error)?;

        let ctx = auth::audit_context_of(&MaybeAuthUser(claims));
        self.state
            .person_repository
            .delete(&id, &ctx)
            .await
            .map_err(|e| Status::internal(format!("failed to delete person: {e}")))?;
        if let Err(e) = self.state.search_engine.delete_person(&id.to_string()) {
            tracing::warn!("gRPC DeletePerson: failed to remove from search engine: {e}");
        }
        Ok(Response::new(proto::DeletePersonResponse {}))
    }
}
