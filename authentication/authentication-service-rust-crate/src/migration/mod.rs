//! Database migrations for the authentication service.
//!
//! Embedded in the crate (rather than a separate `migration` workspace
//! crate) so the loco `App` boot, the CLI entrypoint, and the migrations
//! all live in one compilation unit. The [`Migrator`] is handed to
//! `create_app` / `cli::main` in [`crate::app`] and `src/bin/main.rs`.
#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
pub use sea_orm_migration::prelude::*;
/// `users` table — passwordless magic-link accounts.
mod m20220101_000001_users;
/// `sessions` table — issued-token / revocation rows (`jid` = JWT `jti`).
mod m20220101_000002_sessions;
/// `auth_events` table — the authentication audit trail.
mod m20220101_000003_auth_events;
/// `users.deleted_at` column — soft-delete / GDPR-erasure tombstone.
mod m20220101_000004_users_deleted_at;

/// loco/`SeaORM` migrator that runs every migration in order.
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    /// The ordered list of migrations to apply (oldest first). The
    /// `inject-above` marker is where the loco generator appends new
    /// migrations.
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_users::Migration),
            Box::new(m20220101_000002_sessions::Migration),
            Box::new(m20220101_000003_auth_events::Migration),
            Box::new(m20220101_000004_users_deleted_at::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
