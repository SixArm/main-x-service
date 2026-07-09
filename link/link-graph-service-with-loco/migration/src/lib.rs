//! Database migrations for `link-graph-service`.
//!
//! The schema source of truth is the hand-written, numbered SQL under
//! `migrations/` (`up.sql` / `down.sql` per step); each Rust migration
//! wraps the SQL pair via `include_str!`, consistent with the sibling
//! service crates (spec §10). v1 creates three derived read-model
//! tables: `edges`, `entity_presence`, `consumer_offsets`. The
//! `processed_events` and `audit_log` tables (spec §10.3/§10.4) are
//! v1-deferred.

#![allow(elided_lifetimes_in_paths)]
pub use sea_orm_migration::prelude::*;

mod m20260709_000001_edges;
mod m20260709_000002_entity_presence;
mod m20260709_000003_consumer_offsets;

/// The migration runner this crate exposes to loco / `sea-orm-migration`.
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    /// The ordered list of migrations to apply (oldest first).
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260709_000001_edges::Migration),
            Box::new(m20260709_000002_entity_presence::Migration),
            Box::new(m20260709_000003_consumer_offsets::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
