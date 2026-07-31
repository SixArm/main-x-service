//! Routes, redirects, menus, and audience rules (CMS-R17, CMS-R18,
//! CMS-R20).
//!
//! ## Renaming a page leaves a redirect
//!
//! Changing a variant's path automatically creates a `301` from the old
//! one. This is the default, not an option, because renaming a page
//! without leaving a redirect is the most common self-inflicted injury
//! in a CMS: every inbound link, bookmark, and search result breaks at
//! once, and nobody finds out until the traffic disappears.
//!
//! ## Loops are refused at write time
//!
//! A redirect that would close a cycle is a `422` here, not a hang at
//! request time. New redirects are also **collapsed to the end of the
//! chain**, so resolution stays one lookup however many times a page
//! has been renamed.

use loco_rs::prelude::*;
use sea_orm::{QueryOrder, QuerySelect, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{conflict, ensure_valid, unprocessable};
use crate::auth::MaybeAuthUser;
use crate::models::_entities::{audience_rules, entries, entry_variants, menus, redirects, routes};
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
use crate::rules::audience::{self, Predicate};
use crate::rules::path;
use crate::streaming;
use crate::validation::Problems;

/// `PUT …/path` body.
#[derive(Debug, Deserialize)]
struct PathPayload {
    path: String,
}

/// `POST /api/sites/{pid}/redirects` body.
#[derive(Debug, Deserialize)]
struct RedirectPayload {
    locale: String,
    from_path: String,
    /// Absent means a `410 Gone` marker.
    #[serde(default)]
    to_path: Option<String>,
    #[serde(default = "default_status")]
    status: i32,
}

const fn default_status() -> i32 {
    301
}

/// `POST /api/sites/{pid}/menus` body.
#[derive(Debug, Deserialize)]
struct MenuPayload {
    locale: String,
    key: String,
    #[serde(default)]
    items: Vec<MenuItem>,
}

/// One menu item.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct MenuItem {
    label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    entry_pid: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    audience_rule_key: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    children: Vec<MenuItem>,
}

/// `POST /api/sites/{pid}/audience-rules` body.
#[derive(Debug, Deserialize)]
struct AudienceRulePayload {
    key: String,
    name: String,
    predicate: Predicate,
    #[serde(default = "default_true")]
    active: bool,
}

const fn default_true() -> bool {
    true
}

/// Every redirect on a site and locale, in the pure-core shape.
pub(crate) async fn redirect_table<C: sea_orm::ConnectionTrait>(
    db: &C,
    site_pid: Uuid,
    locale: &str,
) -> Result<Vec<path::Redirect>> {
    let rows = redirects::Entity::find()
        .filter(redirects::Column::SitePid.eq(site_pid))
        .filter(redirects::Column::Locale.eq(locale))
        .limit(5000)
        .all(db)
        .await?;
    Ok(rows
        .into_iter()
        .map(|row| path::Redirect {
            from: row.from_path,
            to: row.to_path,
            status: u16::try_from(row.status).unwrap_or(301),
        })
        .collect())
}

/// The hop cap, from `CMS_REDIRECT_MAX_HOPS`.
pub(crate) fn max_hops() -> usize {
    std::env::var("CMS_REDIRECT_MAX_HOPS")
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(path::DEFAULT_MAX_HOPS)
}

/// `PUT /api/entries/{pid}/variants/{locale}/path` — set or change a
/// variant's published address.
#[debug_handler]
#[allow(clippy::too_many_lines)] // one route change, applied end to end
async fn set_path(
    State(ctx): State<AppContext>,
    Path((pid, locale)): Path<(String, String)>,
    caller: MaybeAuthUser,
    Json(payload): Json<PathPayload>,
) -> Result<Response> {
    let entry = records::find_entry(&ctx.db, records::parse_pid(&pid)?).await?;
    let variant = records::find_variant_by_locale(&ctx.db, entry.pid, &locale).await?;
    let content_type =
        super::entries::content_type_of(&ctx.db, entry.site_pid, &entry.content_type_key).await?;
    if !content_type.routable {
        return Err(unprocessable(&format!(
            "content type {:?} is not routable, so its entries have no address",
            content_type.key
        )));
    }
    let normalized = path::normalize(&payload.path).map_err(|problem| unprocessable(&problem))?;

    let txn = ctx.db.begin().await?;
    // Another live page already at this address is a conflict, not a
    // silent takeover.
    if let Some(existing) = routes::Entity::find()
        .filter(routes::Column::SitePid.eq(entry.site_pid))
        .filter(routes::Column::Locale.eq(locale.clone()))
        .filter(routes::Column::Path.eq(normalized.clone()))
        .filter(routes::Column::IsCurrent.eq(true))
        .one(&txn)
        .await?
        && existing.variant_pid != variant.pid
    {
        txn.rollback().await?;
        return Err(conflict(&format!(
            "{normalized} is already the address of another page in {locale}"
        )));
    }

    let previous = routes::Entity::find()
        .filter(routes::Column::VariantPid.eq(variant.pid))
        .filter(routes::Column::IsCurrent.eq(true))
        .one(&txn)
        .await?;
    if previous.as_ref().is_some_and(|row| row.path == normalized) {
        txn.rollback().await?;
        return format::json(serde_json::json!({
            "path": normalized,
            "changed": false,
            "redirect_created": false,
        }));
    }

    // Retire the old route and leave a redirect from it. Keeping the
    // old row (rather than deleting it) is what makes a page's address
    // history answerable.
    let mut redirect_created = false;
    if let Some(previous) = previous {
        let old_path = previous.path.clone();
        let mut retired: routes::ActiveModel = previous.into();
        retired.is_current = ActiveValue::set(false);
        retired.update(&txn).await?;

        let table = redirect_table(&txn, entry.site_pid, &locale).await?;
        if path::would_cycle(&old_path, &normalized, &table) {
            txn.rollback().await?;
            return Err(unprocessable(&format!(
                "redirecting {old_path} to {normalized} would create a loop"
            )));
        }
        // Point at the end of the chain, so resolution stays one lookup.
        let target = path::collapse(&normalized, &table, max_hops());
        redirects::Entity::delete_many()
            .filter(redirects::Column::SitePid.eq(entry.site_pid))
            .filter(redirects::Column::Locale.eq(locale.clone()))
            .filter(redirects::Column::FromPath.eq(old_path.clone()))
            .exec(&txn)
            .await?;
        redirects::ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            site_pid: ActiveValue::set(entry.site_pid),
            locale: ActiveValue::set(locale.clone()),
            from_path: ActiveValue::set(old_path.clone()),
            to_path: ActiveValue::set(Some(target.clone())),
            status: ActiveValue::set(301),
            reason: ActiveValue::set("slug_change".to_string()),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
        redirect_created = true;

        // Anything that pointed at the old address now points at the
        // new one, so a rename does not lengthen every existing chain.
        redirects::Entity::update_many()
            .col_expr(
                redirects::Column::ToPath,
                sea_orm::sea_query::Expr::value(Some(target.clone())),
            )
            .filter(redirects::Column::SitePid.eq(entry.site_pid))
            .filter(redirects::Column::Locale.eq(locale.clone()))
            .filter(redirects::Column::ToPath.eq(old_path))
            .exec(&txn)
            .await?;
    }

    let route = routes::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        site_pid: ActiveValue::set(entry.site_pid),
        locale: ActiveValue::set(locale.clone()),
        path: ActiveValue::set(normalized.clone()),
        variant_pid: ActiveValue::set(variant.pid),
        is_current: ActiveValue::set(true),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    // A path can free up when a page is renamed away from it; a stale
    // redirect out of the newly-claimed address would send readers away
    // from the page that now lives there.
    redirects::Entity::delete_many()
        .filter(redirects::Column::SitePid.eq(entry.site_pid))
        .filter(redirects::Column::Locale.eq(locale.clone()))
        .filter(redirects::Column::FromPath.eq(normalized.clone()))
        .exec(&txn)
        .await?;

    Audit::record(
        &txn,
        "route",
        route.pid,
        "route_changed",
        caller.actor(),
        Some(serde_json::json!({
            "entry": entry.key,
            "locale": locale,
            "path": normalized,
            "redirect_created": redirect_created,
        })),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "route",
        "route_changed",
        &route.pid.to_string(),
        &entry.key,
        caller.actor(),
        Some(serde_json::json!({ "path": normalized, "locale": locale })),
    )
    .await?;
    txn.commit().await?;
    format::json(serde_json::json!({
        "path": normalized,
        "changed": true,
        "redirect_created": redirect_created,
    }))
}

/// `GET /api/sites/{pid}/routes` — the live address book.
#[debug_handler]
async fn list_routes(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let site = records::find_site(&ctx.db, records::parse_pid(&pid)?).await?;
    let rows = routes::Entity::find()
        .filter(routes::Column::SitePid.eq(site.pid))
        .filter(routes::Column::IsCurrent.eq(true))
        .order_by_asc(routes::Column::Path)
        .limit(2000)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

/// `POST /api/sites/{pid}/redirects` — declare one by hand.
#[debug_handler]
async fn create_redirect(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    caller: MaybeAuthUser,
    Json(payload): Json<RedirectPayload>,
) -> Result<Response> {
    let site = records::find_site(&ctx.db, records::parse_pid(&pid)?).await?;
    let from = path::normalize(&payload.from_path).map_err(|p| unprocessable(&p))?;
    let to = match &payload.to_path {
        Some(to) => Some(path::normalize(to).map_err(|p| unprocessable(&p))?),
        None => None,
    };
    if !matches!(payload.status, 301 | 302 | 410) {
        return Err(unprocessable("status must be 301, 302, or 410"));
    }
    if payload.status == 410 && to.is_some() {
        return Err(unprocessable(
            "a 410 marker says the page is gone; it cannot also have a target",
        ));
    }
    if payload.status != 410 && to.is_none() {
        return Err(unprocessable("a redirect needs a target (or status 410)"));
    }

    let txn = ctx.db.begin().await?;
    let table = redirect_table(&txn, site.pid, &payload.locale).await?;
    if let Some(to) = &to
        && path::would_cycle(&from, to, &table)
    {
        txn.rollback().await?;
        return Err(unprocessable(&format!(
            "redirecting {from} to {to} would create a loop"
        )));
    }
    let collapsed = to.as_ref().map(|to| path::collapse(to, &table, max_hops()));
    redirects::Entity::delete_many()
        .filter(redirects::Column::SitePid.eq(site.pid))
        .filter(redirects::Column::Locale.eq(payload.locale.clone()))
        .filter(redirects::Column::FromPath.eq(from.clone()))
        .exec(&txn)
        .await?;
    let row = redirects::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        site_pid: ActiveValue::set(site.pid),
        locale: ActiveValue::set(payload.locale.clone()),
        from_path: ActiveValue::set(from.clone()),
        to_path: ActiveValue::set(collapsed.clone()),
        status: ActiveValue::set(payload.status),
        reason: ActiveValue::set("manual".to_string()),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(
        &txn,
        "redirect",
        row.pid,
        "created",
        caller.actor(),
        Some(serde_json::json!({
            "site": site.key, "from": from, "to": collapsed, "status": payload.status,
        })),
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `GET /api/sites/{pid}/redirects`.
#[debug_handler]
async fn list_redirects(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
) -> Result<Response> {
    let site = records::find_site(&ctx.db, records::parse_pid(&pid)?).await?;
    let rows = redirects::Entity::find()
        .filter(redirects::Column::SitePid.eq(site.pid))
        .order_by_asc(redirects::Column::FromPath)
        .limit(2000)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

/// `DELETE /api/redirects/{pid}`.
#[debug_handler]
async fn delete_redirect(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let target = records::parse_pid(&pid)?;
    let txn = ctx.db.begin().await?;
    let deleted = redirects::Entity::delete_many()
        .filter(redirects::Column::Pid.eq(target))
        .exec(&txn)
        .await?;
    if deleted.rows_affected == 0 {
        txn.rollback().await?;
        return Err(Error::NotFound);
    }
    Audit::record(&txn, "redirect", target, "deleted", caller.actor(), None).await?;
    txn.commit().await?;
    format::empty_json()
}

/// `POST /api/sites/{pid}/menus` — declare navigation.
#[debug_handler]
async fn create_menu(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    caller: MaybeAuthUser,
    Json(payload): Json<MenuPayload>,
) -> Result<Response> {
    let site = records::find_site(&ctx.db, records::parse_pid(&pid)?).await?;
    let mut problems = Problems::new();
    problems.require_key("key", &payload.key);
    for item in &payload.items {
        problems.require_text("items[].label", &item.label);
        if item.entry_pid.is_none() && item.url.is_none() {
            problems.push("each menu item needs an entry_pid or a url".to_string());
        }
        if item.entry_pid.is_some() && item.url.is_some() {
            problems.push("a menu item targets an entry or a url, not both".to_string());
        }
    }
    ensure_valid(&problems.into_vec())?;

    let txn = ctx.db.begin().await?;
    let row = menus::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        site_pid: ActiveValue::set(site.pid),
        locale: ActiveValue::set(payload.locale.clone()),
        key: ActiveValue::set(payload.key.clone()),
        items: ActiveValue::set(serde_json::json!(payload.items)),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(
        &txn,
        "menu",
        row.pid,
        "created",
        caller.actor(),
        Some(serde_json::json!({ "site": site.key, "key": row.key, "locale": row.locale })),
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `POST /api/sites/{pid}/audience-rules` — declare a personalization
/// rule.
#[debug_handler]
async fn create_audience_rule(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    caller: MaybeAuthUser,
    Json(payload): Json<AudienceRulePayload>,
) -> Result<Response> {
    let site = records::find_site(&ctx.db, records::parse_pid(&pid)?).await?;
    let mut problems = Problems::new();
    problems.require_key("key", &payload.key);
    problems.require_text("name", &payload.name);
    let mut problems = problems.into_vec();
    problems.extend(audience::validate(&payload.predicate));
    ensure_valid(&problems)?;

    let txn = ctx.db.begin().await?;
    let row = audience_rules::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        site_pid: ActiveValue::set(site.pid),
        key: ActiveValue::set(payload.key.clone()),
        name: ActiveValue::set(payload.name.clone()),
        predicate: ActiveValue::set(serde_json::json!(payload.predicate)),
        active: ActiveValue::set(payload.active),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(
        &txn,
        "audience_rule",
        row.pid,
        "created",
        caller.actor(),
        Some(serde_json::json!({ "site": site.key, "key": row.key })),
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `GET /api/sites/{pid}/audience-rules`.
#[debug_handler]
async fn list_audience_rules(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
) -> Result<Response> {
    let site = records::find_site(&ctx.db, records::parse_pid(&pid)?).await?;
    let rows = audience_rules::Entity::find()
        .filter(audience_rules::Column::SitePid.eq(site.pid))
        .filter(audience_rules::Column::DeletedAt.is_null())
        .order_by_asc(audience_rules::Column::Id)
        .all(&ctx.db)
        .await?;
    format::json(serde_json::json!({
        "rules": rows,
        "context_keys": audience::CONTEXT_KEYS,
        "note": "personalization reads only these request-context keys — no cookies, IPs, user agents, or referrers, and no visitor identity exists in this service",
    }))
}

/// `GET /api/sites/{pid}/menus` — declared navigation.
#[debug_handler]
async fn list_menus(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let site = records::find_site(&ctx.db, records::parse_pid(&pid)?).await?;
    let rows = menus::Entity::find()
        .filter(menus::Column::SitePid.eq(site.pid))
        .filter(menus::Column::DeletedAt.is_null())
        .order_by_asc(menus::Column::Id)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

/// The variant that answers `path` on this site and locale, following
/// redirects. Shared with the delivery controller.
pub(crate) struct Landing {
    /// The variant, when a page answers.
    pub variant: Option<entry_variants::Model>,
    /// The entry behind it.
    pub entry: Option<entries::Model>,
    /// The final path after redirects.
    pub path: String,
    /// What the caller should be told: 200, 301, 404, 410, 508.
    pub status: u16,
    /// The redirect hops walked.
    pub hops: Vec<String>,
    /// Why the walk failed, when it did.
    pub problem: Option<&'static str>,
}

/// Resolve a path to the page that answers it.
pub(crate) async fn land<C: sea_orm::ConnectionTrait>(
    db: &C,
    site_pid: Uuid,
    locale: &str,
    requested: &str,
) -> Result<Landing> {
    let normalized = path::normalize(requested).unwrap_or_else(|_| requested.to_string());
    let table = redirect_table(db, site_pid, locale).await?;
    let followed = path::follow(&normalized, &table, max_hops());
    let Some(target) = followed.target.clone() else {
        return Ok(Landing {
            variant: None,
            entry: None,
            path: normalized,
            status: followed.status,
            hops: followed.hops,
            problem: followed.problem,
        });
    };

    let route = routes::Entity::find()
        .filter(routes::Column::SitePid.eq(site_pid))
        .filter(routes::Column::Locale.eq(locale))
        .filter(routes::Column::Path.eq(target.clone()))
        .filter(routes::Column::IsCurrent.eq(true))
        .one(db)
        .await?;
    let Some(route) = route else {
        return Ok(Landing {
            variant: None,
            entry: None,
            path: target,
            status: 404,
            hops: followed.hops,
            problem: None,
        });
    };
    let variant = entry_variants::Entity::find()
        .filter(entry_variants::Column::Pid.eq(route.variant_pid))
        .filter(entry_variants::Column::DeletedAt.is_null())
        .one(db)
        .await?;
    let entry = match &variant {
        Some(variant) => {
            entries::Entity::find()
                .filter(entries::Column::Pid.eq(variant.entry_pid))
                .filter(entries::Column::DeletedAt.is_null())
                .one(db)
                .await?
        }
        None => None,
    };
    Ok(Landing {
        status: if followed.hops.is_empty() { 200 } else { 301 },
        variant,
        entry,
        path: target,
        hops: followed.hops,
        problem: None,
    })
}

/// The routing routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/entries/{pid}/variants/{locale}/path", put(set_path))
        .add("/sites/{pid}/routes", get(list_routes))
        .add(
            "/sites/{pid}/redirects",
            post(create_redirect).get(list_redirects),
        )
        .add("/redirects/{pid}", delete(delete_redirect))
        .add("/sites/{pid}/menus", post(create_menu).get(list_menus))
        .add(
            "/sites/{pid}/audience-rules",
            post(create_audience_rule).get(list_audience_rules),
        )
}
