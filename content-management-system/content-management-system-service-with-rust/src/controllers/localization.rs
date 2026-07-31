//! Localization (CMS-R13–R15): locale resolution, the translation
//! workflow, and derived staleness.
//!
//! ## Resolution says what it did
//!
//! `GET …/resolve/{locale}` answers "which locale would serve this
//! entry, and did it fall back?" — the same pure function the delivery
//! composer will call (CMS-T17). Every answer carries
//! `locale_requested`, `locale_served`, `fallback_applied`, and the
//! hops actually walked, because a CMS that serves English under a
//! `/fr/` URL without saying so is not localized, it is lying.
//!
//! ## Staleness is derived, and says how far behind
//!
//! A translation records the exact source revision it was made from.
//! When the source publishes newer revisions the translation is stale —
//! reported with the count *and the revision numbers*, so a translator
//! can read the diff instead of starting again. Nothing is stored, and
//! nothing is unpublished automatically unless the content type opts
//! in (`unpublish_on_stale`, off by default): stale-but-published
//! usually beats absent, and that judgement belongs to an editor.

use loco_rs::prelude::*;
use sea_orm::{QueryOrder, QuerySelect, TransactionTrait};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::{ensure_valid, unprocessable};
use crate::auth::MaybeAuthUser;
use crate::models::_entities::{entries, entry_variants, revisions, sites};
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
use crate::rules::lifecycle::translation;
use crate::rules::locale::{self, LocaleConfig};
use crate::rules::staleness::{self, Staleness};
use crate::streaming;
use crate::validation::Problems;

/// `POST …/translation` body.
#[derive(Debug, Deserialize)]
struct TranslationPayload {
    /// One of `request`, `claim`, `complete`, `cancel`.
    action: String,
    #[serde(default)]
    translator_ref: Option<String>,
    #[serde(default)]
    due_on: Option<chrono::NaiveDate>,
}

/// One locale's row in an entry's translation matrix.
#[derive(Debug, Serialize)]
struct LocaleRow {
    locale: String,
    is_source: bool,
    status: String,
    published: bool,
    translation_status: Option<String>,
    translator_ref: Option<String>,
    due_on: Option<chrono::NaiveDate>,
    staleness: Staleness,
}

/// A site's declared locale configuration, read back out of storage.
struct DeclaredLocales {
    /// Every locale the site publishes.
    locales: Vec<String>,
    /// Per-locale fallback chains.
    chains: Vec<(String, Vec<String>)>,
    /// Locales that refuse fallback.
    strict: Vec<String>,
}

/// Read a site's declared locale configuration back out of storage.
fn locale_config(site: &sites::Model) -> DeclaredLocales {
    let locales: Vec<String> = serde_json::from_value(site.locales.clone()).unwrap_or_default();
    let chains: std::collections::BTreeMap<String, Vec<String>> =
        serde_json::from_value(site.fallback_chains.clone()).unwrap_or_default();
    let strict: Vec<String> =
        serde_json::from_value(site.strict_locales.clone()).unwrap_or_default();
    DeclaredLocales {
        locales,
        chains: chains.into_iter().collect(),
        strict,
    }
}

/// Every live variant of an entry.
async fn variants_of<C: sea_orm::ConnectionTrait>(
    db: &C,
    entry_pid: Uuid,
) -> Result<Vec<entry_variants::Model>> {
    let rows = entry_variants::Entity::find()
        .filter(entry_variants::Column::EntryPid.eq(entry_pid))
        .filter(entry_variants::Column::DeletedAt.is_null())
        .order_by_asc(entry_variants::Column::Id)
        .all(db)
        .await?;
    Ok(rows)
}

/// The revision numbers on a variant, and the number of a specific
/// revision — the two inputs staleness needs.
async fn revision_numbers<C: sea_orm::ConnectionTrait>(
    db: &C,
    variant_pid: Uuid,
) -> Result<Vec<(Uuid, i32)>> {
    let rows = revisions::Entity::find()
        .filter(revisions::Column::VariantPid.eq(variant_pid))
        .order_by_asc(revisions::Column::Number)
        .limit(1000)
        .all(db)
        .await?;
    Ok(rows.into_iter().map(|row| (row.pid, row.number)).collect())
}

/// Compute a variant's staleness against its entry's source variant.
async fn staleness_of<C: sea_orm::ConnectionTrait>(
    db: &C,
    entry: &entries::Model,
    variant: &entry_variants::Model,
    source: Option<&entry_variants::Model>,
) -> Result<Staleness> {
    if variant.locale == entry.source_locale {
        return Ok(staleness::staleness(None, None, &[]));
    }
    let Some(source) = source else {
        return Ok(staleness::staleness(None, None, &[]));
    };
    let numbers = revision_numbers(db, source.pid).await?;
    let number_of = |pid: Option<Uuid>| {
        pid.and_then(|pid| {
            numbers
                .iter()
                .find(|(candidate, _)| *candidate == pid)
                .map(|(_, number)| *number)
        })
    };
    let all: Vec<i32> = numbers.iter().map(|(_, number)| *number).collect();
    Ok(staleness::staleness(
        number_of(variant.translation_of_revision_pid),
        number_of(source.published_revision_pid),
        &all,
    ))
}

/// `GET /api/entries/{pid}/resolve/{locale}` — which locale would
/// serve, and whether that is a fallback.
#[debug_handler]
async fn resolve(
    State(ctx): State<AppContext>,
    Path((pid, requested)): Path<(String, String)>,
) -> Result<Response> {
    let entry = records::find_entry(&ctx.db, records::parse_pid(&pid)?).await?;
    let site = records::find_site(&ctx.db, entry.site_pid).await?;
    let declared = locale_config(&site);
    let variants = variants_of(&ctx.db, entry.pid).await?;
    let published: Vec<String> = variants
        .iter()
        .filter(|variant| variant.published_revision_pid.is_some())
        .map(|variant| variant.locale.clone())
        .collect();

    let resolution = locale::resolve(
        &LocaleConfig {
            default_locale: &site.default_locale,
            locales: &declared.locales,
            fallback_chains: &declared.chains,
            strict_locales: &declared.strict,
        },
        &requested,
        &published,
    );
    let served_revision = resolution.locale_served.as_ref().and_then(|locale| {
        variants
            .iter()
            .find(|variant| &variant.locale == locale)
            .and_then(|variant| variant.published_revision_pid)
    });
    format::json(serde_json::json!({
        "entry_pid": entry.pid,
        "entry_key": entry.key,
        "resolution": resolution,
        "published_revision_pid": served_revision,
        "published_locales": published,
    }))
}

/// `GET /api/entries/{pid}/translations` — the locale matrix, with
/// staleness per locale.
#[debug_handler]
async fn matrix(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let entry = records::find_entry(&ctx.db, records::parse_pid(&pid)?).await?;
    let site = records::find_site(&ctx.db, entry.site_pid).await?;
    let declared = locale_config(&site);
    let variants = variants_of(&ctx.db, entry.pid).await?;
    let source = variants
        .iter()
        .find(|variant| variant.locale == entry.source_locale)
        .cloned();

    let mut rows = Vec::new();
    for variant in &variants {
        rows.push(LocaleRow {
            locale: variant.locale.clone(),
            is_source: variant.locale == entry.source_locale,
            status: variant.status.clone(),
            published: variant.published_revision_pid.is_some(),
            translation_status: variant.translation_status.clone(),
            translator_ref: variant.translator_ref.clone(),
            due_on: variant.translation_due_on,
            staleness: staleness_of(&ctx.db, &entry, variant, source.as_ref()).await?,
        });
    }
    // The gap list is as useful as the matrix: a locale the site
    // declares but this entry has never been started in.
    let missing: Vec<&String> = declared
        .locales
        .iter()
        .filter(|locale| !variants.iter().any(|variant| &&variant.locale == locale))
        .collect();
    format::json(serde_json::json!({
        "entry_pid": entry.pid,
        "entry_key": entry.key,
        "source_locale": entry.source_locale,
        "locales": rows,
        "missing_locales": missing,
    }))
}

/// `POST /api/entries/{pid}/variants/{locale}/translation` — drive the
/// translation workflow.
///
/// `request` records **which source revision** is being translated, so
/// staleness is computable from the moment the work starts rather than
/// only after it finishes. `complete` flips the status; the translated
/// text itself arrives through the ordinary save endpoint, so there is
/// one write path for revisions and not two that can drift.
#[debug_handler]
#[allow(clippy::too_many_lines)] // one workflow action, applied end to end
async fn translate(
    State(ctx): State<AppContext>,
    Path((pid, locale)): Path<(String, String)>,
    caller: MaybeAuthUser,
    Json(payload): Json<TranslationPayload>,
) -> Result<Response> {
    let entry = records::find_entry(&ctx.db, records::parse_pid(&pid)?).await?;
    let Some(action) = translation::Action::parse(&payload.action) else {
        return Err(unprocessable(&format!(
            "unknown translation action {:?}; expected request, claim, complete, or cancel",
            payload.action
        )));
    };
    let mut problems = Problems::new();
    problems.ref_opt(
        "translator_ref",
        entity_ref::EntityType::Worker,
        payload.translator_ref.as_deref(),
    );
    ensure_valid(&problems.into_vec())?;
    if locale == entry.source_locale {
        return Err(unprocessable(&format!(
            "{locale:?} is this entry's source locale; there is nothing to translate it from"
        )));
    }

    let txn = ctx.db.begin().await?;
    let variant = entry_variants::Entity::find()
        .filter(entry_variants::Column::EntryPid.eq(entry.pid))
        .filter(entry_variants::Column::Locale.eq(locale.clone()))
        .filter(entry_variants::Column::DeletedAt.is_null())
        .lock_exclusive()
        .one(&txn)
        .await?
        .ok_or(Error::NotFound)?;

    let from = variant.translation_status.clone();
    let to = match translation::next(from.as_deref(), action) {
        Ok(to) => to,
        Err(message) => {
            txn.rollback().await?;
            return Err(unprocessable(&message));
        }
    };

    // On request, pin the source revision this translation is *of*.
    // Doing it here rather than at completion is what lets the matrix
    // show staleness for work that is still in progress.
    let mut source_revision = variant.translation_of_revision_pid;
    if action == translation::Action::Request {
        let source = variants_of(&txn, entry.pid)
            .await?
            .into_iter()
            .find(|candidate| candidate.locale == entry.source_locale);
        let Some(source) = source else {
            txn.rollback().await?;
            return Err(unprocessable(&format!(
                "this entry has no {} variant to translate from",
                entry.source_locale
            )));
        };
        // Prefer what is live; fall back to what is written. Translating
        // a draft is legitimate — translating *nothing* is not.
        source_revision = source
            .published_revision_pid
            .or(source.current_revision_pid);
        if source_revision.is_none() {
            txn.rollback().await?;
            return Err(unprocessable(
                "the source variant has no revision to translate from",
            ));
        }
    }

    let variant_pid = variant.pid;
    let mut active: entry_variants::ActiveModel = variant.into();
    active.translation_status = ActiveValue::set(to.map(ToString::to_string));
    active.translation_of_revision_pid = ActiveValue::set(source_revision);
    match action {
        translation::Action::Request => {
            active.translation_requested_at = ActiveValue::set(Some(chrono::Utc::now().into()));
            active.translation_requested_by =
                ActiveValue::set(caller.actor().map(ToString::to_string));
            active.translation_due_on = ActiveValue::set(payload.due_on);
            active.translator_ref = ActiveValue::set(payload.translator_ref.clone());
        }
        translation::Action::Claim => {
            active.translator_ref = ActiveValue::set(
                payload
                    .translator_ref
                    .clone()
                    .or_else(|| caller.actor().map(ToString::to_string)),
            );
        }
        translation::Action::Cancel => {
            active.translation_requested_at = ActiveValue::set(None);
            active.translation_requested_by = ActiveValue::set(None);
            active.translation_due_on = ActiveValue::set(None);
            active.translator_ref = ActiveValue::set(None);
            active.translation_of_revision_pid = ActiveValue::set(None);
        }
        translation::Action::Complete => {}
    }
    let updated = active.update(&txn).await?;

    Audit::record(
        &txn,
        "variant",
        variant_pid,
        &format!("translation_{}", action.as_str()),
        caller.actor(),
        Some(serde_json::json!({
            "entry": entry.key,
            "locale": locale,
            "from": from,
            "to": to,
            "source_revision_pid": source_revision,
            "translator": updated.translator_ref,
            "due_on": updated.translation_due_on,
        })),
    )
    .await?;
    let kind = match action {
        translation::Action::Request => Some("translation_requested"),
        translation::Action::Complete => Some("translation_completed"),
        _ => None,
    };
    if let Some(kind) = kind {
        streaming::emit_on(
            &txn,
            "variant",
            kind,
            &variant_pid.to_string(),
            &entry.key,
            caller.actor(),
            Some(serde_json::json!({ "locale": locale })),
        )
        .await?;
    }
    txn.commit().await?;

    format::json(serde_json::json!({
        "variant_pid": variant_pid,
        "locale": locale,
        "translation_status": updated.translation_status,
        "translation_of_revision_pid": updated.translation_of_revision_pid,
        "translator_ref": updated.translator_ref,
        "due_on": updated.translation_due_on,
    }))
}

/// `GET /api/sites/{pid}/translations` — the translator's queue and the
/// stale list, in one read.
///
/// Stale translations are **reported**, not unpublished — unless the
/// content type opted into `unpublish_on_stale`, in which case the
/// entry is listed under `would_unpublish` so the decision is visible
/// before anything acts on it.
#[debug_handler]
async fn site_translations(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
) -> Result<Response> {
    let site = records::find_site(&ctx.db, records::parse_pid(&pid)?).await?;
    let entry_rows = entries::Entity::find()
        .filter(entries::Column::SitePid.eq(site.pid))
        .filter(entries::Column::DeletedAt.is_null())
        .limit(1000)
        .all(&ctx.db)
        .await?;

    let mut queue = Vec::new();
    let mut stale = Vec::new();
    let mut would_unpublish = Vec::new();
    for entry in entry_rows {
        let variants = variants_of(&ctx.db, entry.pid).await?;
        let source = variants
            .iter()
            .find(|variant| variant.locale == entry.source_locale)
            .cloned();
        let content_type =
            super::entries::content_type_of(&ctx.db, site.pid, &entry.content_type_key).await;
        let unpublish_on_stale = content_type.is_ok_and(|t| t.unpublish_on_stale);
        for variant in &variants {
            if let Some(status) = &variant.translation_status
                && status != "translated"
            {
                queue.push(serde_json::json!({
                    "entry_key": entry.key,
                    "entry_pid": entry.pid,
                    "locale": variant.locale,
                    "translation_status": status,
                    "translator_ref": variant.translator_ref,
                    "due_on": variant.translation_due_on,
                    "requested_at": variant.translation_requested_at,
                }));
            }
            let verdict = staleness_of(&ctx.db, &entry, variant, source.as_ref()).await?;
            if verdict.stale {
                let row = serde_json::json!({
                    "entry_key": entry.key,
                    "entry_pid": entry.pid,
                    "locale": variant.locale,
                    "published": variant.published_revision_pid.is_some(),
                    "staleness": verdict,
                });
                if unpublish_on_stale && variant.published_revision_pid.is_some() {
                    would_unpublish.push(row.clone());
                }
                stale.push(row);
            }
        }
    }
    format::json(serde_json::json!({
        "as_of": chrono::Utc::now(),
        "rule": "stale ⇔ the source's published revision is newer than the one this was translated from",
        "queue": queue,
        "stale": stale,
        "would_unpublish": would_unpublish,
        "auto_unpublished": false,
    }))
}

/// `GET /api/sites/{pid}/locale-coverage` — which locales each content
/// type actually reaches.
#[debug_handler]
async fn coverage(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let site = records::find_site(&ctx.db, records::parse_pid(&pid)?).await?;
    let declared = locale_config(&site);
    let entry_rows = entries::Entity::find()
        .filter(entries::Column::SitePid.eq(site.pid))
        .filter(entries::Column::DeletedAt.is_null())
        .limit(1000)
        .all(&ctx.db)
        .await?;
    let total = entry_rows.len();

    let mut per_locale: Vec<Value> = Vec::new();
    for locale in &declared.locales {
        let mut published = 0usize;
        let mut started = 0usize;
        let mut gaps: Vec<&str> = Vec::new();
        for entry in &entry_rows {
            let variants = variants_of(&ctx.db, entry.pid).await?;
            match variants.iter().find(|variant| &variant.locale == locale) {
                Some(variant) => {
                    started += 1;
                    if variant.published_revision_pid.is_some() {
                        published += 1;
                    }
                }
                None => gaps.push(entry.key.as_str()),
            }
        }
        per_locale.push(serde_json::json!({
            "locale": locale,
            "entries_total": total,
            "entries_started": started,
            "entries_published": published,
            // The gap list, not just the count: a percentage tells an
            // editor how bad it is, a list tells them what to do.
            "missing_entry_keys": gaps,
        }));
    }
    format::json(serde_json::json!({
        "as_of": chrono::Utc::now(),
        "default_locale": site.default_locale,
        "coverage": per_locale,
    }))
}

/// The localization routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/entries/{pid}/resolve/{locale}", get(resolve))
        .add("/entries/{pid}/translations", get(matrix))
        .add(
            "/entries/{pid}/variants/{locale}/translation",
            post(translate),
        )
        .add("/sites/{pid}/translations", get(site_translations))
        .add("/sites/{pid}/locale-coverage", get(coverage))
}
