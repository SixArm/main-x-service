//! The delivery surface (CMS-R16, CMS-R18, CMS-R19) — the one place
//! this service answers anonymous readers.
//!
//! ## Published revisions only
//!
//! The composer reads `published_revision_pid` and cannot reach a
//! draft. There is no parameter, header, or policy rule that widens it;
//! unpublished content is a different, authenticated path (preview,
//! CMS-T22).
//!
//! ## The public allow-list
//!
//! Everything else in this service is refused without a credential when
//! `CMS_REQUIRE_AUTH` is on. Delivery is the deliberate exception, and
//! it is as narrow as it can be made: `GET`/`HEAD` only, a site whose
//! `visibility` is `public`, published revisions only. The blanket
//! guard defers these paths **because the decision needs a database
//! read** (a site's visibility), and this controller makes it on every
//! request — so flipping a site to `restricted` takes effect on the
//! next request rather than at the next restart.
//!
//! ## Honest caching
//!
//! Responses carry `as_of` and a weak `ETag` over the payload **minus**
//! `as_of`, so an unchanged page keeps its tag as the clock moves. When
//! audience rules consulted any request context, the tag mixes that
//! context in and the response declares `Vary` — a personalized page
//! cached under a key that ignores what personalized it is a data-leak
//! mechanism, not a performance win.

use axum::extract::{Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response as AxumResponse};
use loco_rs::prelude::*;
use sea_orm::{QueryOrder, QuerySelect};
use serde::Deserialize;
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::models::_entities::{
    audience_rules, content_references, content_types, entries, entry_variants, menus, renditions,
    revisions, sites, templates,
};
use crate::models::records;
use crate::rules::audience::{self, Context, Predicate, Rule};
use crate::rules::locale::{self, LocaleConfig};
use crate::rules::seo;

/// Query parameters a channel may declare about itself.
#[derive(Debug, Deserialize, Default)]
struct DeliveryParams {
    /// `web` (default), `app`, `screen`, or `feed`.
    #[serde(default)]
    channel: Option<String>,
    /// A tag the channel asserts about itself — a kiosk's location, a
    /// campaign. Asserted, never inferred.
    #[serde(default)]
    audience_tag: Option<String>,
}

/// Authorize a delivery read: public sites answer anyone; a restricted
/// site needs a credential once enforcement is on.
///
/// With `CMS_REQUIRE_AUTH` off this is a no-op, exactly like the rest of
/// the service — the family's default-off posture, pinned by the
/// exposure test in `auth.rs`.
fn authorize(site: &sites::Model, headers: &HeaderMap) -> Result<(), (StatusCode, String)> {
    if !crate::auth::require_auth() || site.visibility == "public" {
        return Ok(());
    }
    let verifier = crate::auth::verifier().current();
    crate::auth::bearer_claims(headers, &verifier).map(|_| ())
}

/// Turn an authorization refusal into a response.
fn refuse((status, reason): (StatusCode, String)) -> AxumResponse {
    (status, reason).into_response()
}

/// A JSON response with delivery's headers.
fn delivered(
    payload: &Value,
    tag: &str,
    vary: &[String],
    public: bool,
    headers: &HeaderMap,
) -> AxumResponse {
    if super::matches_etag(headers, tag) {
        return StatusCode::NOT_MODIFIED.into_response();
    }
    let mut response = axum::Json(payload.clone()).into_response();
    let out = response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(tag) {
        out.insert(header::ETAG, value);
    }
    // A personalized response must not be cached under a key that
    // ignores what personalized it.
    if !vary.is_empty()
        && let Ok(value) = HeaderValue::from_str(&vary.join(", "))
    {
        out.insert(header::VARY, value);
    }
    out.insert(
        header::CACHE_CONTROL,
        if public {
            HeaderValue::from_static("public, max-age=60")
        } else {
            HeaderValue::from_static("private, no-store")
        },
    );
    response
}

/// The audience rules of a site, in the pure-core shape.
async fn rules_of(db: &DatabaseConnection, site_pid: Uuid) -> Result<Vec<Rule>> {
    let rows = audience_rules::Entity::find()
        .filter(audience_rules::Column::SitePid.eq(site_pid))
        .filter(audience_rules::Column::DeletedAt.is_null())
        .filter(audience_rules::Column::Active.eq(true))
        .order_by_asc(audience_rules::Column::Id)
        .all(db)
        .await?;
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            serde_json::from_value::<Predicate>(row.predicate)
                .ok()
                .map(|predicate| Rule {
                    key: row.key,
                    predicate,
                })
        })
        .collect())
}

/// Summarise one referenced entry — **one hop, no recursion**. A page
/// referencing a page referencing a page returns summaries, which is a
/// `DoS` boundary as much as a design one.
async fn entry_summary(db: &DatabaseConnection, entry_pid: Uuid) -> Result<Option<Value>> {
    let Ok(entry) = records::find_entry(db, entry_pid).await else {
        return Ok(None);
    };
    let variants = entry_variants::Entity::find()
        .filter(entry_variants::Column::EntryPid.eq(entry.pid))
        .filter(entry_variants::Column::DeletedAt.is_null())
        .all(db)
        .await?;
    let published = variants
        .iter()
        .find(|variant| variant.published_revision_pid.is_some());
    let title = match published.and_then(|variant| variant.published_revision_pid) {
        Some(revision_pid) => records::find_revision(db, revision_pid)
            .await
            .ok()
            .map(|revision| revision.title),
        None => None,
    };
    let route = routes_for(db, published).await?;
    Ok(Some(serde_json::json!({
        "entry_pid": entry.pid,
        "key": entry.key,
        "title": title,
        "path": route,
        "published": published.is_some(),
    })))
}

/// The current path of a variant, if it has one.
async fn routes_for(
    db: &DatabaseConnection,
    variant: Option<&entry_variants::Model>,
) -> Result<Option<String>> {
    let Some(variant) = variant else {
        return Ok(None);
    };
    let row = crate::models::_entities::routes::Entity::find()
        .filter(crate::models::_entities::routes::Column::VariantPid.eq(variant.pid))
        .filter(crate::models::_entities::routes::Column::IsCurrent.eq(true))
        .one(db)
        .await?;
    Ok(row.map(|row| row.path))
}

/// Summarise one referenced asset, listing **only the renditions that
/// exist** — a channel picks from what is there rather than guessing a
/// URL pattern.
async fn asset_summary(db: &DatabaseConnection, asset_pid: Uuid) -> Result<Option<Value>> {
    let Ok(asset) = records::find_asset(db, asset_pid).await else {
        return Ok(None);
    };
    let rendition_rows = renditions::Entity::find()
        .filter(renditions::Column::AssetPid.eq(asset.pid))
        .filter(renditions::Column::DeletedAt.is_null())
        .filter(renditions::Column::State.eq("produced"))
        .all(db)
        .await?;
    Ok(Some(serde_json::json!({
        "asset_pid": asset.pid,
        "kind": asset.kind,
        "mime": asset.mime,
        "alt_text": asset.alt_text,
        "caption": asset.caption,
        "credit": asset.credit,
        "width": asset.width,
        "height": asset.height,
        "url": format!("/api/assets/{}/content", asset.pid),
        "renditions": rendition_rows
            .iter()
            .map(|row| serde_json::json!({
                "key": row.key, "width": row.width, "height": row.height, "format": row.format,
            }))
            .collect::<Vec<_>>(),
    })))
}

/// Everything the composer needs for one page.
struct Composition<'a> {
    site: &'a sites::Model,
    entry: &'a entries::Model,
    variant: &'a entry_variants::Model,
    revision: &'a revisions::Model,
    path: &'a str,
    resolution: &'a locale::Resolution,
    evaluation: &'a audience::Evaluation,
}

/// Compose the delivery document for one published revision.
async fn compose(db: &DatabaseConnection, input: &Composition<'_>) -> Result<Value> {
    let Composition {
        site,
        entry,
        variant,
        revision,
        path,
        resolution,
        evaluation,
    } = input;
    let content_type = content_types::Entity::find()
        .filter(content_types::Column::SitePid.eq(site.pid))
        .filter(content_types::Column::Key.eq(entry.content_type_key.clone()))
        .filter(content_types::Column::DeletedAt.is_null())
        .one(db)
        .await?;
    let template = match content_type.as_ref().and_then(|t| t.template_key.clone()) {
        Some(key) => {
            templates::Entity::find()
                .filter(templates::Column::SitePid.eq(site.pid))
                .filter(templates::Column::Key.eq(key))
                .filter(templates::Column::DeletedAt.is_null())
                .one(db)
                .await?
        }
        None => None,
    };

    let edges = content_references::Entity::find()
        .filter(content_references::Column::FromRevisionPid.eq(revision.pid))
        .limit(200)
        .all(db)
        .await?;
    let mut referenced_entries = Vec::new();
    let mut referenced_assets = Vec::new();
    let mut referenced_entities = Vec::new();
    for edge in edges {
        if let Some(pid) = edge.to_entry_pid
            && let Some(summary) = entry_summary(db, pid).await?
        {
            referenced_entries.push(summary);
        }
        if let Some(pid) = edge.to_asset_pid
            && let Some(summary) = asset_summary(db, pid).await?
        {
            referenced_assets.push(summary);
        }
        if let Some(urn) = edge.to_entity_ref {
            referenced_entities.push(Value::String(urn));
        }
    }

    let seo_block: Map<String, Value> = revision.seo.as_object().cloned().unwrap_or_default();
    let declared_canonical = seo_block.get("canonical_url").and_then(Value::as_str);
    let served_locale = resolution
        .locale_served
        .clone()
        .unwrap_or_else(|| variant.locale.clone());

    Ok(serde_json::json!({
        "as_of": chrono::Utc::now(),
        "site": site.key,
        // The honesty fields: a reader asking for French is told when
        // they got English (CMS-R14).
        "locale_requested": resolution.locale_requested,
        "locale_served": served_locale,
        "fallback_applied": resolution.fallback_applied,
        "fallback_chain_walked": resolution.chain_walked,
        "path": path,
        "entry": {
            "pid": entry.pid,
            "key": entry.key,
            "content_type": entry.content_type_key,
        },
        "revision": {
            "pid": revision.pid,
            "number": revision.number,
            "title": revision.title,
            "blocks": revision.blocks,
            "fields": revision.fields,
        },
        "seo": {
            "meta_title": seo_block.get("meta_title"),
            "meta_description": seo_block.get("meta_description"),
            "robots": seo_block.get("robots").and_then(Value::as_str).unwrap_or(&site.robots_default),
            "canonical": seo::canonical(declared_canonical, site.base_url.as_deref(), &served_locale, path),
        },
        // A declared region contract, not markup: this service renders
        // nothing (CMS-D6).
        "template": template.map(|row| serde_json::json!({
            "key": row.key,
            "regions": row.regions,
        })),
        "references": {
            "entries": referenced_entries,
            "assets": referenced_assets,
            "entities": referenced_entities,
        },
        "personalization": {
            "matched_rules": evaluation.matched,
            "consulted_context": evaluation.consulted,
        },
        "published_at": variant.published_at,
        "first_published_at": variant.first_published_at,
    }))
}

/// `GET /delivery/{site}/{locale}/{*path}` — the page.
#[debug_handler]
#[allow(clippy::too_many_lines)] // one composition, read top to bottom
async fn page(
    State(ctx): State<AppContext>,
    Path((site_key, requested_locale, requested_path)): Path<(String, String, String)>,
    Query(params): Query<DeliveryParams>,
    headers: HeaderMap,
) -> Result<AxumResponse> {
    let Ok(site) = records::find_site_by_key(&ctx.db, &site_key).await else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    if let Err(refusal) = authorize(&site, &headers) {
        return Ok(refuse(refusal));
    }
    let public = site.visibility == "public";

    // Resolve the address first: a redirect answers before any locale
    // question arises.
    let landing =
        super::routing::land(&ctx.db, site.pid, &requested_locale, &requested_path).await?;
    match landing.status {
        301 => {
            tracing::debug!(
                hops = landing.hops.len(),
                "delivery followed a redirect chain"
            );
            let target = landing.path.clone();
            let mut response = StatusCode::MOVED_PERMANENTLY.into_response();
            if let Ok(value) =
                HeaderValue::from_str(&format!("/delivery/{site_key}/{requested_locale}{target}"))
            {
                response.headers_mut().insert(header::LOCATION, value);
            }
            return Ok(response);
        }
        410 => return Ok(StatusCode::GONE.into_response()),
        508 => {
            tracing::warn!(
                site = %site.key, path = %requested_path, problem = ?landing.problem,
                "redirect chain could not be resolved"
            );
            return Ok(StatusCode::LOOP_DETECTED.into_response());
        }
        _ => {}
    }
    let (Some(variant), Some(entry)) = (landing.variant, landing.entry) else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };

    // Which locale answers? The variant found at this path is in the
    // requested locale by construction; the fallback question is asked
    // when *that* variant has nothing published.
    let locales: Vec<String> = serde_json::from_value(site.locales.clone()).unwrap_or_default();
    let chains: std::collections::BTreeMap<String, Vec<String>> =
        serde_json::from_value(site.fallback_chains.clone()).unwrap_or_default();
    let chains: Vec<(String, Vec<String>)> = chains.into_iter().collect();
    let strict: Vec<String> =
        serde_json::from_value(site.strict_locales.clone()).unwrap_or_default();
    let sibling_variants = entry_variants::Entity::find()
        .filter(entry_variants::Column::EntryPid.eq(entry.pid))
        .filter(entry_variants::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    let published_locales: Vec<String> = sibling_variants
        .iter()
        .filter(|candidate| candidate.published_revision_pid.is_some())
        .map(|candidate| candidate.locale.clone())
        .collect();
    let resolution = locale::resolve(
        &LocaleConfig {
            default_locale: &site.default_locale,
            locales: &locales,
            fallback_chains: &chains,
            strict_locales: &strict,
        },
        &requested_locale,
        &published_locales,
    );
    let Some(served_locale) = resolution.locale_served.clone() else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    let serving = sibling_variants
        .iter()
        .find(|candidate| candidate.locale == served_locale)
        .unwrap_or(&variant);
    // Published revisions only — the composer cannot reach a draft.
    let Some(revision_pid) = serving.published_revision_pid else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    let revision = records::find_revision(&ctx.db, revision_pid).await?;

    let rules = rules_of(&ctx.db, site.pid).await?;
    let context = Context {
        locale: served_locale.clone(),
        channel: params.channel.clone().unwrap_or_else(|| "web".to_string()),
        audience_tag: params.audience_tag.clone(),
        preview: false,
    };
    let evaluation = audience::evaluate(&rules, &context);

    let payload = compose(
        &ctx.db,
        &Composition {
            site: &site,
            entry: &entry,
            variant: serving,
            revision: &revision,
            path: &landing.path,
            resolution: &resolution,
            evaluation: &evaluation,
        },
    )
    .await?;

    // The tag mixes in exactly the context the rules consulted, and the
    // response varies by the same.
    let salt: String = evaluation
        .consulted
        .iter()
        .filter_map(|key| context.get(key).map(|value| format!("{key}={value};")))
        .collect();
    let vary: Vec<String> = evaluation
        .consulted
        .iter()
        .filter(|key| key.as_str() != "locale")
        .map(|_| "X-CMS-Channel".to_string())
        .collect();
    let tag = super::weak_etag(&payload, &salt);
    Ok(delivered(&payload, &tag, &vary, public, &headers))
}

/// `GET /delivery/{site}/{locale}/menus/{key}` — a resolved menu tree.
///
/// Items whose target is not published are **omitted**: a navigation
/// link into a 404 is worse than a shorter menu.
#[debug_handler]
async fn menu(
    State(ctx): State<AppContext>,
    Path((site_key, locale, key)): Path<(String, String, String)>,
    headers: HeaderMap,
) -> Result<AxumResponse> {
    let Ok(site) = records::find_site_by_key(&ctx.db, &site_key).await else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    if let Err(refusal) = authorize(&site, &headers) {
        return Ok(refuse(refusal));
    }
    let Some(menu) = menus::Entity::find()
        .filter(menus::Column::SitePid.eq(site.pid))
        .filter(menus::Column::Locale.eq(locale.clone()))
        .filter(menus::Column::Key.eq(key.clone()))
        .filter(menus::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await?
    else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };

    let items = menu.items.as_array().cloned().unwrap_or_default();
    let mut resolved = Vec::new();
    for item in items {
        let label = item
            .get("label")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if let Some(url) = item.get("url").and_then(Value::as_str) {
            resolved.push(serde_json::json!({ "label": label, "url": url }));
            continue;
        }
        let Some(entry_pid) = item
            .get("entry_pid")
            .and_then(Value::as_str)
            .and_then(|pid| Uuid::parse_str(pid).ok())
        else {
            continue;
        };
        let Some(summary) = entry_summary(&ctx.db, entry_pid).await? else {
            continue;
        };
        // Omit unpublished targets rather than linking into nothing.
        if summary.get("published").and_then(Value::as_bool) != Some(true) {
            continue;
        }
        resolved.push(serde_json::json!({
            "label": label,
            "path": summary.get("path"),
            "entry_pid": entry_pid,
        }));
    }
    let payload = serde_json::json!({
        "as_of": chrono::Utc::now(),
        "site": site.key,
        "locale": locale,
        "key": key,
        "items": resolved,
        "note": "items whose target is not published are omitted",
    });
    let tag = super::weak_etag(&payload, "");
    Ok(delivered(
        &payload,
        &tag,
        &[],
        site.visibility == "public",
        &headers,
    ))
}

/// `GET /delivery/{site}/sitemap.xml` — derived from what is published.
#[debug_handler]
async fn sitemap(
    State(ctx): State<AppContext>,
    Path(site_key): Path<String>,
    headers: HeaderMap,
) -> Result<AxumResponse> {
    let Ok(site) = records::find_site_by_key(&ctx.db, &site_key).await else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    if let Err(refusal) = authorize(&site, &headers) {
        return Ok(refuse(refusal));
    }

    let entry_rows = entries::Entity::find()
        .filter(entries::Column::SitePid.eq(site.pid))
        .filter(entries::Column::DeletedAt.is_null())
        .limit(5000)
        .all(&ctx.db)
        .await?;
    let mut sitemap_entries = Vec::new();
    for entry in entry_rows {
        let variants = entry_variants::Entity::find()
            .filter(entry_variants::Column::EntryPid.eq(entry.pid))
            .filter(entry_variants::Column::DeletedAt.is_null())
            .filter(entry_variants::Column::PublishedRevisionPid.is_not_null())
            .all(&ctx.db)
            .await?;
        // Reciprocal alternates: every published locale lists every
        // other, which is what a crawler needs and what a one-way
        // alternate gets wrong.
        let mut alternates = Vec::new();
        for variant in &variants {
            if let Some(path) = routes_for(&ctx.db, Some(variant)).await?
                && let Some(url) = seo::absolute(site.base_url.as_deref(), &variant.locale, &path)
            {
                alternates.push((variant.locale.clone(), url));
            }
        }
        for variant in &variants {
            let Some(path) = routes_for(&ctx.db, Some(variant)).await? else {
                continue;
            };
            let Some(revision_pid) = variant.published_revision_pid else {
                continue;
            };
            let revision = records::find_revision(&ctx.db, revision_pid).await?;
            let robots = revision
                .seo
                .as_object()
                .and_then(|seo| seo.get("robots"))
                .and_then(Value::as_str);
            if !seo::is_indexable(robots) {
                continue;
            }
            let Some(location) = seo::absolute(site.base_url.as_deref(), &variant.locale, &path)
            else {
                continue;
            };
            sitemap_entries.push(seo::SitemapEntry {
                location,
                last_modified: Some(revision.created_at.to_rfc3339()),
                alternates: alternates.clone(),
            });
        }
    }

    let xml = seo::render_sitemap(&sitemap_entries);
    let mut response = xml.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/xml; charset=utf-8"),
    );
    Ok(response)
}

/// `GET /delivery/{site}/{locale}/feed.xml` — recently published
/// pages in one locale, as Atom (CMS-R19).
///
/// Behind the same visibility check as every other delivery read, and
/// **published-only** by construction: it reads
/// `published_revision_pid`, so a draft cannot reach it even if the
/// entry is otherwise live. A `noindex` page is excluded too — a feed
/// is a syndication surface, and a page the site asked crawlers to
/// ignore has not asked to be syndicated either.
///
/// Ordered newest first by publication time, capped at
/// [`seo::FEED_LIMIT`].
#[debug_handler]
async fn feed(
    State(ctx): State<AppContext>,
    Path((site_key, locale)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<AxumResponse> {
    let Ok(site) = records::find_site_by_key(&ctx.db, &site_key).await else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    if let Err(refusal) = authorize(&site, &headers) {
        return Ok(refuse(refusal));
    }

    let entry_rows = entries::Entity::find()
        .filter(entries::Column::SitePid.eq(site.pid))
        .filter(entries::Column::DeletedAt.is_null())
        .limit(5000)
        .all(&ctx.db)
        .await?;
    let mut items = Vec::new();
    for entry in entry_rows {
        let Ok(variant) = records::find_variant_by_locale(&ctx.db, entry.pid, &locale).await else {
            continue;
        };
        let Some(revision_pid) = variant.published_revision_pid else {
            continue;
        };
        let Some(path) = routes_for(&ctx.db, Some(&variant)).await? else {
            continue;
        };
        let revision = records::find_revision(&ctx.db, revision_pid).await?;
        let seo_block = revision.seo.as_object();
        let robots = seo_block
            .and_then(|seo| seo.get("robots"))
            .and_then(Value::as_str);
        if !seo::is_indexable(robots) {
            continue;
        }
        let Some(location) = seo::absolute(site.base_url.as_deref(), &variant.locale, &path) else {
            continue;
        };
        // The published time, not the revision's write time: a page
        // that was drafted in March and published in July belongs at
        // July in a feed of what is new.
        let published_at = variant
            .published_at
            .unwrap_or(revision.created_at)
            .to_rfc3339();
        items.push((
            published_at.clone(),
            seo::FeedEntry {
                location,
                title: revision.title.clone(),
                updated: published_at,
                // The entry's `pid`, so a rename does not resurface the
                // page as a new item in every reader.
                id: format!("urn:uuid:{}", entry.pid),
                summary: seo_block
                    .and_then(|seo| seo.get("meta_description"))
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
            },
        ));
    }
    items.sort_by(|a, b| b.0.cmp(&a.0));
    let feed_entries: Vec<seo::FeedEntry> = items.into_iter().map(|(_, entry)| entry).collect();

    let self_url = seo::absolute(site.base_url.as_deref(), &locale, "/feed.xml")
        .unwrap_or_else(|| format!("/delivery/{}/{}/feed.xml", site.key, locale));
    let site_url = seo::absolute(site.base_url.as_deref(), &locale, "/");
    let xml = seo::render_feed(
        &site.name,
        &self_url,
        site_url.as_deref(),
        &feed_entries,
        // A feed with nothing in it still needs an `updated`. The
        // site's own creation time is a fact; "now" would make an
        // unchanged empty feed look fresh on every poll.
        &site.created_at.to_rfc3339(),
    );
    let mut response = xml.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/atom+xml; charset=utf-8"),
    );
    Ok(response)
}

/// `GET /delivery/{site}/robots.txt`.
#[debug_handler]
async fn robots(
    State(ctx): State<AppContext>,
    Path(site_key): Path<String>,
) -> Result<AxumResponse> {
    let Ok(site) = records::find_site_by_key(&ctx.db, &site_key).await else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    // `robots.txt` itself is always answerable: telling a crawler to go
    // away is the point, and refusing to say so achieves nothing.
    let body = seo::render_robots(
        site.visibility == "public",
        site.base_url.as_deref(),
        &site.key,
        &site.robots_default,
    );
    let mut response = body.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    Ok(response)
}

/// The delivery routes — mounted at the root, not under `/api`.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/delivery")
        .add("/{site}/sitemap.xml", get(sitemap))
        .add("/{site}/{locale}/feed.xml", get(feed))
        .add("/{site}/robots.txt", get(robots))
        .add("/{site}/{locale}/menus/{key}", get(menu))
        .add("/{site}/{locale}/{*path}", get(page))
}
