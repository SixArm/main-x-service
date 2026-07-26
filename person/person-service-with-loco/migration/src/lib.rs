//! Loco migrator for person-service.
//!
//! Each migration wraps the corresponding hand-written SQL under
//! `../migrations/<timestamp>_<name>/{up,down}.sql` via `include_str!`,
//! so the SQL stays the single source of truth while loco gains a Rust
//! `Migrator` it can run (`auto_migrate`, `cargo loco db migrate`).

// SEC-I3: migrators run pure SQL orchestration; forbid unsafe.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(elided_lifetimes_in_paths)]
pub use sea_orm_migration::prelude::*;

mod m20241228_000001_create_organizations;
mod m20241228_000002_create_patients;
mod m20241228_000003_create_patient_related_tables;
mod m20241228_000004_create_audit_tables;
mod m20241228_000005_add_indexes_and_triggers;
mod m20260603_000001_rename_patient_tables_to_person;
mod m20260608_000001_add_person_persistence_fields;
mod m20260708_000001_create_event_outbox;
mod m20260710_000001_create_entity_links;
mod m20260710_000002_create_bulk_jobs;
mod m20260719_000001_create_review_queue;
mod m20260726_000001_audit_chain;
mod m20260726_000002_rename_patient_id_columns;

/// The migration runner this crate exposes to loco / `sea-orm-migration`.
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20241228_000001_create_organizations::Migration),
            Box::new(m20241228_000002_create_patients::Migration),
            Box::new(m20241228_000003_create_patient_related_tables::Migration),
            Box::new(m20241228_000004_create_audit_tables::Migration),
            Box::new(m20241228_000005_add_indexes_and_triggers::Migration),
            Box::new(m20260603_000001_rename_patient_tables_to_person::Migration),
            Box::new(m20260608_000001_add_person_persistence_fields::Migration),
            Box::new(m20260708_000001_create_event_outbox::Migration),
            Box::new(m20260710_000001_create_entity_links::Migration),
            Box::new(m20260710_000002_create_bulk_jobs::Migration),
            Box::new(m20260719_000001_create_review_queue::Migration),
            Box::new(m20260726_000001_audit_chain::Migration),
            Box::new(m20260726_000002_rename_patient_id_columns::Migration),
        ]
    }
}
