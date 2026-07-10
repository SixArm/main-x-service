//! Cross-service entity-link endpoints — the **write side** of
//! `agents/share/cross-service-linking.md` (§4.1, §4.2) for the person
//! service.
//!
//! Person is the reference originator of the `same_identity`
//! (person ↔ worker) backbone edge (§9) — the federation link that
//! resolves one human across the general (person) and workforce (worker)
//! registries and powers the aggregator's `single-view`. These endpoints
//! record an **outbound** edge in this service's own `entity_links` table
//! (persistence in [`crate::db::entity_links`]). The write is
//! **optimistic**: it stores the assertion and never calls the target
//! (worker) service — verification is the read-model aggregator's job.
//!
//! Authorization reuses the person record-level ABAC guard
//! ([`authorize_record`]): a link write/read/delete is gated at the same
//! level as writing/reading the underlying person (a no-op when
//! `PERSON_REQUIRE_AUTH` is off). Each mutation writes a best-effort
//! audit row.
//!
//! **Cross-service `linked`/`unlinked` event emission is deferred** (see
//! [`crate::streaming`]): the durable [`Envelope`](crate::streaming::Envelope)
//! has no link kind + no `data` payload, and the in-memory
//! [`PersonEvent::Linked`](crate::streaming::PersonEvent) carries only
//! person `Uuid`s (no `to_ref` / `edge_kind` / provenance), so neither
//! can carry the §4.2 edge `data` without a cross-cutting refactor. The
//! **bulk endpoint** ([`bulk_links`]) is the aggregator's sync path
//! (design §8), so the deliverable does not block on the event shape.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use entity_ref::{EdgeKind, EntityRef, EntityType};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use authentication_verifier::Action;

use super::auth::{MaybeAuthUser, authorize_record, person_resource_attrs};
use super::state::AppState;
use crate::db::AuditContext;
use crate::db::entity_links::{self, NewEdge};
use crate::db::models::entity_links::Model as EntityLinkModel;

/// Body of `POST /api/persons/{id}/links`: the edge to assert from this
/// person. `to_ref` is the far record's `EntityRef` URN (for v1 a
/// `worker:<uuid>`); `kind` is an edge-kind token (§9). `provenance`
/// defaults to `operator`.
#[derive(Debug, Deserialize)]
pub struct LinkRequest {
    /// The edge kind token (§9); for v1 person originates only
    /// `same_identity`.
    pub kind: String,
    /// The far record's `EntityRef` URN, e.g. `worker:<uuid>`.
    pub to_ref: String,
    /// Optional role label (unused by `same_identity`).
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
/// `person:<id>` URN. Distinct from the aggregator-facing [`EdgeDetail`]
/// (`edge_id` / `edge_kind` field names).
#[derive(Debug, Serialize)]
pub struct LinkView {
    /// The edge id (also the event's `edge_id`).
    pub id: String,
    /// This person as an `EntityRef` URN (`person:<id>`).
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
            from_ref: format!("person:{}", m.from_pid),
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
/// `from_ref` as `person:<id>`), so the link-graph aggregator
/// deserializes it directly into its `LinkedEvent`. Distinct from the
/// operator-facing [`LinkView`] (`id` / `kind`).
#[derive(Debug, Serialize)]
pub struct EdgeDetail {
    /// The edge id.
    pub edge_id: String,
    /// This person as an `EntityRef` URN (`person:<id>`).
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
            from_ref: format!("person:{}", m.from_pid),
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

/// Validate an incoming edge: `to_ref` must parse as an [`EntityRef`],
/// `kind` must be a known [`EdgeKind`], and — for v1 — the kind must be
/// `same_identity` with a `person → worker` endpoint pair (§9). Rejects
/// (with a message the handler surfaces as `422`): a non-`same_identity`
/// kind (e.g. `subject_of`, `works_at`), a `same_identity` pointing at a
/// non-worker, an unknown kind, and a malformed `to_ref`. Pure and
/// DB-free, so the accept/reject matrix is unit-tested without a database.
///
/// # Errors
///
/// Returns a human-readable reason when the ref is malformed, the kind is
/// unknown, the kind is not `same_identity`, or the endpoint pair is not
/// `person → worker`.
pub fn validate_edge(kind: &str, to_ref: &str) -> Result<(EdgeKind, EntityRef), String> {
    let to = to_ref
        .parse::<EntityRef>()
        .map_err(|e| format!("invalid to_ref: {e}"))?;
    let edge_kind =
        EdgeKind::from_token(kind).ok_or_else(|| format!("unknown edge kind: {kind:?}"))?;
    if edge_kind != EdgeKind::SameIdentity {
        return Err(format!(
            "person originates only `same_identity` edges, not `{edge_kind}`"
        ));
    }
    if !edge_kind.permits(EntityType::Person, to.entity_type) {
        return Err(format!(
            "edge kind `{edge_kind}` does not permit person → {} (same_identity links person ↔ worker)",
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

/// `404` — unknown person id.
fn not_found(id: Uuid) -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(json!({ "error": format!("Person with id '{id}' not found") })),
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

/// Create / upsert an outbound edge from a person.
/// `POST /api/persons/{id}/links`.
///
/// Loads the person (`404` if unknown), authorises at the person-write
/// level, validates the edge (`422` otherwise), idempotently upserts the
/// `entity_links` row, writes a best-effort audit row, and responds `200`
/// with the stored [`LinkView`]. (Cross-service event emission is
/// deferred — see the module docs.)
pub async fn create_link(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    caller: MaybeAuthUser,
    Json(req): Json<LinkRequest>,
) -> Response {
    let person = match state.person_repository.get_by_id(&id).await {
        Ok(Some(p)) => p,
        Ok(None) => return not_found(id),
        Err(e) => return db_error(&e),
    };
    if let Err(r) = authorize_record(&caller, Action::Write, &person_resource_attrs(&person)) {
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
        from_pid: person.id,
        kind: edge_kind.as_str().to_string(),
        // Store the canonical URN form (normalised by `EntityRef`).
        to_ref: to.to_string(),
        role: req.role,
        confidence: req.confidence,
        provenance,
        valid_from: req.valid_from,
        valid_to: req.valid_to,
    };
    let link = match entity_links::upsert(&state.db, &edge).await {
        Ok(row) => row,
        Err(e) => return db_error(&e),
    };
    // Best-effort audit: a link write is a mutation of a person record.
    let view = LinkView::of(&link);
    if let Ok(new_values) = serde_json::to_value(&view)
        && let Err(e) = state
            .audit_log
            .log_create("person_link", link.id, new_values, &audit_ctx(&caller))
            .await
    {
        tracing::warn!("failed to audit person link create: {e}");
    }
    (StatusCode::OK, Json(view)).into_response()
}

/// List a person's active outbound edges. `GET /api/persons/{id}/links`.
///
/// Loads the person (`404` if unknown), authorises at the person-read
/// level, and responds `200` with the active [`LinkView`]s (newest
/// first).
pub async fn list_links(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    caller: MaybeAuthUser,
) -> Response {
    let person = match state.person_repository.get_by_id(&id).await {
        Ok(Some(p)) => p,
        Ok(None) => return not_found(id),
        Err(e) => return db_error(&e),
    };
    if let Err(r) = authorize_record(&caller, Action::Read, &person_resource_attrs(&person)) {
        return rejection(r);
    }
    match entity_links::list_active(&state.db, person.id).await {
        Ok(rows) => {
            let views: Vec<LinkView> = rows.iter().map(LinkView::of).collect();
            (StatusCode::OK, Json(views)).into_response()
        }
        Err(e) => db_error(&e),
    }
}

/// Withdraw (soft-delete) an outbound edge.
/// `DELETE /api/persons/{id}/links/{link_id}`.
///
/// Loads the person (`404` if unknown), authorises at the person-delete
/// level, finds the person-scoped active edge (`404` if
/// unknown/withdrawn/other person), soft-deletes it, writes a best-effort
/// audit row, and responds `200` with an empty JSON body. (Cross-service
/// event emission is deferred — see the module docs.)
pub async fn delete_link(
    State(state): State<AppState>,
    Path((id, link_id)): Path<(Uuid, Uuid)>,
    caller: MaybeAuthUser,
) -> Response {
    let person = match state.person_repository.get_by_id(&id).await {
        Ok(Some(p)) => p,
        Ok(None) => return not_found(id),
        Err(e) => return db_error(&e),
    };
    if let Err(r) = authorize_record(&caller, Action::Delete, &person_resource_attrs(&person)) {
        return rejection(r);
    }
    let row = match entity_links::find_active(&state.db, person.id, link_id).await {
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
    match entity_links::soft_delete(&state.db, row).await {
        Ok(deleted) => {
            if let Some(old_values) = old_values
                && let Err(e) = state
                    .audit_log
                    .log_delete("person_link", deleted.id, old_values, &audit_ctx(&caller))
                    .await
            {
                tracing::warn!("failed to audit person link delete: {e}");
            }
            (StatusCode::OK, Json(json!({}))).into_response()
        }
        Err(e) => db_error(&e),
    }
}

/// `GET /api/persons/links[?since=<rfc3339>]` — every active outbound
/// edge across all persons, in the canonical §4.2 shape, for the
/// link-graph aggregator's reconciliation (design §8). Read-only; gated
/// by the blanket guard's read action.
///
/// Returns `{ "edges": [EdgeDetail…] }`. `422` when `since` is not valid
/// RFC3339.
pub async fn bulk_links(
    State(state): State<AppState>,
    Query(params): Query<BulkParams>,
) -> Response {
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
            (StatusCode::OK, Json(json!({ "edges": edges }))).into_response()
        }
        Err(e) => db_error(&e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORKER: &str = "worker:0c4f1e2a-0000-4000-8000-000000000000";
    const PERSON: &str = "person:0c4f1e2a-0000-4000-8000-000000000001";
    const ORG: &str = "organization:0c4f1e2a-0000-4000-8000-000000000002";

    /// The one accepted combination: `same_identity` person → worker.
    #[test]
    fn accepts_same_identity_person_to_worker() {
        let (kind, to) =
            validate_edge("same_identity", WORKER).expect("same_identity person→worker");
        assert_eq!(kind, EdgeKind::SameIdentity);
        assert_eq!(to.entity_type, EntityType::Worker);
    }

    /// `subject_of` (case → person) is not a person-originated edge.
    #[test]
    fn rejects_subject_of() {
        assert!(validate_edge("subject_of", PERSON).is_err());
    }

    /// `same_identity` requires a worker target — a person (or any
    /// non-worker) target is rejected by the endpoint check.
    #[test]
    fn rejects_same_identity_to_non_worker() {
        assert!(validate_edge("same_identity", PERSON).is_err());
        assert!(validate_edge("same_identity", ORG).is_err());
    }

    /// `works_at` (person → org) is a real registry kind but not shipped
    /// on the person side in v1 — rejected as a non-`same_identity` kind.
    #[test]
    fn rejects_non_same_identity_kind() {
        assert!(validate_edge("works_at", ORG).is_err());
        assert!(validate_edge("member_of", ORG).is_err());
    }

    /// A malformed `to_ref` (not a valid `EntityRef` URN) is rejected.
    #[test]
    fn rejects_malformed_to_ref() {
        for bad in ["not-a-ref", "worker:", "widget:123", ""] {
            assert!(validate_edge("same_identity", bad).is_err(), "{bad:?}");
        }
    }

    /// An unknown edge-kind token is rejected before the endpoint check.
    #[test]
    fn rejects_unknown_kind() {
        assert!(validate_edge("befriends", WORKER).is_err());
    }

    /// DB-gated round-trip against a real Postgres (set `DATABASE_URL`):
    /// upsert an edge → `list_all_active` projects the canonical
    /// `edge_id` / `edge_kind` / `from_ref = person:<id>` shape → the
    /// idempotent re-upsert keeps the same id → soft-delete removes it
    /// from the active set.
    #[tokio::test]
    #[ignore = "requires DATABASE_URL to a migrated Postgres"]
    async fn round_trip_upsert_bulk_list_delete() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
        let db = sea_orm::Database::connect(&url).await.expect("connect");
        let from_pid = Uuid::new_v4();
        let to_ref = format!("worker:{}", Uuid::new_v4());
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
        assert_eq!(detail.from_ref, format!("person:{from_pid}"));
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
}
