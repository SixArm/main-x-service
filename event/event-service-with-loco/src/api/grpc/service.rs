//! [`EventGrpcService`]: the `EventService` trait implementation.
//!
//! Follows person-service's and worker-service's reference
//! implementations (`person_service::api::grpc::service`,
//! `worker_service::api::grpc::service`) for this repo's gRPC
//! rollout. Every RPC delegates to [`AppState`]'s `event_repository` /
//! `search_engine` / `matcher` — the same handles
//! [`crate::api::rest::handlers`] calls — and to the shared validation
//! ([`crate::validation::validate_event`]) and duplicate-detection
//! ([`crate::api::rest::handlers::check_duplicates_internal`])
//! modules. Nothing here reimplements a REST business rule; this file
//! is wire conversion (`proto::Event` ⇄ [`crate::models::Event`]) plus
//! RPC-shaped error mapping (`tonic::Status` codes instead of HTTP
//! status codes) around calls into that shared code.
//!
//! ## Auth parity with REST
//!
//! [`grpc_enforce`] is the gRPC counterpart of
//! [`crate::api::rest::auth::require_auth_mw`] (REST's blanket-guard
//! middleware): `EVENT_REQUIRE_AUTH` off ⇒ every RPC is open, exactly
//! today's REST default; on ⇒ a missing/invalid bearer (in the
//! `authorization` gRPC metadata entry, `Bearer <paseto>`) is
//! `UNAUTHENTICATED` and a denied ABAC policy decision is
//! `PERMISSION_DENIED` — the same 401/403 split REST makes, in gRPC's
//! terms. Unlike person's and worker's gRPC surfaces, there is no
//! record-level ABAC pass here either: this crate's own REST handlers
//! (`create_event`/`get_event`/`delete_event`) apply only the blanket
//! guard too, with no per-record `authorize_record` call to mirror —
//! confirmed by reading them, not assumed absent.
//!
//! What is **not** carried over to this slice, and is tracked rather
//! than silently absent (this crate's `AGENTS.md` gRPC section):
//! `UpdateEvent` (no RPC for it yet); and `ListEvents` calls
//! [`crate::db::EventRepository::list_active`] directly since this
//! crate has no REST list endpoint to mirror at all.

use uuid::Uuid;

use authentication_verifier::{Action, Claims};
use tonic::metadata::MetadataMap;
use tonic::{Request, Response, Status};

use super::proto;
use crate::api::rest::AppState;
use crate::api::rest::auth;
use crate::api::rest::handlers::check_duplicates_internal;
use crate::models::{Event, EventStatus};

/// Default `ListEvents` page size when the caller sends `limit: 0`,
/// and the ceiling any larger request is clamped to — matching the
/// family's usual `GET /api/<plural>` defaults, even though this
/// crate has no REST list endpoint of its own to copy them from.
const DEFAULT_LIST_LIMIT: u32 = 10;
const MAX_LIST_LIMIT: u32 = 100;

/// Same offset ceiling as the family's usual SEC-G7 bound on other
/// crates' list/search endpoints — declared here since this crate has
/// no existing list endpoint to share the constant with. Unbounded
/// paging materialises arbitrarily many rows only to skip them, which
/// is a cheap denial of service regardless of transport.
const MAX_LIST_OFFSET: u32 = 10_000;

/// The `EventService` gRPC handler. Thin — just the shared
/// [`AppState`], cloned in from the same value `App::after_routes`
/// builds for the REST router (`src/app.rs`), so both surfaces share
/// one database pool, one search index, one matcher, one event
/// publisher.
pub struct EventGrpcService {
    state: AppState,
}

impl EventGrpcService {
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
/// counterpart of REST's `require_auth_mw`. See the module docs'
/// "Auth parity with REST" section for the exact contract.
#[allow(clippy::result_large_err)]
fn grpc_enforce(metadata: &MetadataMap, action: Action) -> Result<(), Status> {
    if !auth::require_auth_from_env() {
        // Enforcement off: still surface a presented-but-invalid token
        // as an error instead of quietly treating the call as
        // anonymous — a caller that sent a bad credential almost
        // certainly meant to authenticate.
        grpc_bearer_claims(metadata)?;
        return Ok(());
    }
    let Some(claims) = grpc_bearer_claims(metadata)? else {
        return Err(Status::unauthenticated("missing authorization metadata"));
    };
    let decision = auth::policy()
        .current()
        .evaluate(&claims, action, auth::ENTITY);
    if decision.allowed {
        Ok(())
    } else {
        Err(Status::permission_denied(decision.reason))
    }
}

/// Parse an `event_status` wire token (e.g. `"scheduled"`) into the
/// domain enum by reusing its existing `serde` implementation
/// (`#[serde(rename_all = "snake_case")]`) rather than hand-rolling a
/// second mapping that could drift from it. `EventStatus` has no
/// `Display` impl (unlike `WorkerType`), so both directions go through
/// `serde_json` for consistency.
#[allow(clippy::result_large_err)]
fn event_status_from_proto(raw: Option<String>) -> Result<Option<EventStatus>, Status> {
    raw.map(|s| {
        serde_json::from_value(serde_json::Value::String(s.clone())).map_err(|_| {
            Status::invalid_argument(format!("'{s}' is not a recognised event_status"))
        })
    })
    .transpose()
}

/// The inverse of [`event_status_from_proto`]: the domain enum's
/// `serde` `snake_case` token, via the same round-trip.
fn event_status_to_proto(status: EventStatus) -> String {
    match serde_json::to_value(status) {
        Ok(serde_json::Value::String(s)) => s,
        // `EventStatus` is a unit-only enum with a string serde form;
        // any other shape would be a change to that type this
        // function was not updated for.
        _ => unreachable!("EventStatus must serialize to a JSON string"),
    }
}

/// Project the persisted domain record onto the (deliberately partial
/// — see the module docs) proto message.
fn event_to_proto(e: &Event) -> proto::Event {
    proto::Event {
        id: e.id.to_string(),
        active: e.active,
        name: e.name.clone(),
        start_date: e.start_date.to_rfc3339(),
        end_date: e.end_date.map(|d| d.to_rfc3339()),
        event_status: Some(event_status_to_proto(e.event_status)),
        created_at: e.created_at.to_rfc3339(),
        updated_at: e.updated_at.to_rfc3339(),
    }
}

/// Build a new domain [`Event`] from a create-request's proto message.
/// `id`/`created_at`/`updated_at` are server-set — [`Event::new`]
/// fills them, and any value the caller sent for them is ignored.
#[allow(clippy::result_large_err)]
fn event_from_proto(e: proto::Event) -> Result<Event, Status> {
    let start_date = chrono::DateTime::parse_from_rfc3339(&e.start_date)
        .map(|d| d.with_timezone(&chrono::Utc))
        .map_err(|_| {
            Status::invalid_argument(
                "start_date must be an RFC 3339 timestamp, e.g. \"2026-01-15T09:00:00Z\"",
            )
        })?;
    let end_date = e
        .end_date
        .map(|s| chrono::DateTime::parse_from_rfc3339(&s).map(|d| d.with_timezone(&chrono::Utc)))
        .transpose()
        .map_err(|_| Status::invalid_argument("end_date must be an RFC 3339 timestamp"))?;
    let event_status = event_status_from_proto(e.event_status)?;

    let mut event = Event::new(e.name, start_date);
    event.end_date = end_date;
    if let Some(status) = event_status {
        event.event_status = status;
    }
    Ok(event)
}

#[tonic::async_trait]
impl proto::event_service_server::EventService for EventGrpcService {
    /// Validates ([`crate::validation::validate_event`]) and real-time
    /// duplicate-checks ([`check_duplicates_internal`]) exactly as
    /// `POST /api/events` does — the same functions, not
    /// reimplementations of them.
    async fn create_event(
        &self,
        request: Request<proto::CreateEventRequest>,
    ) -> Result<Response<proto::Event>, Status> {
        grpc_enforce(request.metadata(), Action::Write)?;

        let proto_event = request
            .into_inner()
            .event
            .ok_or_else(|| Status::invalid_argument("event is required"))?;
        let event = event_from_proto(proto_event)?;

        let validation_errors = crate::validation::validate_event(&event);
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

        let duplicates = check_duplicates_internal(&self.state, &event).await;
        if !duplicates.is_empty() {
            return Err(Status::already_exists(format!(
                "{} potential duplicate(s) found; review before creating",
                duplicates.len()
            )));
        }

        let created = self
            .state
            .event_repository
            .create(&event)
            .await
            .map_err(|e| Status::internal(format!("failed to create event: {e}")))?;
        if let Err(e) = self.state.search_engine.index_event(&created) {
            tracing::warn!("gRPC CreateEvent: failed to index in search engine: {e}");
        }
        Ok(Response::new(event_to_proto(&created)))
    }

    async fn get_event(
        &self,
        request: Request<proto::GetEventRequest>,
    ) -> Result<Response<proto::Event>, Status> {
        grpc_enforce(request.metadata(), Action::Read)?;
        let id = parse_uuid(&request.into_inner().id)?;

        let event = self
            .state
            .event_repository
            .get_by_id(&id)
            .await
            .map_err(|e| Status::internal(format!("failed to retrieve event: {e}")))?
            .ok_or_else(|| Status::not_found(format!("event '{id}' not found")))?;
        Ok(Response::new(event_to_proto(&event)))
    }

    /// Calls `EventRepository::list_active` directly — this crate has
    /// no REST list endpoint to mirror, but the repository method
    /// itself is real, already-tested domain logic.
    async fn list_events(
        &self,
        request: Request<proto::ListEventsRequest>,
    ) -> Result<Response<proto::ListEventsResponse>, Status> {
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

        let events = self
            .state
            .event_repository
            .list_active(u64::from(limit), u64::from(req.offset))
            .await
            .map_err(|e| Status::internal(format!("failed to list events: {e}")))?;
        Ok(Response::new(proto::ListEventsResponse {
            events: events.iter().map(event_to_proto).collect(),
        }))
    }

    /// Soft-delete, exactly as `DELETE /api/events/{id}` does, then a
    /// best-effort search-index removal.
    async fn delete_event(
        &self,
        request: Request<proto::DeleteEventRequest>,
    ) -> Result<Response<proto::DeleteEventResponse>, Status> {
        grpc_enforce(request.metadata(), Action::Delete)?;
        let id = parse_uuid(&request.into_inner().id)?;

        self.state
            .event_repository
            .get_by_id(&id)
            .await
            .map_err(|e| Status::internal(format!("failed to retrieve event: {e}")))?
            .ok_or_else(|| Status::not_found(format!("event '{id}' not found")))?;

        self.state
            .event_repository
            .delete(&id)
            .await
            .map_err(|e| Status::internal(format!("failed to delete event: {e}")))?;
        if let Err(e) = self.state.search_engine.delete_event(&id.to_string()) {
            tracing::warn!("gRPC DeleteEvent: failed to remove from search engine: {e}");
        }
        Ok(Response::new(proto::DeleteEventResponse {}))
    }
}
