//! Loco migrator for person-service.
//!
//! Each migration wraps the corresponding hand-written SQL under
//! `../migrations/<timestamp>_<name>/{up,down}.sql` via `include_str!`,
//! so the SQL stays the single source of truth while loco gains a Rust
//! `Migrator` it can run (`auto_migrate`, `cargo loco db migrate`).

#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
pub use sea_orm_migration::prelude::*;

mod m20241228_000001_create_organizations;
mod m20241228_000002_create_patients;
mod m20241228_000003_create_patient_related_tables;
mod m20241228_000004_create_audit_tables;
mod m20241228_000005_add_indexes_and_triggers;
mod m20260603_000001_rename_patient_tables_to_person;
mod m20260608_000001_add_person_persistence_fields;

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
        ]
    }
}
