//! `suggestion_runs` model — the durable per-pass audit trail for the
//! periodic cross-service `same_identity` suggestion job (spec T-33,
//! design §16 OQ-9(d) "audit ... every run's counts").
//!
//! [`crate::reconcile`]'s own periodic pass records its one summary
//! number (`link_graph_reconciliation_divergence`) only as a Prometheus
//! gauge plus a `tracing::info!` line — enough there, because "did the
//! last pass find drift" is answered by the gauge's *current* value.
//! This job's summary is richer (fetch counts on two upstream services
//! plus a post/fail/drop split) and needs to survive a missed scrape or
//! a process restart, so an operator can answer "what did the
//! suggestion job actually do" after the fact — a gauge cannot answer
//! that, only a durable row can. One row per **completed** pass; a pass
//! that fails at the fetch step (the only error
//! [`crate::suggest::job::run_suggestion_pass`] can return) records
//! nothing, matching that job's existing log-and-retry posture for that
//! case — there are no counts worth keeping from a run that never
//! fetched anything.

use loco_rs::prelude::*;
use sea_orm::{ActiveValue, ConnectionTrait};
use uuid::Uuid;

pub use super::_entities::suggestion_runs::{self, ActiveModel, Entity, Model};

/// Default `SeaORM` active-model behaviour — no custom hooks.
impl ActiveModelBehavior for ActiveModel {}

/// The counts to persist for one completed pass. A separate type from
/// [`crate::suggest::job::SuggestionRunStats`] (rather than taking that
/// type directly) so this model module carries no dependency on the
/// suggest/job module's HTTP-client-heavy imports — the usual
/// models-are-the-lower-layer direction in this crate.
#[derive(Debug, Clone, Copy)]
pub struct SuggestionRunRecord {
    /// When the pass started (captured before the fetch, so
    /// `completed_at - started_at` reflects the pass's real wall-clock
    /// span, not just the moment of insert).
    pub started_at: chrono::DateTime<chrono::FixedOffset>,
    /// Persons fetched.
    pub persons_fetched: usize,
    /// Workers fetched.
    pub workers_fetched: usize,
    /// Candidates found (at/above the suggestion threshold).
    pub candidates: usize,
    /// Candidates successfully `POSTed`.
    pub posted: usize,
    /// Candidates whose POST failed.
    pub failed: usize,
    /// Candidates dropped by the `max_edges_per_run` cap.
    pub dropped: usize,
    /// The `max_candidates` value this pass ran with.
    pub max_candidates: usize,
    /// The `max_edges_per_run` value this pass ran with.
    pub max_edges_per_run: usize,
}

/// Cast a count to `i64` for storage, saturating rather than panicking —
/// the same posture `crate::metrics` already takes for its gauges
/// (`i64::try_from(count).unwrap_or(i64::MAX)`). No realistic pass count
/// approaches `i64::MAX`; this only guards the type conversion itself.
fn to_i64(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

impl Model {
    /// Record one completed suggestion pass.
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn record<C: ConnectionTrait>(db: &C, rec: &SuggestionRunRecord) -> ModelResult<()> {
        let am = suggestion_runs::ActiveModel {
            id: ActiveValue::set(Uuid::new_v4()),
            started_at: ActiveValue::set(rec.started_at),
            completed_at: ActiveValue::set(chrono::Utc::now().fixed_offset()),
            persons_fetched: ActiveValue::set(to_i64(rec.persons_fetched)),
            workers_fetched: ActiveValue::set(to_i64(rec.workers_fetched)),
            candidates: ActiveValue::set(to_i64(rec.candidates)),
            posted: ActiveValue::set(to_i64(rec.posted)),
            failed: ActiveValue::set(to_i64(rec.failed)),
            dropped: ActiveValue::set(to_i64(rec.dropped)),
            max_candidates: ActiveValue::set(to_i64(rec.max_candidates)),
            max_edges_per_run: ActiveValue::set(to_i64(rec.max_edges_per_run)),
        };
        Entity::insert(am).exec(db).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::to_i64;

    #[test]
    fn to_i64_never_panics_and_saturates_at_the_top() {
        assert_eq!(to_i64(0), 0);
        assert_eq!(to_i64(42), 42);
        assert_eq!(to_i64(usize::MAX), i64::MAX);
    }
}
