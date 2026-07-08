//! Loco migrator for worker-service.
//!
//! Each migration wraps the corresponding hand-written SQL under
//! `../migrations/<timestamp>_<name>/{up,down}.sql` via `include_str!`,
//! so the SQL stays the single source of truth while loco gains a Rust
//! `Migrator` it can run (`auto_migrate`, `cargo loco db migrate`).

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
        ]
    }
}
