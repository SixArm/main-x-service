//! Reconciliation (spec T-20 / design §8) — the cost of two sources of
//! truth. The per-service `entity_links` (authoritative, in each entity
//! service) and this aggregator's derived `edges` can drift (a dropped
//! event, a relay bug). A periodic pass pulls a service's authoritative
//! edges, diffs them against the read-model, emits a **divergence** metric
//! (an SLO — steady-state ~0), and repairs the read-model.
//!
//! The pull is behind the [`AuthoritativeSource`] trait so the diff/repair
//! logic is testable without a live service; the real source (a bulk-read
//! endpoint or a topic replay, §8) is a follow-up — it needs a bulk-links
//! endpoint on each service (case has the per-record `entity_links` write
//! side today; a global list is the next step).

use std::collections::HashSet;

use async_trait::async_trait;
use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseConnection};
use serde::Deserialize;
use uuid::Uuid;

use entity_ref::EntityType;

use crate::events::LinkedEvent;
use crate::metrics::Metrics;
use crate::models::edges;

/// A source of a service's **authoritative** outbound edges (its
/// `entity_links`), for reconciliation. Each edge is described by the same
/// [`LinkedEvent`] shape the bus carries. Injectable so [`reconcile`] is
/// testable with a mock.
#[async_trait]
pub trait AuthoritativeSource: Send + Sync {
    /// The entity this source is authoritative for. Reconciliation is
    /// **scoped** to this entity's edges (SEC-B1): a source only ever adds
    /// or removes edges originating from its own entity, so a per-entity
    /// pass never deletes another entity's edges.
    fn entity(&self) -> EntityType;

    /// Fetch every active authoritative edge (bulk read or topic replay).
    ///
    /// # Errors
    ///
    /// When the underlying fetch fails.
    async fn fetch_all(&self) -> ModelResult<Vec<LinkedEvent>>;
}

/// The divergence between the read-model and an authoritative edge set.
#[derive(Debug, Default)]
pub struct Divergence {
    /// Authoritative edges absent from the read-model (must be added).
    pub missing: Vec<LinkedEvent>,
    /// Read-model edge ids absent from the authoritative set (must be
    /// removed).
    pub extra: Vec<Uuid>,
}

impl Divergence {
    /// Total diverging edges (the SLO number).
    #[must_use]
    pub fn count(&self) -> usize {
        self.missing.len() + self.extra.len()
    }
}

/// Pure diff (keyed on `edge_id`): which authoritative edges are missing
/// from the read-model, and which read-model edges are extra. Testable
/// without a database.
#[must_use]
pub fn diff<H: std::hash::BuildHasher>(
    readmodel_ids: &HashSet<Uuid, H>,
    authoritative: &[LinkedEvent],
) -> Divergence {
    let auth_ids: HashSet<Uuid> = authoritative.iter().map(|e| e.edge_id).collect();
    let missing = authoritative
        .iter()
        .filter(|e| !readmodel_ids.contains(&e.edge_id))
        .cloned()
        .collect();
    let extra = readmodel_ids
        .iter()
        .filter(|id| !auth_ids.contains(id))
        .copied()
        .collect();
    Divergence { missing, extra }
}

/// Reconcile the read-model against a service's authoritative edges: diff,
/// set the `link_graph_reconciliation_divergence` metric, then repair
/// (upsert the missing, remove the extra). Returns the divergence count
/// found (before repair).
///
/// # Errors
///
/// When the fetch, a read-model query, or a repair write fails.
pub async fn reconcile<C, S>(db: &C, source: &S) -> ModelResult<usize>
where
    C: ConnectionTrait,
    S: AuthoritativeSource + ?Sized,
{
    let authoritative = source.fetch_all().await?;
    // Scope the read-model side to THIS source's entity (SEC-B1). Diffing
    // against `all_edge_ids` would mark every other entity's edges as
    // "extra" and delete them, so each per-entity pass would wipe the rest
    // of the graph.
    let readmodel_ids = edges::Model::edge_ids_from_entity(db, source.entity()).await?;
    let divergence = diff(&readmodel_ids, &authoritative);
    let count = divergence.count();

    Metrics::global()
        .reconciliation_divergence
        .set(i64::try_from(count).unwrap_or(i64::MAX));

    // Repair the read-model to match the authoritative source.
    let observed_at = Utc::now().fixed_offset();
    for ev in &divergence.missing {
        // SEC-B7: validate each authoritative edge's endpoint types (and
        // that it originates from this source's own entity) before applying
        // it — a compromised or buggy source must not be able to inject a
        // cross-typed or foreign-origin edge into the graph. Ill-typed edges
        // are skipped (and stay "missing", so a persistently bad source keeps
        // showing as divergence) rather than corrupting the read-model.
        if !edge_valid_for_source(ev, source.entity()) {
            tracing::warn!(
                edge_id = %ev.edge_id,
                kind = ?ev.edge_kind,
                from = %ev.from_ref,
                to = %ev.to_ref,
                "reconcile: rejecting an ill-typed or foreign-origin authoritative edge"
            );
            continue;
        }
        edges::Model::apply_linked(db, ev, ev.edge_id, observed_at).await?;
    }
    for id in &divergence.extra {
        edges::Model::apply_unlinked(db, *id).await?;
    }
    Ok(count)
}

/// The bulk-links response shape from a service's `GET /…/links` — the
/// canonical §4.2 edge list, each row deserializing straight into a
/// [`LinkedEvent`] (`edge_id` / `edge_kind` field names, `from_ref` URN).
#[derive(Debug, Deserialize)]
struct BulkLinksResponse {
    edges: Vec<LinkedEvent>,
}

/// The **real** authoritative source: a one-shot `GET` to a service's
/// bulk-links endpoint (design §8), optionally bearer-authenticated. The
/// URL comes from `LINK_GRAPH_RECONCILE_URL_<ENTITY>` and the token from
/// `LINK_GRAPH_RECONCILE_TOKEN`.
pub struct HttpAuthoritativeSource {
    entity: EntityType,
    url: String,
    token: Option<String>,
}

impl HttpAuthoritativeSource {
    /// Build from the environment for `entity` (e.g. `case`), or `None`
    /// when `entity` is not a known type, no
    /// `LINK_GRAPH_RECONCILE_URL_<ENTITY>` is configured, or (SEC-B7) the
    /// URL names a **remote** host but no `LINK_GRAPH_RECONCILE_TOKEN` is
    /// set — an unauthenticated pull from a remote source is refused, since
    /// its edges are applied directly to the graph.
    #[must_use]
    pub fn from_env_for(entity: &str) -> Option<Self> {
        let entity = EntityType::from_token(entity)?;
        let url = std::env::var(format!(
            "LINK_GRAPH_RECONCILE_URL_{}",
            entity.as_str().to_ascii_uppercase()
        ))
        .ok()
        .filter(|s| !s.trim().is_empty())?;
        let token = std::env::var("LINK_GRAPH_RECONCILE_TOKEN")
            .ok()
            .filter(|s| !s.trim().is_empty());
        if !source_auth_ok(&url, token.is_some()) {
            tracing::warn!(
                %url,
                "refusing an unauthenticated remote reconcile source: set \
                 LINK_GRAPH_RECONCILE_TOKEN (only a loopback URL may be token-less)"
            );
            return None;
        }
        Some(Self { entity, url, token })
    }
}

/// SEC-B7: whether a reconcile source `url` may be used with the given token
/// presence. A **loopback** URL (dev/test) may be token-less; any other host
/// requires a bearer token, since a remote source's edges are applied to the
/// graph unverified-by-origin otherwise. Pure, so it is unit-testable.
#[must_use]
fn source_auth_ok(url: &str, has_token: bool) -> bool {
    has_token || is_loopback_url(url)
}

/// Whether `url`'s host is a loopback address (`127.0.0.1`, `::1`,
/// `localhost`). A parse failure or missing host is treated as **not**
/// loopback (fail-closed → a token is then required).
#[must_use]
fn is_loopback_url(url: &str) -> bool {
    reqwest::Url::parse(url)
        .ok()
        .and_then(|u| {
            u.host_str().map(|h| {
                let h = h.trim_start_matches('[').trim_end_matches(']');
                h == "127.0.0.1" || h == "::1" || h.eq_ignore_ascii_case("localhost")
            })
        })
        .unwrap_or(false)
}

/// SEC-B7: whether an authoritative `ev` is well-formed for a source that is
/// authoritative for `source_entity`. An edge must **originate** from the
/// source's own entity (a source is only authoritative for its own outbound
/// edges) and its endpoint types must be **permitted** for its kind by the
/// closed registry (`EdgeKind::permits`), so a compromised or buggy source
/// cannot inject cross-typed or foreign-origin edges. Pure and testable.
#[must_use]
fn edge_valid_for_source(ev: &LinkedEvent, source_entity: EntityType) -> bool {
    ev.from_ref.entity_type == source_entity
        && ev
            .edge_kind
            .permits(ev.from_ref.entity_type, ev.to_ref.entity_type)
}

#[async_trait]
impl AuthoritativeSource for HttpAuthoritativeSource {
    fn entity(&self) -> EntityType {
        self.entity
    }

    async fn fetch_all(&self) -> ModelResult<Vec<LinkedEvent>> {
        let mut request = reqwest::Client::new().get(&self.url);
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        let response = request
            .send()
            .await
            .map_err(|e| ModelError::Any(Box::new(e)))?
            .error_for_status()
            .map_err(|e| ModelError::Any(Box::new(e)))?;
        let body: BulkLinksResponse = response
            .json()
            .await
            .map_err(|e| ModelError::Any(Box::new(e)))?;
        Ok(body.edges)
    }
}

/// Run reconciliation periodically against `source` until the process
/// exits — the "worker" wiring (design §8). The interval is
/// `LINK_GRAPH_RECONCILE_SECS` (default 300); the first tick is skipped so
/// boot is not blocked. A failed pass is logged and retried next tick.
/// Spawned from `App::after_routes` only when a source is configured.
pub async fn run_periodic<S: AuthoritativeSource>(db: DatabaseConnection, source: S) {
    let secs = std::env::var("LINK_GRAPH_RECONCILE_SECS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|s| *s > 0)
        .unwrap_or(300);
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(secs));
    interval.tick().await; // consume the immediate first tick
    loop {
        interval.tick().await;
        match reconcile(&db, &source).await {
            Ok(n) => tracing::info!(divergence = n, "reconciliation pass complete"),
            Err(error) => tracing::warn!(%error, "reconciliation pass failed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use entity_ref::{EdgeKind, EntityRef, EntityType};

    fn edge(id: u128) -> LinkedEvent {
        LinkedEvent {
            edge_id: Uuid::from_u128(id),
            from_ref: EntityRef::new(EntityType::Case, Uuid::from_u128(1)),
            to_ref: EntityRef::new(EntityType::Person, Uuid::from_u128(2)),
            edge_kind: EdgeKind::SubjectOf,
            role: None,
            confidence: None,
            provenance: "operator".into(),
            valid_from: None,
            valid_to: None,
        }
    }

    #[test]
    fn diff_finds_missing_and_extra_by_edge_id() {
        // read-model has {1, 2}; authoritative has {2, 3}.
        let readmodel: HashSet<Uuid> = [Uuid::from_u128(1), Uuid::from_u128(2)]
            .into_iter()
            .collect();
        let authoritative = vec![edge(2), edge(3)];
        let d = diff(&readmodel, &authoritative);

        // 3 is authoritative but not in the read-model.
        assert_eq!(d.missing.len(), 1);
        assert_eq!(d.missing[0].edge_id, Uuid::from_u128(3));
        // 1 is in the read-model but not authoritative.
        assert_eq!(d.extra, vec![Uuid::from_u128(1)]);
        assert_eq!(d.count(), 2);
    }

    #[test]
    fn diff_is_zero_when_the_sets_match() {
        let readmodel: HashSet<Uuid> = [Uuid::from_u128(5)].into_iter().collect();
        let d = diff(&readmodel, &[edge(5)]);
        assert_eq!(d.count(), 0);
        assert!(d.missing.is_empty() && d.extra.is_empty());
    }

    #[test]
    fn bulk_response_deserializes_the_case_bulk_links_shape() {
        // Exactly the JSON the case service's GET /api/cases/links emits
        // (its `EdgeDetail`): the cross-service integration contract. If
        // this parses, the HttpAuthoritativeSource can consume case.
        let json = serde_json::json!({
            "edges": [{
                "edge_id": "0c4f1e2a-0000-4000-8000-000000000010",
                "from_ref": "case:0c4f1e2a-0000-4000-8000-000000000001",
                "to_ref": "person:0c4f1e2a-0000-4000-8000-000000000002",
                "edge_kind": "subject_of",
                "role": null,
                "confidence": null,
                "provenance": "operator",
                "valid_from": null,
                "valid_to": null
            }]
        });
        let parsed: BulkLinksResponse = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.edges.len(), 1);
        assert_eq!(parsed.edges[0].edge_kind, EdgeKind::SubjectOf);
        assert_eq!(parsed.edges[0].from_ref.entity_type, EntityType::Case);
        assert_eq!(parsed.edges[0].to_ref.entity_type, EntityType::Person);
        assert_eq!(parsed.edges[0].provenance, "operator");
    }

    #[test]
    fn bulk_response_deserializes_the_person_same_identity_shape() {
        // The person service's GET /api/persons/links emits the same
        // canonical shape for its same_identity (person → worker) edges.
        let json = serde_json::json!({
            "edges": [{
                "edge_id": "0c4f1e2a-0000-4000-8000-000000000010",
                "from_ref": "person:0c4f1e2a-0000-4000-8000-000000000001",
                "to_ref": "worker:0c4f1e2a-0000-4000-8000-000000000002",
                "edge_kind": "same_identity",
                "role": null,
                "confidence": 1.0,
                "provenance": "operator",
                "valid_from": null,
                "valid_to": null
            }]
        });
        let parsed: BulkLinksResponse = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.edges[0].edge_kind, EdgeKind::SameIdentity);
        assert_eq!(parsed.edges[0].from_ref.entity_type, EntityType::Person);
        assert_eq!(parsed.edges[0].to_ref.entity_type, EntityType::Worker);
        assert_eq!(parsed.edges[0].confidence, Some(1.0));
    }

    #[test]
    fn bulk_response_deserializes_the_worker_same_identity_shape() {
        // LNK-2: the worker service's GET /api/workers/links emits the same
        // canonical §4.2 shape for the WORKER side of the symmetric
        // same_identity edge (worker → person) — the inverse direction of the
        // person seam above. Both directions deserialize into a LinkedEvent,
        // and the aggregator canonicalises the pair (graph.rs), so the
        // symmetric double-assert is deduped.
        let json = serde_json::json!({
            "edges": [{
                "edge_id": "0c4f1e2a-0000-4000-8000-000000000010",
                "from_ref": "worker:0c4f1e2a-0000-4000-8000-000000000002",
                "to_ref": "person:0c4f1e2a-0000-4000-8000-000000000001",
                "edge_kind": "same_identity",
                "role": null,
                "confidence": 1.0,
                "provenance": "operator",
                "valid_from": null,
                "valid_to": null
            }]
        });
        let parsed: BulkLinksResponse = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.edges[0].edge_kind, EdgeKind::SameIdentity);
        assert_eq!(parsed.edges[0].from_ref.entity_type, EntityType::Worker);
        assert_eq!(parsed.edges[0].to_ref.entity_type, EntityType::Person);
        assert_eq!(parsed.edges[0].confidence, Some(1.0));
    }

    /// SEC-B7: a remote reconcile source must carry a token; only a loopback
    /// URL may be token-less.
    #[test]
    fn source_auth_requires_a_token_for_remote_hosts() {
        // Remote host: token required.
        assert!(!super::source_auth_ok(
            "https://links.example.com/api",
            false
        ));
        assert!(super::source_auth_ok("https://links.example.com/api", true));
        // Loopback (dev/test): token optional.
        assert!(super::source_auth_ok(
            "http://127.0.0.1:5150/api/links",
            false
        ));
        assert!(super::source_auth_ok(
            "http://localhost:5150/api/links",
            false
        ));
        assert!(super::source_auth_ok("http://[::1]:5150/api/links", false));
        // Unparseable / hostless: fail closed (a token is required).
        assert!(!super::is_loopback_url("not a url"));
        assert!(!super::source_auth_ok("not a url", false));
    }

    /// SEC-B7: an authoritative edge is applied only if it originates from
    /// the source's entity and its endpoint types are permitted for its kind.
    #[test]
    fn edge_validity_enforces_origin_and_endpoint_types() {
        // A well-typed case → person `subject_of` from the case source.
        let ok = edge(1);
        assert!(super::edge_valid_for_source(&ok, EntityType::Case));

        // Same edge, but claimed by a source authoritative for a different
        // entity (person) — its `from_ref` is a case, so it is rejected.
        assert!(!super::edge_valid_for_source(&ok, EntityType::Person));

        // An ill-typed edge: `same_identity` only permits person↔worker, so
        // a case→person `same_identity` is refused even from the case source.
        let mut ill = edge(2);
        ill.edge_kind = EdgeKind::SameIdentity;
        assert!(!super::edge_valid_for_source(&ill, EntityType::Case));
    }
}
