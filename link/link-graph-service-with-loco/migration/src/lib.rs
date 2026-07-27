//! Database migrations for `link-graph-service`.
//!
//! The schema source of truth is the hand-written, numbered SQL under
//! `migrations/` (`up.sql` / `down.sql` per step); each Rust migration
//! wraps the SQL pair via `include_str!`, consistent with the sibling
//! service crates (spec §10). Creates the derived read-model tables
//! `edges`, `entity_presence`, `consumer_offsets`, and the governance
//! `audit_log` (§10.4). The `processed_events` idempotency table (§10.3)
//! is deferred with the Fluvio bus consumer.

// SEC-I3: migrators run pure SQL orchestration; forbid unsafe.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(elided_lifetimes_in_paths)]
pub use sea_orm_migration::prelude::*;

mod m20260709_000001_edges;
mod m20260709_000002_entity_presence;
mod m20260709_000003_consumer_offsets;
mod m20260709_000004_audit_log;
mod m20260728_000001_add_audit_mac;

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
            Box::new(m20260709_000004_audit_log::Migration),
            Box::new(m20260728_000001_add_audit_mac::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
