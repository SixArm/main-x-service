//! Sites + templates (CMS-R1): the delivery namespace and the declared
//! presentation contracts. Every mutation runs on one transaction with
//! its audit and outbox rows (CMS-D15).

use std::collections::BTreeMap;

use loco_rs::prelude::*;
use sea_orm::{PaginatorTrait, QueryOrder, QuerySelect, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{conflict, ensure_valid};
use crate::auth::MaybeAuthUser;
use crate::models::_entities::{content_types, sites, templates};
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
use crate::rules::locale::{self, LocaleConfig};
use crate::rules::template::{RegionSpec, validate_regions};
use crate::rules::tokens;
use crate::streaming;
use crate::validation::Problems;

/// `POST`/`PUT /api/sites` body.
#[derive(Debug, Deserialize)]
struct SitePayload {
    key: String,
    name: String,
    #[serde(default)]
    owner_ref: Option<String>,
    default_locale: String,
    locales: Vec<String>,
    /// Per-locale ordered fallback chains; a locale with no entry falls
    /// straight back to the default.
    #[serde(default)]
    fallback_chains: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    strict_locales: Vec<String>,
    /// **Defaults to `restricted`**: a new site is not anonymously
    /// readable until someone says so in as many words (CMS-D7).
    #[serde(default = "default_visibility")]
    visibility: String,
    #[serde(default)]
    base_url: Option<String>,
    #[serde(default = "default_robots")]
    robots_default: String,
    #[serde(default = "default_true")]
    require_distinct_approver: bool,
}

/// `POST`/`PUT` template body.
#[derive(Debug, Deserialize)]
struct TemplatePayload {
    key: String,
    name: String,
    regions: Vec<RegionSpec>,
    #[serde(default)]
    applies_to_type_keys: Vec<String>,
}

/// A `{pid}` reference response.
#[derive(Debug, Serialize)]
struct PidRef {
    pid: String,
}

fn default_visibility() -> String {
    "restricted".to_string()
}
fn default_robots() -> String {
    "index,follow".to_string()
}
const fn default_true() -> bool {
    true
}

/// Validate a site payload: key/name/owner shapes, the token
/// vocabularies, and the whole locale configuration in one pass
/// (`rules::locale`).
fn validate_site(payload: &SitePayload) -> Vec<String> {
    let mut problems = Problems::new();
    problems.require_key("key", &payload.key);
    problems.require_text("name", &payload.name);
    problems.ref_opt(
        "owner_ref",
        entity_ref::EntityType::Organization,
        payload.owner_ref.as_deref(),
    );
    problems.require_token("visibility", tokens::VISIBILITIES, &payload.visibility);
    problems.require_token("robots_default", tokens::ROBOTS, &payload.robots_default);
    problems.url_opt("base_url", payload.base_url.as_deref());
    let mut problems = problems.into_vec();
    let chains: Vec<(String, Vec<String>)> = payload
        .fallback_chains
        .iter()
        .map(|(locale, chain)| (locale.clone(), chain.clone()))
        .collect();
    problems.extend(locale::validate(&LocaleConfig {
        default_locale: &payload.default_locale,
        locales: &payload.locales,
        fallback_chains: &chains,
        strict_locales: &payload.strict_locales,
    }));
    problems
}

/// Serialize a payload's locale lists to the stored JSONB columns.
fn locale_columns(
    payload: &SitePayload,
) -> (serde_json::Value, serde_json::Value, serde_json::Value) {
    (
        serde_json::json!(payload.locales),
        serde_json::json!(payload.fallback_chains),
        serde_json::json!(payload.strict_locales),
    )
}

/// Recount the live `public` sites into the gauge, after any change
/// that could move the number.
///
/// Counting on each site mutation rather than incrementing a counter is
/// deliberate: sites change rarely, and "how many sites are anonymously
/// readable" is a number an operator may alert on, so it must be the
/// truth about the database rather than a running total that drifts the
/// first time a write is rolled back. A failed count logs and leaves the
/// previous value rather than failing the request — but it never
/// silently reports zero, which is the reading that would wrongly say
/// "nothing is exposed".
async fn refresh_public_site_gauge(db: &DatabaseConnection) {
    match sites::Entity::find()
        .filter(sites::Column::DeletedAt.is_null())
        .filter(sites::Column::Visibility.eq("public"))
        .count(db)
        .await
    {
        Ok(count) => crate::metrics::Metrics::global()
            .sites_public
            .set(i64::try_from(count).unwrap_or(i64::MAX)),
        Err(error) => {
            tracing::warn!(%error, "could not refresh the public-site gauge; keeping the last value");
        }
    }
}

/// `POST /api/sites` — declare a delivery namespace.
#[debug_handler]
async fn create_site(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<SitePayload>,
) -> Result<Response> {
    ensure_valid(&validate_site(&payload))?;
    if records::find_site_by_key(&ctx.db, &payload.key)
        .await
        .is_ok()
    {
        return Err(conflict(&format!(
            "site key {:?} is already in use",
            payload.key
        )));
    }
    let (locales, chains, strict) = locale_columns(&payload);
    let txn = ctx.db.begin().await?;
    let row = sites::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        key: ActiveValue::set(payload.key.clone()),
        name: ActiveValue::set(payload.name.clone()),
        owner_ref: ActiveValue::set(payload.owner_ref.clone()),
        default_locale: ActiveValue::set(payload.default_locale.clone()),
        locales: ActiveValue::set(locales),
        fallback_chains: ActiveValue::set(chains),
        strict_locales: ActiveValue::set(strict),
        visibility: ActiveValue::set(payload.visibility.clone()),
        base_url: ActiveValue::set(payload.base_url.clone()),
        robots_default: ActiveValue::set(payload.robots_default.clone()),
        require_distinct_approver: ActiveValue::set(payload.require_distinct_approver),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(
        &txn,
        "site",
        row.pid,
        "created",
        caller.actor(),
        Some(serde_json::json!({
            "key": row.key,
            "visibility": row.visibility,
            "owner": row.owner_ref,
        })),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "site",
        "site_configured",
        &row.pid.to_string(),
        &row.name,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    crate::metrics::Metrics::global().site_created_total.inc();
    refresh_public_site_gauge(&ctx.db).await;
    format::json(PidRef {
        pid: row.pid.to_string(),
    })
}

/// `GET /api/sites` — live sites.
#[debug_handler]
async fn list_sites(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = sites::Entity::find()
        .filter(sites::Column::DeletedAt.is_null())
        .order_by_asc(sites::Column::Id)
        .limit(1000)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

/// `GET /api/sites/{pid}` — one site with its templates and content
/// types (the operator's whole namespace in one read).
#[debug_handler]
async fn get_site(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let site = records::find_site(&ctx.db, records::parse_pid(&pid)?).await?;
    let template_rows = templates::Entity::find()
        .filter(templates::Column::SitePid.eq(site.pid))
        .filter(templates::Column::DeletedAt.is_null())
        .order_by_asc(templates::Column::Id)
        .all(&ctx.db)
        .await?;
    let type_rows = content_types::Entity::find()
        .filter(content_types::Column::SitePid.eq(site.pid))
        .filter(content_types::Column::DeletedAt.is_null())
        .order_by_asc(content_types::Column::Id)
        .all(&ctx.db)
        .await?;
    format::json(serde_json::json!({
        "site": site,
        "templates": template_rows,
        "content_types": type_rows,
    }))
}

/// `PUT /api/sites/{pid}` — replace the site's configuration.
///
/// A **visibility change is recorded explicitly** in the audit
/// snapshot, because flipping `restricted → public` is the single edit
/// that changes who may read this site's published content without a
/// credential (CMS-D7).
#[debug_handler]
async fn update_site(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    caller: MaybeAuthUser,
    Json(payload): Json<SitePayload>,
) -> Result<Response> {
    let site = records::find_site(&ctx.db, records::parse_pid(&pid)?).await?;
    ensure_valid(&validate_site(&payload))?;
    if payload.key != site.key
        && records::find_site_by_key(&ctx.db, &payload.key)
            .await
            .is_ok()
    {
        return Err(conflict(&format!(
            "site key {:?} is already in use",
            payload.key
        )));
    }
    let (locales, chains, strict) = locale_columns(&payload);
    let visibility_changed = payload.visibility != site.visibility;
    let previous_visibility = site.visibility.clone();
    let txn = ctx.db.begin().await?;
    let mut active: sites::ActiveModel = site.into();
    active.key = ActiveValue::set(payload.key.clone());
    active.name = ActiveValue::set(payload.name.clone());
    active.owner_ref = ActiveValue::set(payload.owner_ref.clone());
    active.default_locale = ActiveValue::set(payload.default_locale.clone());
    active.locales = ActiveValue::set(locales);
    active.fallback_chains = ActiveValue::set(chains);
    active.strict_locales = ActiveValue::set(strict);
    active.visibility = ActiveValue::set(payload.visibility.clone());
    active.base_url = ActiveValue::set(payload.base_url.clone());
    active.robots_default = ActiveValue::set(payload.robots_default.clone());
    active.require_distinct_approver = ActiveValue::set(payload.require_distinct_approver);
    let row = active.update(&txn).await?;
    Audit::record(
        &txn,
        "site",
        row.pid,
        if visibility_changed {
            "visibility_changed"
        } else {
            "updated"
        },
        caller.actor(),
        Some(serde_json::json!({
            "key": row.key,
            "visibility_from": previous_visibility,
            "visibility_to": row.visibility,
        })),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "site",
        "site_configured",
        &row.pid.to_string(),
        &row.name,
        caller.actor(),
        Some(serde_json::json!({ "visibility": row.visibility })),
    )
    .await?;
    txn.commit().await?;
    refresh_public_site_gauge(&ctx.db).await;
    format::json(row)
}

/// `DELETE /api/sites/{pid}` — soft-delete, **refused** while the site
/// still holds templates or content types. Orphaning a namespace's
/// children would leave content nothing can serve and nothing can
/// explain (CMS-D8's delete-refusal posture).
#[debug_handler]
async fn delete_site(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let site = records::find_site(&ctx.db, records::parse_pid(&pid)?).await?;
    let type_count = content_types::Entity::find()
        .filter(content_types::Column::SitePid.eq(site.pid))
        .filter(content_types::Column::DeletedAt.is_null())
        .count(&ctx.db)
        .await?;
    let template_count = templates::Entity::find()
        .filter(templates::Column::SitePid.eq(site.pid))
        .filter(templates::Column::DeletedAt.is_null())
        .count(&ctx.db)
        .await?;
    if type_count > 0 || template_count > 0 {
        return Err(conflict(&format!(
            "site {} still holds {type_count} content type(s) and {template_count} template(s)",
            site.key
        )));
    }
    let txn = ctx.db.begin().await?;
    let site_pid = site.pid;
    let name = site.name.clone();
    let mut active: sites::ActiveModel = site.into();
    active.deleted_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    active.update(&txn).await?;
    Audit::record(&txn, "site", site_pid, "deleted", caller.actor(), None).await?;
    streaming::emit_on(
        &txn,
        "site",
        "deleted",
        &site_pid.to_string(),
        &name,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    refresh_public_site_gauge(&ctx.db).await;
    format::empty_json()
}

/// `POST /api/sites/{pid}/templates` — declare a region contract.
#[debug_handler]
async fn create_template(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    caller: MaybeAuthUser,
    Json(payload): Json<TemplatePayload>,
) -> Result<Response> {
    let site = records::find_site(&ctx.db, records::parse_pid(&pid)?).await?;
    let mut problems = Problems::new();
    problems.require_key("key", &payload.key);
    problems.require_text("name", &payload.name);
    problems.cap_list("applies_to_type_keys", &payload.applies_to_type_keys);
    let mut problems = problems.into_vec();
    problems.extend(validate_regions(&payload.regions));
    ensure_valid(&problems)?;
    if find_template_by_key(&ctx.db, site.pid, &payload.key)
        .await?
        .is_some()
    {
        return Err(conflict(&format!(
            "template key {:?} is already in use on site {}",
            payload.key, site.key
        )));
    }
    let txn = ctx.db.begin().await?;
    let row = templates::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        site_pid: ActiveValue::set(site.pid),
        key: ActiveValue::set(payload.key.clone()),
        name: ActiveValue::set(payload.name.clone()),
        regions: ActiveValue::set(serde_json::json!(payload.regions)),
        applies_to_type_keys: ActiveValue::set(serde_json::json!(payload.applies_to_type_keys)),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(
        &txn,
        "template",
        row.pid,
        "created",
        caller.actor(),
        Some(serde_json::json!({ "site": site.key, "key": row.key })),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "template",
        "created",
        &row.pid.to_string(),
        &row.name,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: row.pid.to_string(),
    })
}

/// `GET /api/sites/{pid}/templates` — the site's live templates.
#[debug_handler]
async fn list_templates(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
) -> Result<Response> {
    let site = records::find_site(&ctx.db, records::parse_pid(&pid)?).await?;
    let rows = templates::Entity::find()
        .filter(templates::Column::SitePid.eq(site.pid))
        .filter(templates::Column::DeletedAt.is_null())
        .order_by_asc(templates::Column::Id)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

/// `GET /api/templates/{pid}`.
#[debug_handler]
async fn get_template(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    format::json(records::find_template(&ctx.db, records::parse_pid(&pid)?).await?)
}

/// `PUT /api/templates/{pid}` — replace the region contract.
#[debug_handler]
async fn update_template(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    caller: MaybeAuthUser,
    Json(payload): Json<TemplatePayload>,
) -> Result<Response> {
    let template = records::find_template(&ctx.db, records::parse_pid(&pid)?).await?;
    let mut problems = Problems::new();
    problems.require_key("key", &payload.key);
    problems.require_text("name", &payload.name);
    problems.cap_list("applies_to_type_keys", &payload.applies_to_type_keys);
    let mut problems = problems.into_vec();
    problems.extend(validate_regions(&payload.regions));
    ensure_valid(&problems)?;
    if payload.key != template.key
        && let Some(other) = find_template_by_key(&ctx.db, template.site_pid, &payload.key).await?
        && other.pid != template.pid
    {
        return Err(conflict(&format!(
            "template key {:?} is already in use on this site",
            payload.key
        )));
    }
    let txn = ctx.db.begin().await?;
    let mut active: templates::ActiveModel = template.into();
    active.key = ActiveValue::set(payload.key.clone());
    active.name = ActiveValue::set(payload.name.clone());
    active.regions = ActiveValue::set(serde_json::json!(payload.regions));
    active.applies_to_type_keys = ActiveValue::set(serde_json::json!(payload.applies_to_type_keys));
    let row = active.update(&txn).await?;
    Audit::record(
        &txn,
        "template",
        row.pid,
        "updated",
        caller.actor(),
        Some(serde_json::json!({ "key": row.key })),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "template",
        "updated",
        &row.pid.to_string(),
        &row.name,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `DELETE /api/templates/{pid}` — soft-delete, **refused** while a
/// live content type still names this template.
#[debug_handler]
async fn delete_template(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let template = records::find_template(&ctx.db, records::parse_pid(&pid)?).await?;
    let users = content_types::Entity::find()
        .filter(content_types::Column::SitePid.eq(template.site_pid))
        .filter(content_types::Column::TemplateKey.eq(template.key.clone()))
        .filter(content_types::Column::DeletedAt.is_null())
        .count(&ctx.db)
        .await?;
    if users > 0 {
        return Err(conflict(&format!(
            "template {} is still used by {users} content type(s)",
            template.key
        )));
    }
    let txn = ctx.db.begin().await?;
    let template_pid = template.pid;
    let name = template.name.clone();
    let mut active: templates::ActiveModel = template.into();
    active.deleted_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    active.update(&txn).await?;
    Audit::record(
        &txn,
        "template",
        template_pid,
        "deleted",
        caller.actor(),
        None,
    )
    .await?;
    streaming::emit_on(
        &txn,
        "template",
        "deleted",
        &template_pid.to_string(),
        &name,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::empty_json()
}

/// The live template with this key on this site, if any.
async fn find_template_by_key(
    db: &DatabaseConnection,
    site_pid: Uuid,
    key: &str,
) -> Result<Option<templates::Model>> {
    let row = templates::Entity::find()
        .filter(templates::Column::SitePid.eq(site_pid))
        .filter(templates::Column::Key.eq(key))
        .filter(templates::Column::DeletedAt.is_null())
        .one(db)
        .await?;
    Ok(row)
}

/// The site + template routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/sites", post(create_site).get(list_sites))
        .add(
            "/sites/{pid}",
            get(get_site).put(update_site).delete(delete_site),
        )
        .add(
            "/sites/{pid}/templates",
            post(create_template).get(list_templates),
        )
        .add(
            "/templates/{pid}",
            get(get_template)
                .put(update_template)
                .delete(delete_template),
        )
}
