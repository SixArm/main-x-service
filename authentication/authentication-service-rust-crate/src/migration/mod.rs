//! Database migrations for the authentication service.
//!
//! Embedded in the crate (rather than a separate `migration` workspace
//! crate) so the loco `App` boot, the CLI entrypoint, and the migrations
//! all live in one compilation unit. The [`Migrator`] is handed to
//! `create_app` / `cli::main` in [`crate::app`] and `src/bin/main.rs`.
#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
pub use sea_orm_migration::prelude::*;
mod m20220101_000001_users;
mod m20220101_000002_sessions;
mod m20220101_000003_auth_events;

/// loco/`SeaORM` migrator that runs every migration in order.
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_users::Migration),
            Box::new(m20220101_000002_sessions::Migration),
            Box::new(m20220101_000003_auth_events::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
