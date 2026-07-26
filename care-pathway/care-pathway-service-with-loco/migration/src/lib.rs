//! Database migrations for `care-pathway-service`.
//!
//! Driven by loco's CLI via [`Migrator`] (`sea-orm-migration`). The
//! migrations create the entity table `care_pathways` (with the
//! `CarePathway` payload as JSONB), `audit_logs` (the CRUD trail, plus
//! the compliance columns), `merge_records` (the merge history),
//! `event_outbox` (the durable event bus), and the instance layer.

// Always start with high quality coding conventions
// (`agents/share/rust-loco-stack.md`).
// SEC-I3: migrators run pure SQL orchestration; forbid unsafe.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]
// `sea_orm_migration::prelude::*` brings in types whose signatures elide
// lifetimes in paths; the lint fires on the macro-generated code rather
// than on anything written here.
#![allow(elided_lifetimes_in_paths)]
pub use sea_orm_migration::prelude::*;
mod m20220101_000001_care_pathways;
mod m20220101_000002_audit_logs;
mod m20220101_000003_merge_records;
mod m20220101_000004_event_outbox;
mod m20260720_000005_instances;
mod m20260720_000006_outcomes;
mod m20260725_000007_compliance;
mod m20260726_000008_record_integrity;
mod m20260726_000009_bulk_jobs;

/// The migration runner this crate exposes to loco / `sea-orm-migration`.
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    /// The ordered list of migrations to apply (oldest first). The
    /// inject marker is kept so `cargo loco generate migration` can splice
    /// new migrations in.
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_care_pathways::Migration),
            Box::new(m20220101_000002_audit_logs::Migration),
            Box::new(m20220101_000003_merge_records::Migration),
            Box::new(m20220101_000004_event_outbox::Migration),
            Box::new(m20260720_000005_instances::Migration),
            Box::new(m20260720_000006_outcomes::Migration),
            Box::new(m20260725_000007_compliance::Migration),
            Box::new(m20260726_000008_record_integrity::Migration),
            Box::new(m20260726_000009_bulk_jobs::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
