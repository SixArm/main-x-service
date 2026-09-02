//! [`WorkerGrpcService`]: the `WorkerService` trait implementation.
//!
//! Follows person-service's reference implementation
//! (`person-service::api::grpc::service`) for this repo's gRPC
//! rollout. Every RPC delegates to [`AppState`]'s `worker_repository` /
//! `search_engine` / `matcher` — the same handles
//! [`crate::api::rest::handlers`] calls — and to the shared validation
//! ([`crate::validation::validate_worker`]), duplicate-detection
//! ([`crate::api::rest::handlers::check_duplicates_internal`]), and
//! auth (`crate::api::rest::auth`) modules. Nothing here reimplements a
//! REST business rule; this file is wire conversion (`proto::Worker` ⇄
//! [`crate::models::Worker`]) plus RPC-shaped error mapping
//! (`tonic::Status` codes instead of HTTP status codes) around calls
//! into that shared code.
//!
//! ## Auth parity with REST — what matches, what does not (yet)
//!
//! [`grpc_enforce`] is the gRPC counterpart of
//! [`crate::api::rest::auth::enforce`] (REST's blanket-guard
//! middleware): `WORKER_REQUIRE_AUTH` off ⇒ every RPC is open, exactly
//! today's REST default; on ⇒ a missing/invalid bearer (in the
//! `authorization` gRPC metadata entry, `Bearer <paseto>`) is
//! `UNAUTHENTICATED` and a denied ABAC policy decision is
//! `PERMISSION_DENIED` — the same 401/403 split REST makes, in gRPC's
//! terms. `GetWorker` and `DeleteWorker` additionally run
//! [`crate::api::rest::auth::authorize_record`] against the loaded
//! record (REST does the same before a single-record read/write), and
//! `GetWorker` honours a `mask` obligation exactly as
//! `crate::api::rest::handlers::get_worker` does.
//!
//! What is **not** carried over to this slice, and is tracked rather
//! than silently absent (this crate's spec §13, PRO-H11): the HIPAA
//! §164.528 disclosure-accounting audit row REST writes on every read
//! (needs a gRPC-side `AccessContext` equivalent — client IP/user-agent
//! come from a different place than an Axum extractor); `ListWorkers`'
//! per-record read-visibility filtering (this RPC applies only the
//! blanket `Read` check today); and `UpdateWorker` entirely (no RPC for
//! it yet). Unlike person's repository, this crate's `WorkerRepository`
//! methods take no `AuditContext` — audit logging is wired internally
//! via the repository's `with_*` builders — so there is no
//! `audit_context_of` equivalent to call here.

use uuid::Uuid;

use authentication_verifier::{Action, Claims};
use tonic::metadata::MetadataMap;
use tonic::{Request, Response, Status};

use super::proto;
use crate::api::rest::AppState;
use crate::api::rest::auth::{self, MaybeAuthUser};
use crate::api::rest::handlers::check_duplicates_internal;
use crate::models::{Gender, HumanName, Worker, worker::WorkerType};

/// Same offset ceiling as REST's SEC-G7 bound
/// (`crate::api::rest::handlers::MAX_LIST_OFFSET`, private to that
/// module) — declared again here rather than exported, since the two
/// call sites otherwise share no code to factor the constant into.
/// Unbounded paging materialises arbitrarily many rows only to skip
/// them, which is a cheap denial of service regardless of transport.
const MAX_LIST_OFFSET: u32 = 10_000;

/// Default `ListWorkers` page size when the caller sends `limit: 0`,
/// and the ceiling any larger request is clamped to — matching REST's
/// `GET /api/workers` defaults exactly.
const DEFAULT_LIST_LIMIT: u32 = 10;
const MAX_LIST_LIMIT: u32 = 100;

/// The `WorkerService` gRPC handler. Thin — just the shared
/// [`AppState`], cloned in from the same value `App::after_routes`
/// builds for the REST router (`src/app.rs`), so both surfaces share
/// one database pool, one search index, one matcher, one event
/// publisher.
pub struct WorkerGrpcService {
    state: AppState,
}

impl WorkerGrpcService {
    /// Wrap an [`AppState`] as a gRPC service.
    #[must_use]
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

/// Parse a proto `id` string as a [`Uuid`], `INVALID_ARGUMENT` on
/// failure rather than a REST-style `404` — the id is malformed, not
/// merely absent.
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
/// Returns the verified claims, if any, so the caller can pass them
/// into [`auth::authorize_record`].
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

/// Parse a `worker_type` wire token (e.g. `"doctor"`) into the domain
/// enum by reusing its existing `serde` implementation
/// (`#[serde(rename_all = "snake_case")]`) rather than hand-rolling a
/// second mapping that could drift from it.
#[allow(clippy::result_large_err)]
fn worker_type_from_proto(raw: Option<String>) -> Result<Option<WorkerType>, Status> {
    raw.map(|s| {
        serde_json::from_value(serde_json::Value::String(s.clone()))
            .map_err(|_| Status::invalid_argument(format!("'{s}' is not a recognised worker_type")))
    })
    .transpose()
}

/// Project the persisted domain record onto the (deliberately partial —
/// see the module docs) proto message. `WorkerType`'s `Display` impl
/// already renders the same `snake_case` token its `serde` form does.
fn worker_to_proto(w: &Worker) -> proto::Worker {
    proto::Worker {
        id: w.id.to_string(),
        active: w.active,
        family_name: w.name.family.clone(),
        given_names: w.name.given.clone(),
        gender: gender_to_proto(w.gender) as i32,
        worker_type: w.worker_type.as_ref().map(std::string::ToString::to_string),
        birth_date: w.birth_date.map(|d| d.to_string()),
        tax_id: w.tax_id.clone(),
        created_at: w.created_at.to_rfc3339(),
        updated_at: w.updated_at.to_rfc3339(),
    }
}

/// Build a new domain [`Worker`] from a create-request's proto message.
/// `id`/`created_at`/`updated_at` are server-set — [`Worker::new`]
/// fills them, and any value the caller sent for them is ignored.
#[allow(clippy::result_large_err)]
fn worker_from_proto(w: proto::Worker) -> Result<Worker, Status> {
    let birth_date = w
        .birth_date
        .map(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d"))
        .transpose()
        .map_err(|_| {
            Status::invalid_argument("birth_date must be an ISO 8601 date, e.g. \"1990-01-15\"")
        })?;
    let worker_type = worker_type_from_proto(w.worker_type)?;
    let mut worker = Worker::new(
        HumanName {
            use_type: None,
            family: w.family_name,
            given: w.given_names,
            prefix: vec![],
            suffix: vec![],
        },
        gender_from_proto(w.gender),
    );
    worker.worker_type = worker_type;
    worker.birth_date = birth_date;
    worker.tax_id = w.tax_id;
    Ok(worker)
}

#[tonic::async_trait]
impl proto::worker_service_server::WorkerService for WorkerGrpcService {
    /// Validates ([`crate::validation::validate_worker`]) and real-time
    /// duplicate-checks ([`check_duplicates_internal`]) exactly as
    /// `POST /api/workers` does — the same functions, not
    /// reimplementations of them.
    async fn create_worker(
        &self,
        request: Request<proto::CreateWorkerRequest>,
    ) -> Result<Response<proto::Worker>, Status> {
        grpc_enforce(request.metadata(), Action::Write)?;

        let proto_worker = request
            .into_inner()
            .worker
            .ok_or_else(|| Status::invalid_argument("worker is required"))?;
        let mut worker = worker_from_proto(proto_worker)?;
        worker.id = Uuid::new_v4();

        let validation_errors = crate::validation::validate_worker(&worker);
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

        let duplicates = check_duplicates_internal(&self.state, &worker).await;
        if !duplicates.is_empty() {
            return Err(Status::already_exists(format!(
                "{} potential duplicate(s) found; review before creating",
                duplicates.len()
            )));
        }

        let created = self
            .state
            .worker_repository
            .create(&worker)
            .await
            .map_err(|e| Status::internal(format!("failed to create worker: {e}")))?;
        if let Err(e) = self.state.search_engine.index_worker(&created) {
            tracing::warn!("gRPC CreateWorker: failed to index in search engine: {e}");
        }
        Ok(Response::new(worker_to_proto(&created)))
    }

    /// Record-level ABAC + `mask` obligation, exactly as
    /// `GET /api/workers/{id}` (`crate::api::rest::handlers::get_worker`)
    /// applies them — same [`auth::authorize_record`] call, same
    /// [`crate::privacy::mask_worker`] on a `mask` obligation. Does
    /// **not** yet write the HIPAA §164.528 disclosure-accounting row
    /// REST does (module docs).
    async fn get_worker(
        &self,
        request: Request<proto::GetWorkerRequest>,
    ) -> Result<Response<proto::Worker>, Status> {
        let claims = grpc_enforce(request.metadata(), Action::Read)?;
        let id = parse_uuid(&request.into_inner().id)?;

        let worker = self
            .state
            .worker_repository
            .get_by_id(&id)
            .await
            .map_err(|e| Status::internal(format!("failed to retrieve worker: {e}")))?
            .ok_or_else(|| Status::not_found(format!("worker '{id}' not found")))?;

        let obligations = auth::authorize_record(
            &MaybeAuthUser(claims),
            Action::Read,
            &auth::worker_resource_attrs(&worker),
        )
        .map_err(map_record_authz_error)?;
        let body = if obligations.iter().any(|o| o == "mask") {
            crate::privacy::mask_worker(&worker)
        } else {
            worker
        };
        Ok(Response::new(worker_to_proto(&body)))
    }

    /// A blanket `Read` check only — **not** REST's per-record
    /// visibility filtering (module docs).
    async fn list_workers(
        &self,
        request: Request<proto::ListWorkersRequest>,
    ) -> Result<Response<proto::ListWorkersResponse>, Status> {
        grpc_enforce(request.metadata(), Action::Read)?;
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

        let workers = self
            .state
            .worker_repository
            .list_active(u64::from(limit), u64::from(req.offset))
            .await
            .map_err(|e| Status::internal(format!("failed to list workers: {e}")))?;
        Ok(Response::new(proto::ListWorkersResponse {
            workers: workers.iter().map(worker_to_proto).collect(),
        }))
    }

    /// Record-level ABAC exactly as `DELETE /api/workers/{id}`
    /// (`crate::api::rest::handlers::delete_worker`) applies it, then a
    /// soft delete and a best-effort search-index removal, same as REST.
    async fn delete_worker(
        &self,
        request: Request<proto::DeleteWorkerRequest>,
    ) -> Result<Response<proto::DeleteWorkerResponse>, Status> {
        let claims = grpc_enforce(request.metadata(), Action::Delete)?;
        let id = parse_uuid(&request.into_inner().id)?;

        let worker = self
            .state
            .worker_repository
            .get_by_id(&id)
            .await
            .map_err(|e| Status::internal(format!("failed to retrieve worker: {e}")))?
            .ok_or_else(|| Status::not_found(format!("worker '{id}' not found")))?;

        auth::authorize_record(
            &MaybeAuthUser(claims),
            Action::Delete,
            &auth::worker_resource_attrs(&worker),
        )
        .map_err(map_record_authz_error)?;

        self.state
            .worker_repository
            .delete(&id)
            .await
            .map_err(|e| Status::internal(format!("failed to delete worker: {e}")))?;
        if let Err(e) = self.state.search_engine.delete_worker(&id.to_string()) {
            tracing::warn!("gRPC DeleteWorker: failed to remove from search engine: {e}");
        }
        Ok(Response::new(proto::DeleteWorkerResponse {}))
    }
}
