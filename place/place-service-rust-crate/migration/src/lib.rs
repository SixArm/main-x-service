//! Loco migrator for place-service.
//!
//! Each migration wraps the corresponding hand-written SQL under
//! `../migrations/<timestamp>_<name>/{up,down}.sql` via `include_str!`,
//! so the SQL stays the single source of truth while loco gains a Rust
//! `Migrator` it can run (`auto_migrate`, `cargo loco db migrate`).

#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
pub use sea_orm_migration::prelude::*;

mod m20260608_000001_create_places;
mod m20260608_000002_create_audit_and_merge;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260608_000001_create_places::Migration),
            Box::new(m20260608_000002_create_audit_and_merge::Migration),
        ]
    }
}
