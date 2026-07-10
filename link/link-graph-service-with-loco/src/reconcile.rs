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
use sea_orm::ConnectionTrait;
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
}
