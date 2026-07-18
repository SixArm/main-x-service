//! Cross-service entity-link endpoints — the **write side** of
//! `agents/share/cross-service-linking.md` (§4.1, §4.2) for the case
//! service.
//!
//! Case is the reference originator: it owns the `subject_of` (case →
//! person) edge (§9), the highest-governance v1 kind (§10). These
//! endpoints record an **outbound** edge in this service's own
//! `entity_links` table and emit a `linked` / `unlinked` event on the
//! existing event stream — the write is **optimistic**: it stores the
//! assertion and never calls the target service (verification is the
//! aggregator's job).
//!
//! Governance (§10): both creating and reading an edge are authorised at
//! the *same* level as reading the underlying case (via
//! [`crate::auth::authorize_record`] on the loaded case), and every
//! write is audited (`linked` / `unlinked` action). A per-record masking
//! pass and a "denied read is indistinguishable from no-such-edge"
//! refinement are follow-ups (spec §13); v1 leans on the blanket
//! write/destructive guard plus the case-level record check.

use std::collections::BTreeMap;

use authentication_verifier::Action;
use axum::http::StatusCode;
use entity_ref::{EdgeKind, EntityRef, EntityType};
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::auth::MaybeAuthUser;
use crate::models::audit_logs::Model as AuditModel;
use crate::models::cases::Model as CaseModel;
use crate::models::entity_links::{Model as EntityLinkModel, NewEdge};
use crate::streaming;

/// Body of `POST /api/cases/{pid}/links`: the edge to assert from this
/// case. `to_ref` is the far record's `EntityRef` URN (e.g.
/// `person:<uuid>`); `kind` is an edge-kind token (§9). `provenance`
/// defaults to `operator`.
#[derive(Debug, Deserialize)]
struct LinkRequest {
    /// The edge kind token (§9), e.g. `subject_of`.
    kind: String,
    /// The far record's `EntityRef` URN, e.g. `person:<uuid>`.
    to_ref: String,
    /// Optional role label (unused by `subject_of`).
    #[serde(default)]
    role: Option<String>,
    /// Optional confidence in `[0.0, 1.0]` (defaults to unset).
    #[serde(default)]
    confidence: Option<f64>,
    /// Optional provenance override; defaults to `operator`.
    #[serde(default)]
    provenance: Option<String>,
    /// Optional validity start (`YYYY-MM-DD`).
    #[serde(default)]
    valid_from: Option<chrono::NaiveDate>,
    /// Optional validity end (`YYYY-MM-DD`).
    #[serde(default)]
    valid_to: Option<chrono::NaiveDate>,
}

/// A stored edge as returned to the caller: a clean projection of the
/// `entity_links` row (no internal timestamps beyond `created`-facing
/// data). `from_ref` is reconstructed as this crate's `case:<pid>` URN.
#[derive(Debug, Serialize)]
struct LinkView {
    /// The edge id (also the event's `edge_id`).
    id: String,
    /// This case as an `EntityRef` URN (`case:<pid>`).
    from_ref: String,
    /// The edge kind token.
    kind: String,
    /// The far record's `EntityRef` URN.
    to_ref: String,
    /// Optional role label.
    role: Option<String>,
    /// Optional confidence.
    confidence: Option<f64>,
    /// Provenance.
    provenance: String,
    /// Optional validity start (`YYYY-MM-DD`).
    valid_from: Option<String>,
    /// Optional validity end (`YYYY-MM-DD`).
    valid_to: Option<String>,
}

impl LinkView {
    /// Project a stored [`EntityLinkModel`] to its response view.
    fn of(m: &EntityLinkModel) -> Self {
        Self {
            id: m.id.to_string(),
            from_ref: format!("case:{}", m.from_pid),
            kind: m.kind.clone(),
            to_ref: m.to_ref.clone(),
            role: m.role.clone(),
            confidence: m.confidence,
            provenance: m.provenance.clone(),
            valid_from: m.valid_from.map(|d| d.to_string()),
            valid_to: m.valid_to.map(|d| d.to_string()),
        }
    }
}

/// The canonical §4.2 edge detail — **the same shape the `linked` /
/// `unlinked` events carry** (`edge_id` / `edge_kind` field names, `from_ref`
/// as `case:<pid>`), so the link-graph aggregator deserializes it directly
/// into its `LinkedEvent` for reconciliation (design §8). Distinct from the
/// operator-facing [`LinkView`] (`id` / `kind`).
#[derive(Debug, Serialize)]
struct EdgeDetail {
    edge_id: String,
    from_ref: String,
    to_ref: String,
    edge_kind: String,
    role: Option<String>,
    confidence: Option<f64>,
    provenance: String,
    valid_from: Option<String>,
    valid_to: Option<String>,
}

impl EdgeDetail {
    fn of(m: &EntityLinkModel) -> Self {
        Self {
            edge_id: m.id.to_string(),
            from_ref: format!("case:{}", m.from_pid),
            to_ref: m.to_ref.clone(),
            edge_kind: m.kind.clone(),
            role: m.role.clone(),
            confidence: m.confidence,
            provenance: m.provenance.clone(),
            valid_from: m.valid_from.map(|d| d.to_string()),
            valid_to: m.valid_to.map(|d| d.to_string()),
        }
    }
}

/// Query params for the bulk-links endpoint.
#[derive(Debug, Deserialize)]
struct BulkParams {
    /// Optional RFC3339 lower bound on `created_at` for an incremental
    /// pull; absent ⇒ a full replay.
    #[serde(default)]
    since: Option<String>,
}

/// Validate an incoming edge: `to_ref` must parse as an [`EntityRef`],
/// `kind` must be a known [`EdgeKind`], and the kind must **permit** a
/// `case → <to_ref.entity_type>` endpoint pair (§9). For the case
/// service this admits exactly `subject_of` (case → person) and rejects
/// everything else (`same_identity`, a `subject_of` pointing at a
/// non-person, a person→case direction, a malformed ref) with a message
/// the handler surfaces as `422`. Pure and DB-free, so the accept/reject
/// matrix is unit-tested without a database.
///
/// # Errors
///
/// Returns a human-readable reason when the ref is malformed, the kind is
/// unknown, or the kind does not permit `case → to`.
pub fn validate_edge(
    kind: &str,
    to_ref: &str,
) -> std::result::Result<(EdgeKind, EntityRef), String> {
    let to = to_ref
        .parse::<EntityRef>()
        .map_err(|e| format!("invalid to_ref: {e}"))?;
    let edge_kind =
        EdgeKind::from_token(kind).ok_or_else(|| format!("unknown edge kind: {kind:?}"))?;
    if !edge_kind.permits(EntityType::Case, to.entity_type) {
        return Err(format!(
            "edge kind `{edge_kind}` does not permit case → {} (case originates only `subject_of` → person)",
            to.entity_type
        ));
    }
    Ok((edge_kind, to))
}

/// Map a validation reason to a `422` loco error.
fn unprocessable(reason: &str) -> Error {
    Error::CustomError(
        StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("validation", reason),
    )
}

/// Map a record-level authorization rejection (`(status, reason)`) to a
/// loco error response — `403` policy-denied, `401` fail-safe (mirrors
/// the cases controller).
fn record_rejection((status, reason): (StatusCode, String)) -> Error {
    let code = if status == StatusCode::FORBIDDEN {
        "forbidden"
    } else {
        "unauthorized"
    };
    Error::CustomError(status, ErrorDetail::new(code, &reason))
}

/// Authorise an action on the edges of a case at the **read-the-case**
/// level (§10) — loads no extra state, reuses the case's resource
/// attributes. A no-op when enforcement is off.
fn authorize_case(caller: &MaybeAuthUser, action: Action, case: &CaseModel) -> Result<()> {
    let attrs = crate::auth::case_resource_attrs(&case.to_case()?);
    crate::auth::authorize_record(caller, action, &attrs).map_err(record_rejection)?;
    Ok(())
}

/// Create / upsert an outbound edge from a case. `POST
/// /api/cases/{pid}/links`.
///
/// Loads the case (404 if unknown), authorises at the case-write level,
/// validates the edge (`422` otherwise), idempotently upserts the
/// `entity_links` row, emits a `linked` event carrying the edge detail,
/// and audits the write. Responds `200` with the stored edge.
///
/// # Errors
///
/// `404` unknown `pid`; `403`/`401` when the record-level policy denies;
/// `422` when the edge is invalid; a DB error on the upsert.
#[debug_handler]
async fn create_link(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(req): Json<LinkRequest>,
) -> Result<Response> {
    let case = CaseModel::find_by_pid(&ctx.db, &pid).await.map_err(super::model_not_found)?;
    authorize_case(&caller, Action::Write, &case)?;
    let (edge_kind, to) =
        validate_edge(&req.kind, &req.to_ref).map_err(|reason| unprocessable(&reason))?;
    let provenance = req
        .provenance
        .filter(|p| !p.trim().is_empty())
        .unwrap_or_else(|| "operator".to_string());
    let edge = NewEdge {
        from_pid: case.pid,
        kind: edge_kind.as_str().to_string(),
        // Store the canonical URN form (normalised by `EntityRef`).
        to_ref: to.to_string(),
        role: req.role,
        confidence: req.confidence,
        provenance,
        valid_from: req.valid_from,
        valid_to: req.valid_to,
    };
    let link = streaming::link_and_emit(&ctx.db, &edge, &case.title, caller.actor()).await?;
    format::json(LinkView::of(&link))
}

/// List a case's active outbound edges. `GET /api/cases/{pid}/links`.
///
/// Loads the case (404 if unknown), authorises at the case-read level,
/// and responds `200` with the active [`LinkView`]s (newest first).
///
/// # Errors
///
/// `404` unknown `pid`; `403`/`401` when the record-level policy denies;
/// a DB error on the query.
#[debug_handler]
async fn list_links(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let case = CaseModel::find_by_pid(&ctx.db, &pid).await.map_err(super::model_not_found)?;
    authorize_case(&caller, Action::Read, &case)?;
    let rows = EntityLinkModel::list_active(&ctx.db, case.pid).await?;
    let views: Vec<LinkView> = rows.iter().map(LinkView::of).collect();
    format::json(views)
}

/// Withdraw (soft-delete) an outbound edge. `DELETE
/// /api/cases/{pid}/links/{id}`.
///
/// Loads the case (404 if unknown), authorises at the case-delete level,
/// finds the case-scoped active edge (404 if unknown/withdrawn/other
/// case), soft-deletes it, emits an `unlinked` event, and audits the
/// withdrawal. Responds `200` with an empty JSON body.
///
/// # Errors
///
/// `400` malformed link id; `404` unknown `pid` or edge; `403`/`401` when
/// the record-level policy denies; a DB error on the soft-delete.
#[debug_handler]
async fn delete_link(
    Path((pid, id)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let case = CaseModel::find_by_pid(&ctx.db, &pid).await.map_err(super::model_not_found)?;
    authorize_case(&caller, Action::Delete, &case)?;
    let Ok(edge_id) = uuid::Uuid::parse_str(&id) else {
        return bad_request("invalid link id");
    };
    let link = EntityLinkModel::find_active(&ctx.db, case.pid, edge_id).await?;
    streaming::unlink_and_emit(&ctx.db, link, &case.title, caller.actor()).await?;
    format::empty_json()
}

/// Authorise the cross-case **bulk** governed read (SEC-G1 / design §10).
/// Unlike the per-`{pid}` endpoints there is no single case to key on, so a
/// dump of **every** `subject_of` (case → person) edge is treated as a
/// privileged read: [`Action::Destructive`], which the built-in default
/// policy grants only to a machine peer (`svc=true`) or an `admin` caller.
/// A deployment can grant a dedicated reconcile identity via policy. A
/// no-op when `CASE_REQUIRE_AUTH` is off (the family default-off posture —
/// tracked as SEC-G8), matching every other authorisation in the service.
fn authorize_bulk(caller: &MaybeAuthUser) -> Result<()> {
    let resource: BTreeMap<String, Vec<String>> =
        BTreeMap::from([("governed".to_string(), vec!["subject_of".to_string()])]);
    crate::auth::authorize_record(caller, Action::Destructive, &resource)
        .map_err(record_rejection)?;
    Ok(())
}

/// `GET /api/cases/links[?since=<rfc3339>]` — every active outbound edge
/// across all cases, in the canonical §4.2 shape, for the link-graph
/// aggregator's reconciliation (design §8).
///
/// This surfaces **all** high-sensitivity `subject_of` (case → person)
/// edges at once, so it is gated as a privileged governed read
/// ([`authorize_bulk`], SEC-G1): a default read-only caller is refused
/// (`403` when enforcement is on) even though the blanket guard's coarse
/// read action would admit it. Every surfacing is audited (§10).
///
/// # Errors
///
/// `403`/`401` when the caller is not authorised for the governed bulk
/// read; `422` when `since` is not valid RFC3339; a DB error on the query.
#[debug_handler]
async fn bulk_links(
    axum::extract::Query(params): axum::extract::Query<BulkParams>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    authorize_bulk(&caller)?;
    let since = match params.since.as_deref() {
        None => None,
        Some(s) => Some(
            chrono::DateTime::parse_from_rfc3339(s)
                .map_err(|e| unprocessable(&format!("invalid `since` (want RFC3339): {e}")))?,
        ),
    };
    let rows = EntityLinkModel::list_all_active(&ctx.db, since).await?;
    let edges: Vec<EdgeDetail> = rows.iter().map(EdgeDetail::of).collect();
    // Governance §10: audit every bulk surfacing of subject_of edges. There
    // is no single case, so the entity_pid is nil (a system/bulk marker).
    AuditModel::record(
        &ctx.db,
        uuid::Uuid::nil(),
        "links_bulk_read",
        caller.actor(),
        Some(serde_json::json!({
            "kind": "subject_of",
            "count": edges.len(),
            "since": params.since,
        })),
    )
    .await?;
    format::json(serde_json::json!({ "edges": edges }))
}

/// Build the cross-service link routes, mounted under the case prefix.
/// Registered alongside the CRUD routes in `app.rs`.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/cases")
        .add("/links", get(bulk_links))
        .add("/{pid}/links", post(create_link))
        .add("/{pid}/links", get(list_links))
        .add("/{pid}/links/{id}", delete(delete_link))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PERSON: &str = "person:0c4f1e2a-0000-4000-8000-000000000000";
    const CASE: &str = "case:0c4f1e2a-0000-4000-8000-000000000001";

    /// The one accepted combination: `subject_of` case → person.
    #[test]
    fn accepts_subject_of_case_to_person() {
        let (kind, to) = validate_edge("subject_of", PERSON).expect("subject_of case→person");
        assert_eq!(kind, EdgeKind::SubjectOf);
        assert_eq!(to.entity_type, EntityType::Person);
    }

    /// `same_identity` (person ↔ worker) is not a case-originated edge.
    #[test]
    fn rejects_same_identity() {
        assert!(validate_edge("same_identity", PERSON).is_err());
    }

    /// `subject_of` requires a person target — a case target (the
    /// person→case direction, expressed as case→case here) is rejected.
    #[test]
    fn rejects_wrong_endpoint_direction() {
        // subject_of is case→person; a case→case pair is not permitted.
        assert!(validate_edge("subject_of", CASE).is_err());
        // works_at (person→org) can never originate from a case.
        assert!(validate_edge("works_at", PERSON).is_err());
    }

    /// A malformed `to_ref` (not a valid `EntityRef` URN) is rejected.
    #[test]
    fn rejects_malformed_to_ref() {
        for bad in ["not-a-ref", "person:", "widget:123", ""] {
            assert!(validate_edge("subject_of", bad).is_err(), "{bad:?}");
        }
    }

    /// An unknown edge-kind token is rejected before the endpoint check.
    #[test]
    fn rejects_unknown_kind() {
        assert!(validate_edge("befriends", PERSON).is_err());
    }
}
