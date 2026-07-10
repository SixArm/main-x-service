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

use crate::events::LinkedEvent;
use crate::metrics::Metrics;
use crate::models::edges;

/// A source of a service's **authoritative** outbound edges (its
/// `entity_links`), for reconciliation. Each edge is described by the same
/// [`LinkedEvent`] shape the bus carries. Injectable so [`reconcile`] is
/// testable with a mock.
#[async_trait]
pub trait AuthoritativeSource: Send + Sync {
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
    let readmodel_ids = edges::Model::all_edge_ids(db).await?;
    let divergence = diff(&readmodel_ids, &authoritative);
    let count = divergence.count();

    Metrics::global()
        .reconciliation_divergence
        .set(i64::try_from(count).unwrap_or(i64::MAX));

    // Repair the read-model to match the authoritative source.
    let observed_at = Utc::now().fixed_offset();
    for ev in &divergence.missing {
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
    url: String,
    token: Option<String>,
}

impl HttpAuthoritativeSource {
    /// Build from the environment for `entity` (e.g. `case`), or `None`
    /// when no `LINK_GRAPH_RECONCILE_URL_<ENTITY>` is configured.
    #[must_use]
    pub fn from_env_for(entity: &str) -> Option<Self> {
        let url = std::env::var(format!(
            "LINK_GRAPH_RECONCILE_URL_{}",
            entity.to_ascii_uppercase()
        ))
        .ok()
        .filter(|s| !s.trim().is_empty())?;
        let token = std::env::var("LINK_GRAPH_RECONCILE_TOKEN")
            .ok()
            .filter(|s| !s.trim().is_empty());
        Some(Self { url, token })
    }
}

#[async_trait]
impl AuthoritativeSource for HttpAuthoritativeSource {
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
}
