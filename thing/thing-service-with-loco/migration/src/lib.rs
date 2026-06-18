//! Loco migrator for thing-service.
//!
//! Each migration wraps the corresponding hand-written SQL under
//! `../migrations/<timestamp>_<name>/{up,down}.sql` via `include_str!`,
//! so the SQL stays the single source of truth while loco gains a Rust
//! `Migrator` it can run (`auto_migrate`, `cargo loco db migrate`).

#![allow(elided_lifetimes_in_paths)]
pub use sea_orm_migration::prelude::*;

mod m20260608_000001_create_things;
mod m20260608_000002_create_audit_and_merge;

/// The loco/SeaORM migrator for thing-service. Boot wires this into
/// `create_app::<App, Migrator>` so migrations run on startup / via the CLI.
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    /// The ordered list of migrations to apply (oldest first). The runner
    /// executes any not yet recorded in `seaql_migrations`.
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260608_000001_create_things::Migration),
            Box::new(m20260608_000002_create_audit_and_merge::Migration),
        ]
    }
}
