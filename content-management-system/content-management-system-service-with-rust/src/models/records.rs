//! Shared record helpers: `ActiveModelBehavior` impls for every domain
//! entity plus the per-entity `find_*` finders (active = not
//! soft-deleted).
//!
//! The finders return the **loco** `Result` with `Error::NotFound` on
//! a missing row (not `ModelError::EntityNotFound`): loco 0.16's
//! `IntoResponse` catch-all turns an unmapped model error into a 500,
//! so mapping here keeps every `GET /…/{pid}` contract an honest 404
//! (family lesson, pinned in the request tests).

use loco_rs::prelude::*;
use sea_orm::ConnectionTrait;
use uuid::Uuid;

use super::_entities::{
    assets, audience_rules, content_references, content_types, entries, entry_variants, menus,
    preview_tokens, redirects, renditions, revisions, routes, sites, templates, webhook_deliveries,
    webhooks,
};

impl ActiveModelBehavior for sites::ActiveModel {}
impl ActiveModelBehavior for templates::ActiveModel {}
impl ActiveModelBehavior for content_types::ActiveModel {}
impl ActiveModelBehavior for entries::ActiveModel {}
impl ActiveModelBehavior for entry_variants::ActiveModel {}
impl ActiveModelBehavior for content_references::ActiveModel {}
impl ActiveModelBehavior for assets::ActiveModel {}
impl ActiveModelBehavior for renditions::ActiveModel {}
impl ActiveModelBehavior for routes::ActiveModel {}
impl ActiveModelBehavior for redirects::ActiveModel {}
impl ActiveModelBehavior for menus::ActiveModel {}
impl ActiveModelBehavior for audience_rules::ActiveModel {}
impl ActiveModelBehavior for preview_tokens::ActiveModel {}
impl ActiveModelBehavior for webhooks::ActiveModel {}
impl ActiveModelBehavior for webhook_deliveries::ActiveModel {}

// `revisions` is append-only, so its `ActiveModelBehavior` is the
// default too — but note that nothing in this crate ever builds an
// update for it (CMS-D3).
impl ActiveModelBehavior for revisions::ActiveModel {}

/// Parse a path `pid` string to a [`Uuid`], mapping failure to `404`.
///
/// # Errors
///
/// [`Error::NotFound`] when `pid` is not a UUID.
pub fn parse_pid(pid: &str) -> Result<Uuid> {
    Uuid::parse_str(pid).map_err(|_| Error::NotFound)
}

/// Generate the per-entity `find_*` finder: the row with this `pid`
/// whose `deleted_at` is null, else `Error::NotFound`.
macro_rules! find_active_by_pid {
    ($fn_name:ident, $module:ident) => {
        /// Find the active (not soft-deleted) row by public id.
        ///
        /// # Errors
        ///
        /// [`Error::NotFound`] when absent or soft-deleted; any query
        /// error otherwise.
        pub async fn $fn_name<C: ConnectionTrait>(db: &C, pid: Uuid) -> Result<$module::Model> {
            let row = $module::Entity::find()
                .filter($module::Column::Pid.eq(pid))
                .filter($module::Column::DeletedAt.is_null())
                .one(db)
                .await
                .map_err(|e| Error::Model(ModelError::from(e)))?;
            row.ok_or(Error::NotFound)
        }
    };
}

find_active_by_pid!(find_site, sites);
find_active_by_pid!(find_template, templates);
find_active_by_pid!(find_content_type, content_types);
find_active_by_pid!(find_entry, entries);
find_active_by_pid!(find_variant, entry_variants);
find_active_by_pid!(find_asset, assets);
find_active_by_pid!(find_rendition, renditions);
find_active_by_pid!(find_menu, menus);
find_active_by_pid!(find_audience_rule, audience_rules);
find_active_by_pid!(find_webhook, webhooks);

/// Find a revision by public id. Revisions have no `deleted_at` — the
/// chain is append-only (CMS-D3) — so this is a plain lookup.
///
/// # Errors
///
/// [`Error::NotFound`] when absent; any query error otherwise.
pub async fn find_revision<C: ConnectionTrait>(db: &C, pid: Uuid) -> Result<revisions::Model> {
    revisions::Entity::find()
        .filter(revisions::Column::Pid.eq(pid))
        .one(db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?
        .ok_or(Error::NotFound)
}

/// Find the live variant of `entry_pid` in `locale`, else `404`.
///
/// # Errors
///
/// [`Error::NotFound`] when absent or soft-deleted; any query error
/// otherwise.
pub async fn find_variant_by_locale<C: ConnectionTrait>(
    db: &C,
    entry_pid: Uuid,
    locale: &str,
) -> Result<entry_variants::Model> {
    entry_variants::Entity::find()
        .filter(entry_variants::Column::EntryPid.eq(entry_pid))
        .filter(entry_variants::Column::Locale.eq(locale))
        .filter(entry_variants::Column::DeletedAt.is_null())
        .one(db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?
        .ok_or(Error::NotFound)
}

/// Find the active site with this `key` (the delivery namespace's
/// public handle), else `404`.
///
/// # Errors
///
/// [`Error::NotFound`] when absent or soft-deleted; any query error
/// otherwise.
pub async fn find_site_by_key<C: ConnectionTrait>(db: &C, key: &str) -> Result<sites::Model> {
    let row = sites::Entity::find()
        .filter(sites::Column::Key.eq(key))
        .filter(sites::Column::DeletedAt.is_null())
        .one(db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    row.ok_or(Error::NotFound)
}
