//! `sea-orm-migration` schema migrations for `case-service`.
//!
//! The [`Migrator`] runs the ordered list below at boot (or via
//! `cargo loco db migrate`): the `cases` table (the registry), then the
//! `audit_logs` and `merge_records` side tables. Case data is personal
//! data, so the audit trail these create is the who/what/when record over
//! every change.

// SEC-I3: migrators run pure SQL orchestration; forbid unsafe.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(elided_lifetimes_in_paths)]
pub use sea_orm_migration::prelude::*;
mod m20220101_000001_cases;
mod m20220101_000002_audit_logs;
mod m20220101_000003_merge_records;
mod m20220101_000004_event_outbox;
mod m20220101_000005_entity_links;
mod m20260726_000006_compliance;
mod m20260727_000007_case_content_hash;
mod m20260727_000008_blake3_digests;

/// The crate's migrator: drives the ordered migration set for the loco
/// CLI / boot-time migration.
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    /// The ordered migration set. Order matters: `cases` is created first
    /// because the side tables reference its `pid`, then `audit_logs` and
    /// `merge_records`.
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_cases::Migration),
            Box::new(m20220101_000002_audit_logs::Migration),
            Box::new(m20220101_000003_merge_records::Migration),
            Box::new(m20220101_000004_event_outbox::Migration),
            Box::new(m20220101_000005_entity_links::Migration),
            Box::new(m20260726_000006_compliance::Migration),
            Box::new(m20260727_000007_case_content_hash::Migration),
            Box::new(m20260727_000008_blake3_digests::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
