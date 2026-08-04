//! Domain models: the `SeaORM` entities for the derived read-model plus
//! their projection / query helpers (spec §5, §10).

pub mod _entities;
pub mod audit_log;
pub mod consumer_offsets;
pub mod edges;
pub mod entity_presence;
pub mod processed_events;
pub mod suggestion_runs;
