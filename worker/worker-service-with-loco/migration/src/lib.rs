//! Loco migrator for worker-service.
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
mod m20241228_000002_create_workers;
mod m20241228_000003_create_worker_related_tables;
mod m20241228_000004_create_audit_tables;
mod m20241228_000005_add_indexes_and_triggers;
mod m20241228_000006_add_worker_type;
mod m20241228_000007_expand_organizations_ods;
mod m20241228_000008_create_ods_tables;
mod m20241228_000009_create_codesystem_tables;
mod m20260608_000001_add_worker_persistence_fields;
mod m20260708_000001_create_event_outbox;
mod m20260710_000001_create_entity_links;
mod m20260719_000001_create_review_queue;
mod m20260723_000001_create_worker_assessments;
mod m20260723_000002_normalize_worker_gender_case;
mod m20260726_000001_audit_chain;
mod m20260726_000003_drop_audit_triggers;
mod m20260726_000004_worker_content_hash;

/// The migration runner this crate exposes to loco / `sea-orm-migration`.
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20241228_000001_create_organizations::Migration),
            Box::new(m20241228_000002_create_workers::Migration),
            Box::new(m20241228_000003_create_worker_related_tables::Migration),
            Box::new(m20241228_000004_create_audit_tables::Migration),
            Box::new(m20241228_000005_add_indexes_and_triggers::Migration),
            Box::new(m20241228_000006_add_worker_type::Migration),
            Box::new(m20241228_000007_expand_organizations_ods::Migration),
            Box::new(m20241228_000008_create_ods_tables::Migration),
            Box::new(m20241228_000009_create_codesystem_tables::Migration),
            Box::new(m20260608_000001_add_worker_persistence_fields::Migration),
            Box::new(m20260708_000001_create_event_outbox::Migration),
            Box::new(m20260710_000001_create_entity_links::Migration),
            Box::new(m20260719_000001_create_review_queue::Migration),
            Box::new(m20260723_000001_create_worker_assessments::Migration),
            Box::new(m20260723_000002_normalize_worker_gender_case::Migration),
            Box::new(m20260726_000001_audit_chain::Migration),
            Box::new(m20260726_000003_drop_audit_triggers::Migration),
            Box::new(m20260726_000004_worker_content_hash::Migration),
        ]
    }
}
