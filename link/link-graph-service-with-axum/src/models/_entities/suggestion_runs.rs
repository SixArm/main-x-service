//! `SeaORM` Entity — `suggestion_runs`. The durable per-pass audit trail
//! for the periodic cross-service `same_identity` suggestion job (spec
//! T-33, design §16 OQ-9(d)).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// One completed suggestion-job pass.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "suggestion_runs")]
pub struct Model {
    /// Row id (UUID v4, generated at record time).
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// When this pass started (before the fetch).
    pub started_at: DateTimeWithTimeZone,
    /// When this pass finished (after the last POST attempt).
    pub completed_at: DateTimeWithTimeZone,
    /// Persons fetched this pass.
    pub persons_fetched: i64,
    /// Workers fetched this pass.
    pub workers_fetched: i64,
    /// Candidates `generate_candidates_bounded` returned.
    pub candidates: i64,
    /// Candidates successfully `POSTed`.
    pub posted: i64,
    /// Candidates whose POST failed.
    pub failed: i64,
    /// Candidates dropped by the `max_edges_per_run` cap.
    pub dropped: i64,
    /// The `LINK_GRAPH_SUGGEST_MAX_CANDIDATES` value this pass ran with.
    pub max_candidates: i64,
    /// The `LINK_GRAPH_SUGGEST_MAX_EDGES_PER_RUN` value this pass ran with.
    pub max_edges_per_run: i64,
}

/// `SeaORM` relations for [`Entity`] (none defined).
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
