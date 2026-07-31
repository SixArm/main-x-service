//! `sea-orm-migration` schema migrations for
//! `content-management-system-service`.
//!
//! The [`Migrator`] runs the ordered list below at boot: sites and
//! templates, content types, the `audit_logs` and `event_outbox` side
//! tables, then entries with their per-locale variants, append-only
//! revisions, and the extracted reference index, then the asset
//! library with its declared renditions, and the translation
//! workflow's per-variant state, then routes, redirects, menus, and
//! audience rules, then preview tokens and webhooks.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(elided_lifetimes_in_paths)]
pub use sea_orm_migration::prelude::*;
mod m20260730_000001_sites;
mod m20260730_000002_content_types;
mod m20260730_000003_audit_logs;
mod m20260730_000004_event_outbox;
mod m20260730_000005_entries;
mod m20260730_000006_assets;
mod m20260730_000007_translation;
mod m20260730_000008_routing;
mod m20260730_000009_preview;
mod m20260730_000010_webhooks;

/// The crate's migrator.
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    /// The ordered migration set.
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260730_000001_sites::Migration),
            Box::new(m20260730_000002_content_types::Migration),
            Box::new(m20260730_000003_audit_logs::Migration),
            Box::new(m20260730_000004_event_outbox::Migration),
            Box::new(m20260730_000005_entries::Migration),
            Box::new(m20260730_000006_assets::Migration),
            Box::new(m20260730_000007_translation::Migration),
            Box::new(m20260730_000008_routing::Migration),
            Box::new(m20260730_000009_preview::Migration),
            Box::new(m20260730_000010_webhooks::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
