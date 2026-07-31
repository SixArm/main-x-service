//! Content insights (CMS-R21, CMS-D13): content health and editorial
//! throughput.
//!
//! Two things this module is **not**:
//!
//! - **Not reader analytics.** There are no visits here, because this
//!   service records none and holds no visitor identity to attach them
//!   to. These are editorial insights about content, not about people
//!   reading it.
//! - **Not stored.** Every number is derived on read from recorded
//!   facts, so there is no `is_stale` or `health_score` column to fall
//!   out of date with the thing it describes.
//!
//! Every finding carries the rule that produced it, and the response
//! ships the rule explanations, so a dashboard shows the same sentence
//! the code applied and an editor can argue with the rule rather than
//! guess at it.
//!
//! Time-in-state comes from **audit rows recording transitions**, not
//! from `updated_at` — a column that moves for unrelated reasons and
//! would quietly turn "time in review" into "time since anything
//! happened".

use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response as AxumResponse};
use loco_rs::prelude::*;
use sea_orm::{QueryOrder, QuerySelect};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use uuid::Uuid;

use crate::models::_entities::{
    assets, audit_logs, content_references, entries, entry_variants, menus, redirects, revisions,
};
use crate::models::records;
use crate::models::usage;
use crate::rules::insight::{self, Finding};
use crate::rules::{schema, seo, staleness};

/// How long a published page may go unrevised before it is called
/// stale.
fn stale_content_days() -> i64 {
    std::env::var("CMS_STALE_CONTENT_DAYS")
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(365)
}

/// How long a variant may sit in review before it is called stuck.
fn review_window_days() -> i64 {
    std::env::var("CMS_REVIEW_SLA_DAYS")
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(7)
}

/// Serve a derived view: ETag-conditional, `as_of`-stamped.
fn derived(payload: &Value, headers: &HeaderMap) -> AxumResponse {
    let tag = super::weak_etag(payload, "");
    if super::matches_etag(headers, &tag) {
        return axum::http::StatusCode::NOT_MODIFIED.into_response();
    }
    let mut response = axum::Json(payload.clone()).into_response();
    if let Ok(value) = axum::http::HeaderValue::from_str(&tag) {
        response
            .headers_mut()
            .insert(axum::http::header::ETAG, value);
    }
    response
}

/// `GET /api/sites/{pid}/insights/health`.
#[debug_handler]
#[allow(clippy::too_many_lines)] // one pass over the health rules
async fn health(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    headers: HeaderMap,
) -> Result<AxumResponse> {
    let site = records::find_site(&ctx.db, records::parse_pid(&pid)?).await?;
    let now = chrono::Utc::now();
    let stale_before = now - chrono::Duration::days(stale_content_days());
    let stuck_before = now - chrono::Duration::days(review_window_days());

    let mut findings: Vec<Finding> = Vec::new();
    let entry_rows = entries::Entity::find()
        .filter(entries::Column::SitePid.eq(site.pid))
        .filter(entries::Column::DeletedAt.is_null())
        .limit(2000)
        .all(&ctx.db)
        .await?;

    let mut published_variants = 0u64;
    for entry in &entry_rows {
        let content_type =
            super::entries::content_type_of(&ctx.db, site.pid, &entry.content_type_key).await;
        let variants = entry_variants::Entity::find()
            .filter(entry_variants::Column::EntryPid.eq(entry.pid))
            .filter(entry_variants::Column::DeletedAt.is_null())
            .all(&ctx.db)
            .await?;
        let source = variants
            .iter()
            .find(|variant| variant.locale == entry.source_locale)
            .cloned();

        for variant in &variants {
            let finding = |rule, detail: String| Finding {
                rule,
                subject: entry.key.clone(),
                locale: Some(variant.locale.clone()),
                detail,
                owner: entry
                    .owner_ref
                    .clone()
                    .or_else(|| variant.reviewer_ref.clone()),
            };

            if variant.status == "in_review" && variant.updated_at.to_utc() < stuck_before {
                findings.push(finding(
                    "stuck_in_review",
                    format!(
                        "in review since {} (window is {} days)",
                        variant.updated_at.to_rfc3339(),
                        review_window_days()
                    ),
                ));
            }
            if variant.status == "approved"
                && variant.published_revision_pid.is_none()
                && variant.scheduled_publish_at.is_none()
            {
                findings.push(finding(
                    "approved_not_published",
                    "approved, but neither published nor scheduled".to_string(),
                ));
            }

            let Some(revision_pid) = variant.published_revision_pid else {
                continue;
            };
            published_variants += 1;
            let revision = records::find_revision(&ctx.db, revision_pid).await?;

            if revision.created_at.to_utc() < stale_before {
                findings.push(finding(
                    "stale_content",
                    format!(
                        "published revision written {} (window is {} days)",
                        revision.created_at.to_rfc3339(),
                        stale_content_days()
                    ),
                ));
            }

            let seo_block = revision.seo.as_object().cloned().unwrap_or_default();
            let robots = seo_block.get("robots").and_then(Value::as_str);
            let missing_meta = ["meta_title", "meta_description"]
                .into_iter()
                .filter(|key| {
                    seo_block
                        .get(*key)
                        .and_then(Value::as_str)
                        .is_none_or(|text| text.trim().is_empty())
                })
                .collect::<Vec<_>>();
            if seo::is_indexable(robots) && !missing_meta.is_empty() {
                findings.push(finding(
                    "seo_metadata_missing",
                    format!("indexable, but missing {missing_meta:?}"),
                ));
            }

            // References: broken targets and images without alt text.
            let edges = content_references::Entity::find()
                .filter(content_references::Column::FromRevisionPid.eq(revision.pid))
                .limit(500)
                .all(&ctx.db)
                .await?;
            for edge in edges {
                if let Some(asset_pid) = edge.to_asset_pid {
                    match records::find_asset(&ctx.db, asset_pid).await {
                        Ok(asset) => {
                            if asset.kind == "image"
                                && asset
                                    .alt_text
                                    .as_ref()
                                    .is_none_or(|text| text.trim().is_empty())
                            {
                                findings.push(finding(
                                    "image_alt_text_missing",
                                    format!("{} has no alt text ({})", asset_pid, edge.field_key),
                                ));
                            }
                        }
                        Err(_) => findings.push(finding(
                            "broken_reference",
                            format!("asset {asset_pid} is missing ({})", edge.field_key),
                        )),
                    }
                }
                if let Some(entry_pid) = edge.to_entry_pid
                    && records::find_entry(&ctx.db, entry_pid).await.is_err()
                {
                    findings.push(finding(
                        "broken_reference",
                        format!("entry {entry_pid} is missing ({})", edge.field_key),
                    ));
                }
            }

            // Content written under an older declaration that today's
            // one would reject.
            if let Ok(content_type) = &content_type
                && revision.type_schema_version < content_type.schema_version
                && let Ok(specs) =
                    serde_json::from_value::<Vec<schema::FieldSpec>>(content_type.fields.clone())
            {
                let values = revision.fields.as_object().cloned().unwrap_or_default();
                // Both halves of "today's declaration would reject it".
                // `validate_values` only inspects fields that are
                // *present*, so on its own it misses the commonest
                // migration of all: a field that became required after
                // the content was written. That content cannot be
                // republished — the publish gate refuses it via
                // `missing_required` — so a health view that stayed
                // silent about it would be reassuring and wrong.
                let mut problems = schema::validate_values(&specs, &values);
                problems.extend(
                    schema::missing_required(&specs, &values)
                        .into_iter()
                        .map(|key| format!("fields.{key} is now required and is absent")),
                );
                if !problems.is_empty() {
                    findings.push(finding(
                        "needs_migration",
                        format!(
                            "written under schema v{} (now v{}): {}",
                            revision.type_schema_version,
                            content_type.schema_version,
                            problems.join("; ")
                        ),
                    ));
                }
            }

            // Stale translations.
            if variant.locale != entry.source_locale
                && let Some(source) = &source
            {
                let numbers = revisions::Entity::find()
                    .filter(revisions::Column::VariantPid.eq(source.pid))
                    .order_by_asc(revisions::Column::Number)
                    .limit(1000)
                    .all(&ctx.db)
                    .await?;
                let number_of = |pid: Option<Uuid>| {
                    pid.and_then(|pid| {
                        numbers
                            .iter()
                            .find(|row| row.pid == pid)
                            .map(|row| row.number)
                    })
                };
                let verdict = staleness::staleness(
                    number_of(variant.translation_of_revision_pid),
                    number_of(source.published_revision_pid),
                    &numbers.iter().map(|row| row.number).collect::<Vec<_>>(),
                );
                if verdict.stale {
                    findings.push(finding(
                        "stale_translation",
                        format!(
                            "{} source revision(s) behind: {:?}",
                            verdict.revisions_behind, verdict.newer_revision_numbers
                        ),
                    ));
                }
            }
        }
    }

    // Orphan assets — reported, never deleted.
    let asset_rows = assets::Entity::find()
        .filter(assets::Column::SitePid.eq(site.pid))
        .filter(assets::Column::DeletedAt.is_null())
        .limit(2000)
        .all(&ctx.db)
        .await?;
    let mut orphan_bytes = 0i64;
    for asset in &asset_rows {
        let referrers =
            usage::live_referrers(&ctx.db, content_references::Column::ToAssetPid, asset.pid)
                .await?;
        if referrers.is_empty() {
            orphan_bytes = orphan_bytes.saturating_add(asset.byte_size);
            findings.push(Finding {
                rule: "orphan_asset",
                subject: asset.pid.to_string(),
                locale: None,
                detail: format!(
                    "{} ({} bytes) referenced by nothing",
                    asset.title.clone().unwrap_or_else(|| asset.mime.clone()),
                    asset.byte_size
                ),
                owner: asset.uploaded_by_ref.clone(),
            });
        }
    }

    // Route hazards: chains near the cap, and noindex pages in menus.
    let redirect_rows = redirects::Entity::find()
        .filter(redirects::Column::SitePid.eq(site.pid))
        .limit(2000)
        .all(&ctx.db)
        .await?;
    let hop_cap = super::routing::max_hops();
    let mut by_locale: BTreeMap<String, Vec<crate::rules::path::Redirect>> = BTreeMap::new();
    for row in &redirect_rows {
        by_locale
            .entry(row.locale.clone())
            .or_default()
            .push(crate::rules::path::Redirect {
                from: row.from_path.clone(),
                to: row.to_path.clone(),
                status: u16::try_from(row.status).unwrap_or(301),
            });
    }
    for (locale, table) in &by_locale {
        for redirect in table {
            let followed = crate::rules::path::follow(&redirect.from, table, hop_cap);
            if followed.problem.is_some() || followed.hops.len() >= hop_cap.saturating_sub(1) {
                findings.push(Finding {
                    rule: "route_hazard",
                    subject: redirect.from.clone(),
                    locale: Some(locale.clone()),
                    detail: format!(
                        "{} hop(s) toward a cap of {hop_cap}{}",
                        followed.hops.len(),
                        followed
                            .problem
                            .map(|p| format!(" — {p}"))
                            .unwrap_or_default()
                    ),
                    owner: None,
                });
            }
        }
    }
    for menu in menus::Entity::find()
        .filter(menus::Column::SitePid.eq(site.pid))
        .filter(menus::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?
    {
        for item in menu.items.as_array().cloned().unwrap_or_default() {
            let Some(entry_pid) = item
                .get("entry_pid")
                .and_then(Value::as_str)
                .and_then(|pid| Uuid::parse_str(pid).ok())
            else {
                continue;
            };
            if records::find_entry(&ctx.db, entry_pid).await.is_err() {
                findings.push(Finding {
                    rule: "route_hazard",
                    subject: format!("menu {}", menu.key),
                    locale: Some(menu.locale.clone()),
                    detail: format!("links to entry {entry_pid}, which no longer exists"),
                    owner: None,
                });
            }
        }
    }

    let grouped: Vec<Value> = insight::group_by_rule(&findings)
        .into_iter()
        .map(|(rule, items)| {
            serde_json::json!({
                "rule": rule,
                "explanation": insight::HEALTH_RULES
                    .iter()
                    .find(|(key, _)| *key == rule)
                    .map(|(_, text)| *text),
                "count": items.len(),
                "findings": items,
            })
        })
        .collect();

    let payload = serde_json::json!({
        "as_of": now,
        "site": site.key,
        "entries": entry_rows.len(),
        "published_variants": published_variants,
        "findings_total": findings.len(),
        "by_rule": grouped,
        "orphan_bytes": orphan_bytes,
        // No severity score is invented: findings are grouped by rule,
        // and the count is the count.
        "note": "no severity is assigned; rules are listed with their counts, and nothing here is acted on automatically",
        "windows": {
            "stale_content_days": stale_content_days(),
            "review_days": review_window_days(),
        },
    });
    Ok(derived(&payload, &headers))
}

/// `GET /api/sites/{pid}/insights/throughput?days=30`.
#[derive(Debug, Deserialize)]
struct ThroughputParams {
    #[serde(default = "default_days")]
    days: i64,
}

const fn default_days() -> i64 {
    30
}

#[debug_handler]
#[allow(clippy::too_many_lines)] // one pass over the audit trail
async fn throughput(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    Query(params): Query<ThroughputParams>,
    headers: HeaderMap,
) -> Result<AxumResponse> {
    let site = records::find_site(&ctx.db, records::parse_pid(&pid)?).await?;
    let days = params.days.clamp(1, 3650);
    let now = chrono::Utc::now();
    let since: chrono::DateTime<chrono::FixedOffset> = (now - chrono::Duration::days(days)).into();

    // Which variants belong to this site? Audit rows are keyed by
    // record pid, so the site scope is applied here rather than in SQL.
    let entry_rows = entries::Entity::find()
        .filter(entries::Column::SitePid.eq(site.pid))
        .filter(entries::Column::DeletedAt.is_null())
        .limit(5000)
        .all(&ctx.db)
        .await?;
    let mut variant_owner: BTreeMap<Uuid, String> = BTreeMap::new();
    for entry in &entry_rows {
        for variant in entry_variants::Entity::find()
            .filter(entry_variants::Column::EntryPid.eq(entry.pid))
            .filter(entry_variants::Column::DeletedAt.is_null())
            .all(&ctx.db)
            .await?
        {
            variant_owner.insert(variant.pid, entry.key.clone());
        }
    }

    let rows = audit_logs::Entity::find()
        .filter(audit_logs::Column::Entity.eq("variant"))
        .filter(audit_logs::Column::CreatedAt.gte(since))
        .order_by_asc(audit_logs::Column::Id)
        .limit(20_000)
        .all(&ctx.db)
        .await?;

    let mut counts: BTreeMap<String, u64> = BTreeMap::new();
    let mut per_actor: BTreeMap<String, BTreeMap<String, u64>> = BTreeMap::new();
    // First occurrence of each transition per variant, for durations.
    let mut first_at: BTreeMap<(Uuid, String), chrono::DateTime<chrono::FixedOffset>> =
        BTreeMap::new();
    for row in &rows {
        if !variant_owner.contains_key(&row.entity_pid) {
            continue;
        }
        *counts.entry(row.action.clone()).or_default() += 1;
        let actor = row
            .actor
            .clone()
            .unwrap_or_else(|| "unattributed".to_string());
        *per_actor
            .entry(actor)
            .or_default()
            .entry(row.action.clone())
            .or_default() += 1;
        first_at
            .entry((row.entity_pid, row.action.clone()))
            .or_insert(row.created_at);
    }

    // Time in state, measured between recorded transitions.
    let mut draft_to_review = Vec::new();
    let mut review_to_approved = Vec::new();
    let mut approved_to_published = Vec::new();
    for variant_pid in variant_owner.keys() {
        let at = |action: &str| first_at.get(&(*variant_pid, action.to_string())).copied();
        if let (Some(submitted), Some(approved)) = (at("submit"), at("approve")) {
            review_to_approved.push((approved - submitted).num_seconds());
        }
        if let (Some(approved), Some(published)) = (at("approve"), at("publish")) {
            approved_to_published.push((published - approved).num_seconds());
        }
        if let (Some(created), Some(submitted)) = (at("created"), at("submit")) {
            draft_to_review.push((submitted - created).num_seconds());
        }
    }

    let published = *counts.get("publish").unwrap_or(&0);
    let submitted = *counts.get("submit").unwrap_or(&0);
    let rejected = *counts.get("reject").unwrap_or(&0);
    let approved = *counts.get("approve").unwrap_or(&0);

    let payload = serde_json::json!({
        "as_of": now,
        "site": site.key,
        "period_days": days,
        "activity": {
            "submitted": submitted,
            "approved": approved,
            "rejected": rejected,
            "published": published,
            "unpublished": counts.get("unpublish").copied().unwrap_or(0),
            "archived": counts.get("archive").copied().unwrap_or(0),
        },
        // Ratios show their working, and a zero denominator is `null`
        // rather than a flattering percentage.
        "rates": {
            "approval_rate": insight::ratio(approved, submitted),
            "rejection_rate": insight::ratio(rejected, submitted),
        },
        "time_in_state": {
            "draft_to_review": insight::summarise(draft_to_review),
            "review_to_approved": insight::summarise(review_to_approved),
            "approved_to_published": insight::summarise(approved_to_published),
            "measured_from": "recorded transition audit rows, not updated_at",
        },
        "per_actor": per_actor,
        "publishing_cadence_per_day": insight::ratio(published, u64::try_from(days).unwrap_or(1)),
    });
    Ok(derived(&payload, &headers))
}

/// `GET /api/sites/{pid}/insights/backlog` — what is waiting, and how
/// long it has waited.
#[debug_handler]
async fn backlog(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    headers: HeaderMap,
) -> Result<AxumResponse> {
    let site = records::find_site(&ctx.db, records::parse_pid(&pid)?).await?;
    let now = chrono::Utc::now();
    let entry_rows = entries::Entity::find()
        .filter(entries::Column::SitePid.eq(site.pid))
        .filter(entries::Column::DeletedAt.is_null())
        .limit(2000)
        .all(&ctx.db)
        .await?;

    let mut pending_review = Vec::new();
    let mut pending_schedule = Vec::new();
    let mut open_translations = Vec::new();
    for entry in &entry_rows {
        for variant in entry_variants::Entity::find()
            .filter(entry_variants::Column::EntryPid.eq(entry.pid))
            .filter(entry_variants::Column::DeletedAt.is_null())
            .all(&ctx.db)
            .await?
        {
            let age_days = (now - variant.updated_at.to_utc()).num_days();
            let row = serde_json::json!({
                "entry_key": entry.key,
                "locale": variant.locale,
                "age_days": age_days,
                "bucket": match age_days {
                    0..=1 => "today",
                    2..=7 => "this week",
                    8..=30 => "this month",
                    _ => "older",
                },
            });
            if variant.status == "in_review" {
                pending_review.push(row.clone());
            }
            if variant.scheduled_publish_at.is_some() || variant.scheduled_unpublish_at.is_some() {
                pending_schedule.push(serde_json::json!({
                    "entry_key": entry.key,
                    "locale": variant.locale,
                    "publish_at": variant.scheduled_publish_at,
                    "unpublish_at": variant.scheduled_unpublish_at,
                }));
            }
            if variant
                .translation_status
                .as_deref()
                .is_some_and(|status| status != "translated")
            {
                open_translations.push(row);
            }
        }
    }

    let payload = serde_json::json!({
        "as_of": now,
        "site": site.key,
        "pending_review": pending_review,
        "pending_schedule": pending_schedule,
        "open_translations": open_translations,
    });
    Ok(derived(&payload, &headers))
}

/// The insight routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/sites/{pid}/insights/health", get(health))
        .add("/sites/{pid}/insights/throughput", get(throughput))
        .add("/sites/{pid}/insights/backlog", get(backlog))
}
