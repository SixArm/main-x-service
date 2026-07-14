//! Cross-service entity-link endpoints — the **write side** of
//! `agents/share/cross-service-linking.md` (§4.1, §4.2) for the worker
//! service.
//!
//! Worker is a co-originator of the `same_identity` (worker ↔ person)
//! backbone edge (§9) — the federation link that resolves one human
//! across the general (person) and workforce (worker) registries and
//! powers the aggregator's `single-view`. These endpoints record an
//! **outbound** edge in this service's own `entity_links` table
//! (persistence in [`crate::db::entity_links`]). The write is
//! **optimistic**: it stores the assertion and never calls the target
//! (person) service — verification is the read-model aggregator's job.
//!
//! Authorization reuses the worker record-level ABAC guard
//! ([`authorize_record`]): a link write/read/delete is gated at the same
//! level as writing/reading the underlying worker (a no-op when
//! `WORKER_REQUIRE_AUTH` is off). Each mutation writes a best-effort
//! audit row.
//!
//! **Cross-service `linked`/`unlinked` events are emitted** (LNK-1): the
//! durable [`Envelope`](crate::streaming::Envelope) gained
//! [`EventKind::Linked`](crate::streaming::EventKind)/`Unlinked` + an
//! additive `data` field carrying the §4.2 edge detail, so under the
//! `outbox` transport the edge upsert and its `linked` (or `unlinked`)
//! event commit in one transaction (the outbox guarantee); under `memory`
//! the in-memory [`WorkerEvent::Linked`](crate::streaming::WorkerEvent) is
//! published as a lossy dev signal (it carries only the two `Uuid`s). The
//! **bulk endpoint** ([`bulk_links`]) remains the aggregator's sync
//! reconciliation path (design §8).

use std::collections::BTreeMap;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use entity_ref::{EdgeKind, EntityRef, EntityType};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use authentication_verifier::Action;

use sea_orm::TransactionTrait;
use time::OffsetDateTime;

use super::auth::{MaybeAuthUser, authorize_record, worker_resource_attrs};
use super::state::AppState;
use crate::db::AuditContext;
use crate::db::audit::AuditActor;
use crate::db::entity_links::{self, NewEdge};
use crate::db::models::entity_links::Model as EntityLinkModel;
use crate::db::outbox::OutboxInsert;
use crate::models::Worker;
use crate::streaming::{Envelope, EventKind, EventTransport, WorkerEvent, transport};

/// Body of `POST /api/workers/{id}/links`: the edge to assert from this
/// worker. `to_ref` is the far record's `EntityRef` URN (a `person:<uuid>`
/// for `same_identity`, an `organization:<uuid>` for `employed_by`);
/// `kind` is an edge-kind token (§9). `provenance` defaults to `operator`.
#[derive(Debug, Deserialize)]
pub struct LinkRequest {
    /// The edge kind token (§9); worker originates `same_identity` or
    /// `employed_by` (see `validate_edge`).
    pub kind: String,
    /// The far record's `EntityRef` URN, e.g. `person:<uuid>` or
    /// `organization:<uuid>`.
    pub to_ref: String,
    /// Optional role label — the job title for an `employed_by` edge;
    /// unused by `same_identity`.
    #[serde(default)]
    pub role: Option<String>,
    /// Optional confidence in `[0.0, 1.0]` (defaults to unset).
    #[serde(default)]
    pub confidence: Option<f64>,
    /// Optional provenance override; defaults to `operator`.
    #[serde(default)]
    pub provenance: Option<String>,
    /// Optional validity start (`YYYY-MM-DD`).
    #[serde(default)]
    pub valid_from: Option<chrono::NaiveDate>,
    /// Optional validity end (`YYYY-MM-DD`).
    #[serde(default)]
    pub valid_to: Option<chrono::NaiveDate>,
}

/// A stored edge as returned to the operator: a clean projection of the
/// `entity_links` row. `from_ref` is reconstructed as this crate's
/// `worker:<id>` URN. Distinct from the aggregator-facing [`EdgeDetail`]
/// (`edge_id` / `edge_kind` field names).
#[derive(Debug, Serialize)]
pub struct LinkView {
    /// The edge id (also the event's `edge_id`).
    pub id: String,
    /// This worker as an `EntityRef` URN (`worker:<id>`).
    pub from_ref: String,
    /// The edge kind token.
    pub kind: String,
    /// The far record's `EntityRef` URN.
    pub to_ref: String,
    /// Optional role label.
    pub role: Option<String>,
    /// Optional confidence.
    pub confidence: Option<f64>,
    /// Provenance.
    pub provenance: String,
    /// Optional validity start (`YYYY-MM-DD`).
    pub valid_from: Option<String>,
    /// Optional validity end (`YYYY-MM-DD`).
    pub valid_to: Option<String>,
}

impl LinkView {
    /// Project a stored [`EntityLinkModel`] to its operator response view.
    fn of(m: &EntityLinkModel) -> Self {
        Self {
            id: m.id.to_string(),
            from_ref: format!("worker:{}", m.from_pid),
            kind: m.kind.clone(),
            to_ref: m.to_ref.clone(),
            role: m.role.clone(),
            confidence: m.confidence,
            provenance: m.provenance.clone(),
            valid_from: date_str(m.valid_from),
            valid_to: date_str(m.valid_to),
        }
    }
}

/// The canonical §4.2 edge detail — **the shape the aggregator's
/// reconciliation pull consumes** (`edge_id` / `edge_kind` field names,
/// `from_ref` as `worker:<id>`), so the link-graph aggregator
/// deserializes it directly into its `LinkedEvent`. Distinct from the
/// operator-facing [`LinkView`] (`id` / `kind`).
#[derive(Debug, Serialize)]
pub struct EdgeDetail {
    /// The edge id.
    pub edge_id: String,
    /// This worker as an `EntityRef` URN (`worker:<id>`).
    pub from_ref: String,
    /// The far record's `EntityRef` URN.
    pub to_ref: String,
    /// The edge kind token.
    pub edge_kind: String,
    /// Optional role label.
    pub role: Option<String>,
    /// Optional confidence.
    pub confidence: Option<f64>,
    /// Provenance.
    pub provenance: String,
    /// Optional validity start (`YYYY-MM-DD`).
    pub valid_from: Option<String>,
    /// Optional validity end (`YYYY-MM-DD`).
    pub valid_to: Option<String>,
}

impl EdgeDetail {
    /// Project a stored [`EntityLinkModel`] to its canonical §4.2 shape.
    fn of(m: &EntityLinkModel) -> Self {
        Self {
            edge_id: m.id.to_string(),
            from_ref: format!("worker:{}", m.from_pid),
            to_ref: m.to_ref.clone(),
            edge_kind: m.kind.clone(),
            role: m.role.clone(),
            confidence: m.confidence,
            provenance: m.provenance.clone(),
            valid_from: date_str(m.valid_from),
            valid_to: date_str(m.valid_to),
        }
    }
}

/// Format a stored `time::Date` as a `YYYY-MM-DD` string (via the domain
/// `chrono::NaiveDate`, whose `Display` is ISO-8601).
fn date_str(d: Option<time::Date>) -> Option<String> {
    d.map(|d| crate::db::convert::time_to_date(d).to_string())
}

/// Query params for the bulk-links endpoint.
#[derive(Debug, Deserialize)]
pub struct BulkParams {
    /// Optional RFC3339 lower bound on `created_at` for an incremental
    /// pull; absent ⇒ a full replay.
    #[serde(default)]
    pub since: Option<String>,
}

/// The edge kinds worker may **originate** (§9 registry): the
/// `same_identity` backbone (worker → person) and — LNK-3 — the
/// `employed_by` affiliation (worker → organization, carries a `role`).
/// `works_at` / `member_of` are person-originated, `subject_of` is
/// case-originated, so they are not in this set.
const PERMITTED_KINDS: [EdgeKind; 2] = [EdgeKind::SameIdentity, EdgeKind::EmployedBy];

/// Validate an incoming edge: `to_ref` must parse as an [`EntityRef`],
/// `kind` must be a known [`EdgeKind`] that worker may originate
/// ([`PERMITTED_KINDS`] — `same_identity` worker → person, `employed_by`
/// worker → organization), and its endpoint pair must be permitted by the
/// §9 registry ([`EdgeKind::permits`]). Rejects (with a message the handler
/// surfaces as `422`): a kind worker does not originate (e.g. `works_at`,
/// `subject_of`), an edge pointing at the wrong target type, an unknown
/// kind, and a malformed `to_ref`. Pure and DB-free, so the accept/reject
/// matrix is unit-tested without a database.
///
/// # Errors
///
/// Returns a human-readable reason when the ref is malformed, the kind is
/// unknown, the kind is not one worker originates, or the endpoint pair is
/// not permitted for the kind.
pub fn validate_edge(kind: &str, to_ref: &str) -> Result<(EdgeKind, EntityRef), String> {
    let to = to_ref
        .parse::<EntityRef>()
        .map_err(|e| format!("invalid to_ref: {e}"))?;
    let edge_kind =
        EdgeKind::from_token(kind).ok_or_else(|| format!("unknown edge kind: {kind:?}"))?;
    if !PERMITTED_KINDS.contains(&edge_kind) {
        return Err(format!(
            "worker does not originate `{edge_kind}` edges (allowed: same_identity, employed_by)"
        ));
    }
    if !edge_kind.permits(EntityType::Worker, to.entity_type) {
        return Err(format!(
            "edge kind `{edge_kind}` does not permit worker → {} \
             (same_identity → person, employed_by → organization)",
            to.entity_type
        ));
    }
    Ok((edge_kind, to))
}

/// Build the best-effort audit context for a link mutation from the
/// verified caller (its `sub`, or `system` when unauthenticated).
fn audit_ctx(caller: &MaybeAuthUser) -> AuditContext {
    AuditContext {
        user_id: caller
            .claims()
            .map(|c| c.sub.clone())
            .or_else(|| Some("system".to_string())),
        ip_address: None,
        user_agent: None,
    }
}

/// Borrow an [`AuditContext`] as the [`AuditActor`] the audit-log helpers
/// take (worker's audit repository borrows its actor metadata).
fn actor(ctx: &AuditContext) -> AuditActor<'_> {
    AuditActor {
        user_id: ctx.user_id.as_deref(),
        ip_address: ctx.ip_address.as_deref(),
        user_agent: ctx.user_agent.as_deref(),
    }
}

/// `404` — unknown worker id.
fn not_found(id: Uuid) -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(json!({ "error": format!("Worker with id '{id}' not found") })),
    )
        .into_response()
}

/// `500` — database failure.
fn db_error(e: &crate::Error) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": format!("{e}") })),
    )
        .into_response()
}

/// Map a record-level authorization rejection `(status, reason)` to a
/// JSON error response (`403` policy-denied / `401` fail-safe).
fn rejection((status, reason): (StatusCode, String)) -> Response {
    (status, Json(json!({ "error": reason }))).into_response()
}

/// LNK-1: publish the in-memory `WorkerEvent::Linked` / `Unlinked` dev
/// signal for `link` on the `memory` transport. This is **lossy** — the
/// in-memory enum carries only two `Uuid`s, not the edge kind or the
/// target's entity type — so it exists for the `/events/recent` dev view;
/// the link-graph aggregator consumes the durable (`outbox`) path, which
/// carries the full §4.2 edge detail in the envelope `data`.
fn publish_in_memory(state: &AppState, link: &EntityLinkModel, kind: EventKind) {
    let Ok(to) = link.to_ref.parse::<EntityRef>() else {
        return;
    };
    let now = chrono::Utc::now();
    let event = match kind {
        EventKind::Linked => WorkerEvent::Linked {
            worker_id: link.from_pid,
            linked_id: to.id,
            timestamp: now,
        },
        EventKind::Unlinked => WorkerEvent::Unlinked {
            worker_id: link.from_pid,
            unlinked_id: to.id,
            timestamp: now,
        },
        _ => return,
    };
    if let Err(e) = state.event_publisher.publish(event) {
        tracing::warn!("failed to publish in-memory worker link event: {e}");
    }
}

/// LNK-1: durably enqueue a `Linked` / `Unlinked` envelope carrying the
/// §4.2 edge `data`, **on the given transaction** so the event commits
/// atomically with the edge mutation (the outbox guarantee). `kind` is
/// [`EventKind::Linked`] or [`EventKind::Unlinked`].
async fn enqueue_link_event<C: sea_orm::ConnectionTrait>(
    txn: &C,
    worker: &Worker,
    link: &EntityLinkModel,
    actor: Option<&str>,
    kind: EventKind,
) -> crate::Result<()> {
    let data = serde_json::to_value(EdgeDetail::of(link))
        .map_err(|e| crate::Error::Streaming(format!("failed to serialize edge detail: {e}")))?;
    let env = Envelope::for_link(
        &link.from_pid.to_string(),
        &worker.full_name(),
        actor,
        kind,
        data,
    );
    OutboxInsert::from_envelope(&env, OffsetDateTime::now_utc())?
        .insert_on(txn)
        .await?;
    Ok(())
}

/// Upsert an outbound edge and emit its `linked` event on the active
/// transport (LNK-1). Under `outbox` the upsert and the §4.2 `linked`
/// envelope are enqueued in one transaction; under `memory` it upserts and
/// publishes the in-memory dev signal ([`publish_in_memory`]).
async fn upsert_and_emit(
    state: &AppState,
    worker: &Worker,
    edge: &NewEdge,
    actor: Option<&str>,
) -> crate::Result<EntityLinkModel> {
    match transport() {
        EventTransport::Memory => {
            let link = entity_links::upsert(&state.db, edge).await?;
            publish_in_memory(state, &link, EventKind::Linked);
            Ok(link)
        }
        EventTransport::Outbox => {
            let txn = state.db.begin().await?;
            let link = entity_links::upsert(&txn, edge).await?;
            enqueue_link_event(&txn, worker, &link, actor, EventKind::Linked).await?;
            txn.commit().await?;
            Ok(link)
        }
    }
}

/// Soft-delete an outbound edge and emit its `unlinked` event on the active
/// transport (LNK-1), mirroring [`upsert_and_emit`].
async fn soft_delete_and_emit(
    state: &AppState,
    worker: &Worker,
    row: EntityLinkModel,
    actor: Option<&str>,
) -> crate::Result<EntityLinkModel> {
    match transport() {
        EventTransport::Memory => {
            let deleted = entity_links::soft_delete(&state.db, row).await?;
            publish_in_memory(state, &deleted, EventKind::Unlinked);
            Ok(deleted)
        }
        EventTransport::Outbox => {
            let txn = state.db.begin().await?;
            // Capture the edge detail before the row is consumed by the delete.
            let deleted = entity_links::soft_delete(&txn, row).await?;
            enqueue_link_event(&txn, worker, &deleted, actor, EventKind::Unlinked).await?;
            txn.commit().await?;
            Ok(deleted)
        }
    }
}

/// Create / upsert an outbound edge from a worker.
/// `POST /api/workers/{id}/links`.
///
/// Loads the worker (`404` if unknown), authorises at the worker-write
/// level, validates the edge (`422` otherwise), idempotently upserts the
/// `entity_links` row, emits its `linked` event (LNK-1; durably under the
/// `outbox` transport), writes a best-effort audit row, and responds `200`
/// with the stored [`LinkView`].
pub async fn create_link(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    caller: MaybeAuthUser,
    Json(req): Json<LinkRequest>,
) -> Response {
    let worker = match state.worker_repository.get_by_id(&id).await {
        Ok(Some(w)) => w,
        Ok(None) => return not_found(id),
        Err(e) => return db_error(&e),
    };
    if let Err(r) = authorize_record(&caller, Action::Write, &worker_resource_attrs(&worker)) {
        return rejection(r);
    }
    let (edge_kind, to) = match validate_edge(&req.kind, &req.to_ref) {
        Ok(pair) => pair,
        Err(reason) => {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({ "error": reason })),
            )
                .into_response();
        }
    };
    let provenance = req
        .provenance
        .filter(|p| !p.trim().is_empty())
        .unwrap_or_else(|| "operator".to_string());
    let edge = NewEdge {
        from_pid: worker.id,
        kind: edge_kind.as_str().to_string(),
        // Store the canonical URN form (normalised by `EntityRef`).
        to_ref: to.to_string(),
        role: req.role,
        confidence: req.confidence,
        provenance,
        valid_from: req.valid_from,
        valid_to: req.valid_to,
    };
    // LNK-1: upsert the edge and emit its `linked` event on the active
    // transport (durably + atomically under `outbox`; in-memory under
    // `memory`).
    let actor_sub = caller.claims().map(|c| c.sub.as_str());
    let link = match upsert_and_emit(&state, &worker, &edge, actor_sub).await {
        Ok(row) => row,
        Err(e) => return db_error(&e),
    };
    // Best-effort audit: a link write is a mutation of a worker record.
    let view = LinkView::of(&link);
    let ctx = audit_ctx(&caller);
    if let Ok(new_values) = serde_json::to_value(&view)
        && let Err(e) = state
            .audit_log
            .log_create("worker_link", link.id, new_values, &actor(&ctx))
            .await
    {
        tracing::warn!("failed to audit worker link create: {e}");
    }
    (StatusCode::OK, Json(view)).into_response()
}

/// List a worker's active outbound edges. `GET /api/workers/{id}/links`.
///
/// Loads the worker (`404` if unknown), authorises at the worker-read
/// level, and responds `200` with the active [`LinkView`]s (newest
/// first).
pub async fn list_links(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    caller: MaybeAuthUser,
) -> Response {
    let worker = match state.worker_repository.get_by_id(&id).await {
        Ok(Some(w)) => w,
        Ok(None) => return not_found(id),
        Err(e) => return db_error(&e),
    };
    if let Err(r) = authorize_record(&caller, Action::Read, &worker_resource_attrs(&worker)) {
        return rejection(r);
    }
    match entity_links::list_active(&state.db, worker.id).await {
        Ok(rows) => {
            let views: Vec<LinkView> = rows.iter().map(LinkView::of).collect();
            (StatusCode::OK, Json(views)).into_response()
        }
        Err(e) => db_error(&e),
    }
}

/// Withdraw (soft-delete) an outbound edge.
/// `DELETE /api/workers/{id}/links/{link_id}`.
///
/// Loads the worker (`404` if unknown), authorises at the worker-delete
/// level, finds the worker-scoped active edge (`404` if
/// unknown/withdrawn/other worker), soft-deletes it, emits its `unlinked`
/// event (LNK-1; durably under the `outbox` transport), writes a
/// best-effort audit row, and responds `200` with an empty JSON body.
pub async fn delete_link(
    State(state): State<AppState>,
    Path((id, link_id)): Path<(Uuid, Uuid)>,
    caller: MaybeAuthUser,
) -> Response {
    let worker = match state.worker_repository.get_by_id(&id).await {
        Ok(Some(w)) => w,
        Ok(None) => return not_found(id),
        Err(e) => return db_error(&e),
    };
    if let Err(r) = authorize_record(&caller, Action::Delete, &worker_resource_attrs(&worker)) {
        return rejection(r);
    }
    let row = match entity_links::find_active(&state.db, worker.id, link_id).await {
        Ok(Some(row)) => row,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": format!("link '{link_id}' not found") })),
            )
                .into_response();
        }
        Err(e) => return db_error(&e),
    };
    let old_values = serde_json::to_value(EdgeDetail::of(&row)).ok();
    // LNK-1: soft-delete the edge and emit its `unlinked` event on the
    // active transport (durably + atomically under `outbox`).
    let actor_sub = caller.claims().map(|c| c.sub.as_str());
    match soft_delete_and_emit(&state, &worker, row, actor_sub).await {
        Ok(deleted) => {
            let ctx = audit_ctx(&caller);
            if let Some(old_values) = old_values
                && let Err(e) = state
                    .audit_log
                    .log_delete("worker_link", deleted.id, old_values, &actor(&ctx))
                    .await
            {
                tracing::warn!("failed to audit worker link delete: {e}");
            }
            (StatusCode::OK, Json(json!({}))).into_response()
        }
        Err(e) => db_error(&e),
    }
}

/// Authorise the cross-worker **bulk** governed read (SEC-G1). Unlike the
/// per-`{id}` endpoints there is no single worker to key on, so a dump of
/// **every** `same_identity` (worker → person) edge — identity-linking PII
/// — is treated as a privileged read: [`Action::Destructive`], which the
/// built-in default policy grants only to a machine peer (`svc=true`) or an
/// `admin` caller (a deployment can grant a dedicated reconcile identity).
/// A no-op when `WORKER_REQUIRE_AUTH` is off (family default-off posture).
fn authorize_bulk(caller: &MaybeAuthUser) -> std::result::Result<(), (StatusCode, String)> {
    let (action, resource) = governed_bulk_authz();
    authorize_record(caller, action, &resource).map(|_| ())
}

/// The action + resource attributes the governed bulk read authorizes
/// with. Pure, so the security-critical classification — that a bulk dump
/// is [`Action::Destructive`] (default-deny), **not** [`Action::Read`]
/// (default-allow) — is unit-tested without a policy/DB (SEC-G1).
fn governed_bulk_authz() -> (Action, BTreeMap<String, Vec<String>>) {
    (
        Action::Destructive,
        BTreeMap::from([("governed".to_string(), vec!["same_identity".to_string()])]),
    )
}

/// `GET /api/workers/links[?since=<rfc3339>]` — every active outbound
/// edge across all workers, in the canonical §4.2 shape, for the
/// link-graph aggregator's reconciliation (design §8).
///
/// Surfaces **all** `same_identity` (worker → person) edges at once, so it
/// is gated as a privileged governed read ([`authorize_bulk`], SEC-G1): a
/// default read-only caller is refused (`403` when enforcement is on),
/// unlike the blanket guard's coarse read action. Every surfacing is
/// audited.
///
/// Returns `{ "edges": [EdgeDetail…] }`. `422` when `since` is not valid
/// RFC3339.
pub async fn bulk_links(
    State(state): State<AppState>,
    caller: MaybeAuthUser,
    Query(params): Query<BulkParams>,
) -> Response {
    if let Err(r) = authorize_bulk(&caller) {
        return rejection(r);
    }
    let since = match params.since.as_deref() {
        None => None,
        Some(s) => match chrono::DateTime::parse_from_rfc3339(s) {
            Ok(dt) => Some(crate::db::convert::ts_to_offset(
                dt.with_timezone(&chrono::Utc),
            )),
            Err(e) => {
                return (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(json!({ "error": format!("invalid `since` (want RFC3339): {e}") })),
                )
                    .into_response();
            }
        },
    };
    match entity_links::list_all_active(&state.db, since).await {
        Ok(rows) => {
            let edges: Vec<EdgeDetail> = rows.iter().map(EdgeDetail::of).collect();
            // Audit every bulk surfacing of same_identity edges. There is no
            // single worker, so the entity id is nil (a system/bulk marker).
            let ctx = audit_ctx(&caller);
            if let Err(e) = state
                .audit_log
                .log_export(
                    "worker_links_bulk",
                    Uuid::nil(),
                    json!({ "kind": "same_identity", "count": edges.len(), "since": params.since }),
                    &actor(&ctx),
                )
                .await
            {
                tracing::warn!("failed to audit worker bulk-links read: {e}");
            }
            (StatusCode::OK, Json(json!({ "edges": edges }))).into_response()
        }
        Err(e) => db_error(&e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PERSON: &str = "person:0c4f1e2a-0000-4000-8000-000000000000";
    const WORKER: &str = "worker:0c4f1e2a-0000-4000-8000-000000000001";
    const ORG: &str = "organization:0c4f1e2a-0000-4000-8000-000000000002";

    /// The `same_identity` backbone: worker → person.
    #[test]
    fn accepts_same_identity_worker_to_person() {
        let (kind, to) =
            validate_edge("same_identity", PERSON).expect("same_identity worker→person");
        assert_eq!(kind, EdgeKind::SameIdentity);
        assert_eq!(to.entity_type, EntityType::Person);
    }

    /// LNK-3: the `employed_by` affiliation, worker → organization.
    #[test]
    fn accepts_employed_by_worker_to_org() {
        let (kind, to) = validate_edge("employed_by", ORG).expect("employed_by worker→org");
        assert_eq!(kind, EdgeKind::EmployedBy);
        assert_eq!(to.entity_type, EntityType::Organization);
    }

    /// `employed_by` requires an organization target — a person / worker
    /// target is rejected by the endpoint check.
    #[test]
    fn rejects_employed_by_to_non_org() {
        assert!(validate_edge("employed_by", PERSON).is_err());
        assert!(validate_edge("employed_by", WORKER).is_err());
    }

    /// SEC-G1: the governed bulk-links read must be classified as a
    /// privileged `Destructive` action (default-deny), NOT `Read`
    /// (default-allow). A downgrade here reopens the mass PII leak, so pin
    /// it. (The full 401/403/200 behaviour is proven e2e by the case
    /// service's `bulk_links_requires_elevated_authority`, which shares this
    /// `authorize_record(Destructive)` gate.)
    #[test]
    fn governed_bulk_read_is_classified_destructive() {
        let (action, resource) = governed_bulk_authz();
        assert_eq!(
            action,
            Action::Destructive,
            "must not be Read (default-allow)"
        );
        assert_eq!(resource["governed"], vec!["same_identity".to_string()]);
    }

    /// `subject_of` (case → person) is not a worker-originated edge.
    #[test]
    fn rejects_subject_of() {
        assert!(validate_edge("subject_of", PERSON).is_err());
    }

    /// `same_identity` requires a person target — a worker (or any
    /// non-person) target is rejected by the endpoint check.
    #[test]
    fn rejects_same_identity_to_non_person() {
        assert!(validate_edge("same_identity", WORKER).is_err());
        assert!(validate_edge("same_identity", ORG).is_err());
    }

    /// Kinds worker does not originate (person-originated `works_at` /
    /// `member_of`, case-originated `subject_of`) are rejected before the
    /// endpoint check.
    #[test]
    fn rejects_kinds_worker_does_not_originate() {
        assert!(validate_edge("works_at", ORG).is_err());
        assert!(validate_edge("member_of", ORG).is_err());
        assert!(validate_edge("subject_of", PERSON).is_err());
    }

    /// A malformed `to_ref` (not a valid `EntityRef` URN) is rejected.
    #[test]
    fn rejects_malformed_to_ref() {
        for bad in ["not-a-ref", "person:", "widget:123", ""] {
            assert!(validate_edge("same_identity", bad).is_err(), "{bad:?}");
        }
    }

    /// An unknown edge-kind token is rejected before the endpoint check.
    #[test]
    fn rejects_unknown_kind() {
        assert!(validate_edge("befriends", PERSON).is_err());
    }

    /// DB-gated round-trip against a real Postgres (set `DATABASE_URL`):
    /// upsert an edge → `list_all_active` projects the canonical
    /// `edge_id` / `edge_kind` / `from_ref = worker:<id>` shape → the
    /// idempotent re-upsert keeps the same id → soft-delete removes it
    /// from the active set.
    #[tokio::test]
    #[ignore = "requires DATABASE_URL to a migrated Postgres"]
    async fn round_trip_upsert_bulk_list_delete() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
        let db = sea_orm::Database::connect(&url).await.expect("connect");
        let from_pid = Uuid::new_v4();
        let to_ref = format!("person:{}", Uuid::new_v4());
        let (kind, to) = validate_edge("same_identity", &to_ref).expect("valid");
        let edge = NewEdge {
            from_pid,
            kind: kind.as_str().to_string(),
            to_ref: to.to_string(),
            role: None,
            confidence: Some(1.0),
            provenance: "operator".to_string(),
            valid_from: None,
            valid_to: None,
        };
        let created = entity_links::upsert(&db, &edge).await.expect("upsert");
        // Idempotent re-assert keeps the same edge id.
        let reasserted = entity_links::upsert(&db, &edge).await.expect("re-upsert");
        assert_eq!(created.id, reasserted.id, "upsert is idempotent on the key");

        let all = entity_links::list_all_active(&db, None)
            .await
            .expect("bulk");
        let mine = all
            .iter()
            .find(|m| m.id == created.id)
            .expect("edge present in bulk list");
        let detail = EdgeDetail::of(mine);
        assert_eq!(detail.edge_id, created.id.to_string());
        assert_eq!(detail.edge_kind, "same_identity");
        assert_eq!(detail.from_ref, format!("worker:{from_pid}"));
        assert_eq!(detail.to_ref, to_ref);

        let found = entity_links::find_active(&db, from_pid, created.id)
            .await
            .expect("find")
            .expect("active");
        entity_links::soft_delete(&db, found).await.expect("delete");
        let after = entity_links::list_all_active(&db, None)
            .await
            .expect("bulk2");
        assert!(
            !after.iter().any(|m| m.id == created.id),
            "soft-deleted edge is gone from the active set"
        );
    }

    /// LNK-1 (DB-gated): the `outbox` emit path enqueues a `linked`
    /// `event_outbox` row **transactionally** with the edge upsert, carrying
    /// the §4.2 edge detail in the envelope payload's `data` — the shape the
    /// link-graph aggregator consumes off the bus.
    #[tokio::test]
    #[ignore = "requires DATABASE_URL to a migrated Postgres"]
    async fn linked_event_is_enqueued_to_the_outbox() {
        use crate::db::models::event_outbox;
        use crate::models::{Gender, HumanName};
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, TransactionTrait};

        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
        let db = sea_orm::Database::connect(&url).await.expect("connect");
        let from_pid = Uuid::new_v4();
        let to_ref = format!("person:{}", Uuid::new_v4());
        let (kind, to) = validate_edge("same_identity", &to_ref).expect("valid");
        let edge = NewEdge {
            from_pid,
            kind: kind.as_str().to_string(),
            to_ref: to.to_string(),
            role: None,
            confidence: Some(1.0),
            provenance: "operator".to_string(),
            valid_from: None,
            valid_to: None,
        };
        let worker = Worker::new(
            HumanName {
                use_type: None,
                family: "Smith".to_string(),
                given: vec!["John".to_string()],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Male,
        );

        // The `outbox` path: upsert + enqueue the `linked` envelope in one txn.
        let txn = db.begin().await.expect("begin");
        let link = entity_links::upsert(&txn, &edge).await.expect("upsert");
        enqueue_link_event(&txn, &worker, &link, Some("actor-1"), EventKind::Linked)
            .await
            .expect("enqueue linked event");
        txn.commit().await.expect("commit");

        // A `linked` outbox row exists for this originating worker, carrying
        // the §4.2 edge detail in its payload `data`.
        let rows = event_outbox::Entity::find()
            .filter(event_outbox::Column::EntityPid.eq(from_pid))
            .filter(event_outbox::Column::Kind.eq("linked"))
            .all(&db)
            .await
            .expect("query outbox");
        let row = rows.first().expect("a linked outbox row for the worker");
        assert_eq!(row.payload["kind"], "linked");
        assert_eq!(row.payload["data"]["edge_kind"], "same_identity");
        assert_eq!(row.payload["data"]["to_ref"], to_ref);
        assert_eq!(row.actor.as_deref(), Some("actor-1"));
    }
}
