//! Loco migrator for event-service.
//!
//! Each migration wraps the corresponding hand-written SQL under
//! `../migrations/<timestamp>_<name>/{up,down}.sql` via `include_str!`,
//! so the SQL stays the single source of truth while loco gains a Rust
//! `Migrator` it can run (`auto_migrate`, `cargo loco db migrate`).

// SEC-I3: migrators run pure SQL orchestration; forbid unsafe.
#![forbid(unsafe_code)]
#![allow(elided_lifetimes_in_paths)]
pub use sea_orm_migration::prelude::*;

mod m20241228_000001_create_organizations;
mod m20241228_000004_create_audit_tables;
mod m20260530_000002_create_events;
mod m20260530_000003_create_event_related_tables;
mod m20260530_000005_add_indexes_and_triggers;
mod m20260608_000001_normalize_event_text_values;
mod m20260708_000001_create_event_outbox;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20241228_000001_create_organizations::Migration),
            Box::new(m20241228_000004_create_audit_tables::Migration),
            Box::new(m20260530_000002_create_events::Migration),
            Box::new(m20260530_000003_create_event_related_tables::Migration),
            Box::new(m20260530_000005_add_indexes_and_triggers::Migration),
            Box::new(m20260608_000001_normalize_event_text_values::Migration),
            Box::new(m20260708_000001_create_event_outbox::Migration),
        ]
    }
}
