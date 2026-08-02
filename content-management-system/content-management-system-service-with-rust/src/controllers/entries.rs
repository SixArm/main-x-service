//! Entries, per-locale variants, and the append-only revision chain
//! (CMS-R3–R5).
//!
//! Three behaviours here are the ones worth reading carefully, because
//! they are the ones a CMS usually gets wrong:
//!
//! - **A save states what it edited.** Every revision carries
//!   `base_revision_pid`; if the variant has moved on, the save is
//!   refused `409` **with the competing revision**, rather than
//!   last-write-wins quietly destroying a colleague's paragraph.
//! - **History is append-only.** Nothing updates or deletes a revision.
//!   Restore writes a *new* revision that copies an old body and records
//!   `restored_from_pid`, so "we went back" is itself in the history
//!   (CMS-D3).
//! - **References are extracted on save**, in the same transaction, so
//!   "where used" can never disagree with the content, and a delete that
//!   would break a live reference is refused rather than discovered by a
//!   reader (CMS-D8).
//!
//! Revision numbers are allocated under `SELECT … FOR UPDATE` on the
//! variant row, and `UNIQUE (variant_pid, number)` backs it up: two
//! concurrent saves cannot both take number 4 (CMS-D15).

use loco_rs::prelude::*;
use sea_orm::{DatabaseTransaction, PaginatorTrait, QueryOrder, QuerySelect, TransactionTrait};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

use super::{authz_error, conflict, ensure_valid, unprocessable};
use crate::auth::{self, Action, MaybeAuthUser};
use crate::models::_entities::{
    content_references, content_types, entries, entry_variants, revisions, sites,
};
use crate::models::audit_logs::Model as Audit;
use crate::models::{records, usage};
use crate::rules::{block, diff, gate, locale as locale_rules, reference, schema, tokens};
use crate::streaming;
use crate::validation::Problems;

/// The body an author submits for a revision.
#[derive(Debug, Deserialize, Default)]
struct Body {
    #[serde(default)]
    title: String,
    #[serde(default)]
    blocks: Vec<Value>,
    #[serde(default)]
    fields: Map<String, Value>,
    #[serde(default)]
    seo: Map<String, Value>,
    #[serde(default)]
    note: Option<String>,
}

/// `POST /api/sites/{pid}/entries` body.
#[derive(Debug, Deserialize)]
struct CreateEntryPayload {
    key: String,
    content_type_key: String,
    #[serde(default)]
    locale: Option<String>,
    #[serde(default)]
    owner_ref: Option<String>,
    #[serde(flatten)]
    body: Body,
}

/// `POST /api/entries/{pid}/variants` body.
#[derive(Debug, Deserialize)]
struct CreateVariantPayload {
    locale: String,
    #[serde(default)]
    body: Option<Body>,
}

/// `POST …/revisions` body — a save, stating the revision it edited.
#[derive(Debug, Deserialize)]
struct SavePayload {
    /// The revision this edit was made from. A stale value is `409`.
    base_revision_pid: Uuid,
    #[serde(flatten)]
    body: Body,
}

/// `POST …/restore` body.
#[derive(Debug, Deserialize)]
struct RestorePayload {
    revision_pid: Uuid,
    #[serde(default)]
    note: Option<String>,
}

/// A `{pid}` reference response.
#[derive(Debug, Serialize)]
struct PidRef {
    pid: String,
}

/// What a save produced.
#[derive(Debug, Serialize)]
struct SavedView {
    revision_pid: String,
    number: i32,
    /// How many blocks the sanitizer altered. Reported rather than
    /// silent, so a caller is never told their markup was stored
    /// verbatim when it was not (CMS-D5).
    blocks_sanitized: usize,
    /// The references extracted from this revision.
    references: usize,
}

/// Validate the SEO block. The full derivation (sitemaps, canonical
/// resolution, `hreflang`) is CMS-T18; what is enforced now is that
/// stored values are well-formed, so the later derivation has nothing
/// to clean up.
fn validate_seo(seo: &Map<String, Value>) -> Vec<String> {
    const PERMITTED: &[&str] = &[
        "meta_title",
        "meta_description",
        "canonical_url",
        "robots",
        "og_title",
        "og_description",
        "og_image",
        "sitemap_priority",
        "sitemap_changefreq",
    ];
    let mut problems = Problems::new();
    for key in seo.keys() {
        if !PERMITTED.contains(&key.as_str()) {
            problems.push(format!(
                "seo.{key} is not an SEO key (permitted: {PERMITTED:?})"
            ));
        }
    }
    let text = |key: &str| seo.get(key).and_then(Value::as_str);
    problems.cap_opt("seo.meta_title", text("meta_title"));
    problems.cap_opt("seo.meta_description", text("meta_description"));
    problems.url_opt("seo.canonical_url", text("canonical_url"));
    if let Some(robots) = text("robots") {
        problems.require_token("seo.robots", tokens::ROBOTS, robots);
    }
    if let Some(image) = text("og_image")
        && Uuid::parse_str(image).is_err()
    {
        problems.push("seo.og_image must be an asset uuid".to_string());
    }
    problems.into_vec()
}

/// The declared field specs of a content type, or a `422` naming the
/// type whose stored schema no longer reads.
fn specs_of(content_type: &content_types::Model) -> Result<Vec<schema::FieldSpec>> {
    serde_json::from_value(content_type.fields.clone()).map_err(|e| {
        unprocessable(&format!(
            "stored field schema for {:?} is unreadable: {e}",
            content_type.key
        ))
    })
}

/// Find the live content type `key` on `site_pid`, else `422` — an
/// entry cannot exist without the type that gives its fields meaning.
pub(crate) async fn content_type_of<C: sea_orm::ConnectionTrait>(
    db: &C,
    site_pid: Uuid,
    key: &str,
) -> Result<content_types::Model> {
    content_types::Entity::find()
        .filter(content_types::Column::SitePid.eq(site_pid))
        .filter(content_types::Column::Key.eq(key))
        .filter(content_types::Column::DeletedAt.is_null())
        .one(db)
        .await?
        .ok_or_else(|| {
            unprocessable(&format!(
                "content_type_key {key:?} is not a type on this site"
            ))
        })
}

/// Validate + sanitize a body against its content type. Returns the
/// sanitized blocks, the extracted references, and how many blocks the
/// sanitizer altered.
fn prepare_body(
    body: &mut Body,
    specs: &[schema::FieldSpec],
) -> Result<(usize, Vec<reference::Reference>)> {
    let mut problems = Problems::new();
    problems.require_text("title", &body.title);
    problems.cap_opt("note", body.note.as_deref());
    let mut problems = problems.into_vec();

    // Sanitize *before* validating, so what is validated is what is
    // stored, and what is stored was never trusted markup.
    let sanitized = block::sanitize_document(&mut body.blocks);
    problems.extend(block::validate_document(&body.blocks));
    problems.extend(schema::validate_values(specs, &body.fields));
    problems.extend(validate_seo(&body.seo));
    ensure_valid(&problems)?;

    let references = reference::extract(&body.blocks, &body.fields, specs);
    Ok((sanitized, references))
}

/// Everything one revision write needs, so the insert takes a value
/// rather than a queue of positional arguments that are easy to
/// transpose.
struct NewRevision<'a> {
    variant_pid: Uuid,
    number: i32,
    schema_version: i32,
    body: &'a Body,
    author: Option<&'a str>,
    restored_from: Option<Uuid>,
    references: &'a [reference::Reference],
}

/// Insert a revision and its extracted references on `txn`, returning
/// the new row. The caller holds the variant lock, so `number` is safe
/// to allocate here.
async fn insert_revision(
    txn: &DatabaseTransaction,
    new: NewRevision<'_>,
) -> Result<revisions::Model> {
    let NewRevision {
        variant_pid,
        number,
        schema_version,
        body,
        author,
        restored_from,
        references,
    } = new;
    let revision = revisions::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        variant_pid: ActiveValue::set(variant_pid),
        number: ActiveValue::set(number),
        title: ActiveValue::set(body.title.clone()),
        blocks: ActiveValue::set(Value::Array(body.blocks.clone())),
        fields: ActiveValue::set(Value::Object(body.fields.clone())),
        seo: ActiveValue::set(Value::Object(body.seo.clone())),
        type_schema_version: ActiveValue::set(schema_version),
        author_ref: ActiveValue::set(author.map(ToString::to_string)),
        note: ActiveValue::set(body.note.clone()),
        restored_from_pid: ActiveValue::set(restored_from),
        ..Default::default()
    }
    .insert(txn)
    .await?;

    for extracted in references {
        let (to_entry, to_asset, to_entity) = match &extracted.target {
            reference::Target::Entry(id) => (Some(*id), None, None),
            reference::Target::Asset(id) => (None, Some(*id), None),
            reference::Target::Entity(urn) => (None, None, Some(urn.clone())),
        };
        content_references::ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            from_revision_pid: ActiveValue::set(revision.pid),
            from_variant_pid: ActiveValue::set(variant_pid),
            kind: ActiveValue::set(extracted.target.kind().to_string()),
            to_entry_pid: ActiveValue::set(to_entry),
            to_asset_pid: ActiveValue::set(to_asset),
            to_entity_ref: ActiveValue::set(to_entity),
            field_key: ActiveValue::set(extracted.field_key.clone()),
            ..Default::default()
        }
        .insert(txn)
        .await?;
    }
    Ok(revision)
}

/// The live variant row, locked for update — the critical section for
/// allocating a revision number and checking the edit base.
async fn lock_variant(
    txn: &DatabaseTransaction,
    entry_pid: Uuid,
    locale: &str,
) -> Result<entry_variants::Model> {
    entry_variants::Entity::find()
        .filter(entry_variants::Column::EntryPid.eq(entry_pid))
        .filter(entry_variants::Column::Locale.eq(locale))
        .filter(entry_variants::Column::DeletedAt.is_null())
        .lock_exclusive()
        .one(txn)
        .await?
        .ok_or(Error::NotFound)
}

/// The next revision number for a locked variant.
async fn next_number(txn: &DatabaseTransaction, variant_pid: Uuid) -> Result<i32> {
    let highest = revisions::Entity::find()
        .filter(revisions::Column::VariantPid.eq(variant_pid))
        .order_by_desc(revisions::Column::Number)
        .one(txn)
        .await?;
    Ok(highest.map_or(1, |row| row.number.saturating_add(1)))
}

/// `POST /api/sites/{pid}/entries` — create an entry, its source-locale
/// variant, and revision 1, in one transaction.
#[debug_handler]
#[allow(clippy::too_many_lines)] // one transaction, read top to bottom
async fn create_entry(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    caller: MaybeAuthUser,
    Json(mut payload): Json<CreateEntryPayload>,
) -> Result<Response> {
    let site = records::find_site(&ctx.db, records::parse_pid(&pid)?).await?;
    let locale = payload
        .locale
        .clone()
        .unwrap_or_else(|| site.default_locale.clone());

    let mut problems = Problems::new();
    problems.require_key("key", &payload.key);
    problems.ref_opt(
        "owner_ref",
        entity_ref::EntityType::Worker,
        payload.owner_ref.as_deref(),
    );
    let mut problems = problems.into_vec();
    problems.extend(check_locale(&site, &locale));
    ensure_valid(&problems)?;

    let content_type = content_type_of(&ctx.db, site.pid, &payload.content_type_key).await?;
    let specs = specs_of(&content_type)?;
    let (sanitized, references) = prepare_body(&mut payload.body, &specs)?;

    if entries::Entity::find()
        .filter(entries::Column::SitePid.eq(site.pid))
        .filter(entries::Column::Key.eq(payload.key.clone()))
        .filter(entries::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await?
        .is_some()
    {
        return Err(conflict(&format!(
            "entry key {:?} is already in use on site {}",
            payload.key, site.key
        )));
    }

    let txn = ctx.db.begin().await?;
    let entry = entries::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        site_pid: ActiveValue::set(site.pid),
        content_type_key: ActiveValue::set(content_type.key.clone()),
        type_schema_version: ActiveValue::set(content_type.schema_version),
        key: ActiveValue::set(payload.key.clone()),
        source_locale: ActiveValue::set(locale.clone()),
        owner_ref: ActiveValue::set(payload.owner_ref.clone()),
        archived_at: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    let variant = entry_variants::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        entry_pid: ActiveValue::set(entry.pid),
        locale: ActiveValue::set(locale.clone()),
        status: ActiveValue::set("draft".to_string()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    let revision = insert_revision(
        &txn,
        NewRevision {
            variant_pid: variant.pid,
            number: 1,
            schema_version: content_type.schema_version,
            body: &payload.body,
            author: caller.actor(),
            restored_from: None,
            references: &references,
        },
    )
    .await?;
    let mut active: entry_variants::ActiveModel = variant.into();
    active.current_revision_pid = ActiveValue::set(Some(revision.pid));
    let variant = active.update(&txn).await?;

    Audit::record(
        &txn,
        "entry",
        entry.pid,
        "created",
        caller.actor(),
        Some(serde_json::json!({
            "site": site.key,
            "key": entry.key,
            "content_type": entry.content_type_key,
            "locale": locale,
            "owner": entry.owner_ref,
            "references": references.len(),
        })),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "entry",
        "entry_created",
        &entry.pid.to_string(),
        &payload.body.title,
        caller.actor(),
        Some(serde_json::json!({ "locale": locale, "revision": 1 })),
    )
    .await?;
    txn.commit().await?;
    record_save_metrics(sanitized);

    format::json(serde_json::json!({
        "pid": entry.pid.to_string(),
        "variant_pid": variant.pid.to_string(),
        "revision_pid": revision.pid.to_string(),
        "number": 1,
        "blocks_sanitized": sanitized,
        "references": references.len(),
    }))
}

/// Whether `locale` is one the site declares.
fn check_locale(site: &sites::Model, locale: &str) -> Vec<String> {
    if !locale_rules::is_locale_code(locale) {
        return vec![format!(
            "locale {locale:?} is not a locale code (expected `xx` or `xx-YY`)"
        )];
    }
    let declared: Vec<String> = serde_json::from_value(site.locales.clone()).unwrap_or_default();
    if declared.iter().any(|l| l == locale) {
        Vec::new()
    } else {
        vec![format!(
            "locale {locale:?} is not declared by site {} (declared: {declared:?})",
            site.key
        )]
    }
}

/// Count a save in the metrics.
fn record_save_metrics(sanitized: usize) {
    let metrics = crate::metrics::Metrics::global();
    metrics.revision_created_total.inc();
    if sanitized > 0 {
        metrics
            .blocks_sanitized_total
            .inc_by(f64::from(u32::try_from(sanitized).unwrap_or(u32::MAX)));
    }
}

/// `GET /api/sites/{pid}/entries` — the site's live entries, filterable
/// by content type.
#[derive(Debug, Deserialize)]
struct ListParams {
    #[serde(default)]
    content_type: Option<String>,
}

#[debug_handler]
async fn list_entries(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    Query(params): Query<ListParams>,
) -> Result<Response> {
    let site = records::find_site(&ctx.db, records::parse_pid(&pid)?).await?;
    let mut query = entries::Entity::find()
        .filter(entries::Column::SitePid.eq(site.pid))
        .filter(entries::Column::DeletedAt.is_null());
    if let Some(content_type) = params.content_type {
        query = query.filter(entries::Column::ContentTypeKey.eq(content_type));
    }
    let rows = query
        .order_by_asc(entries::Column::Id)
        .limit(1000)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

/// `GET /api/entries/{pid}` — the entry with every locale variant, so
/// the locale matrix is one read.
#[debug_handler]
async fn get_entry(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let entry = records::find_entry(&ctx.db, records::parse_pid(&pid)?).await?;
    let variants = entry_variants::Entity::find()
        .filter(entry_variants::Column::EntryPid.eq(entry.pid))
        .filter(entry_variants::Column::DeletedAt.is_null())
        .order_by_asc(entry_variants::Column::Id)
        .all(&ctx.db)
        .await?;
    // The record-level pass, per variant: a policy may allow reading
    // one locale and refuse another, and it may allow a *masked* read.
    // Refused locales are omitted rather than reported, because naming
    // them would leak the shape of what the caller may not see.
    let mut visible = Vec::new();
    for variant in &variants {
        let attrs = auth::variant_resource_attrs(&entry, variant);
        if let Ok(obligations) = auth::authorize_record(&caller, Action::Read, &attrs) {
            visible.push(auth::mask_if_required(
                serde_json::to_value(variant)?,
                &obligations,
            ));
        }
    }
    format::json(serde_json::json!({ "entry": entry, "variants": visible }))
}

/// `POST /api/entries/{pid}/variants` — start this entry in another
/// locale.
#[debug_handler]
async fn create_variant(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    caller: MaybeAuthUser,
    Json(payload): Json<CreateVariantPayload>,
) -> Result<Response> {
    let entry = records::find_entry(&ctx.db, records::parse_pid(&pid)?).await?;
    let site = records::find_site(&ctx.db, entry.site_pid).await?;
    // The record-level pass on a write, **before** validation: the
    // variant does not exist yet, so the attributes describe the one
    // being asked for — which is what a locale-scoped persona (a
    // translator) is actually gated on, and the blanket guard cannot
    // decide it because it runs before any locale is known.
    //
    // Ordered ahead of `check_locale` deliberately: a caller who may
    // not write this locale should not learn from the error whether
    // the site declares it.
    auth::authorize_record(
        &caller,
        Action::Write,
        &auth::proposed_variant_attrs(&entry, &payload.locale),
    )
    .map_err(authz_error)?;
    ensure_valid(&check_locale(&site, &payload.locale))?;
    if records::find_variant_by_locale(&ctx.db, entry.pid, &payload.locale)
        .await
        .is_ok()
    {
        return Err(conflict(&format!(
            "entry {} already has a {} variant",
            entry.key, payload.locale
        )));
    }
    let content_type = content_type_of(&ctx.db, site.pid, &entry.content_type_key).await?;
    let specs = specs_of(&content_type)?;
    let mut body = payload.body.unwrap_or_default();
    if body.title.trim().is_empty() {
        body.title = format!("{} ({})", entry.key, payload.locale);
    }
    let (sanitized, references) = prepare_body(&mut body, &specs)?;

    let txn = ctx.db.begin().await?;
    let variant = entry_variants::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        entry_pid: ActiveValue::set(entry.pid),
        locale: ActiveValue::set(payload.locale.clone()),
        status: ActiveValue::set("draft".to_string()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    let revision = insert_revision(
        &txn,
        NewRevision {
            variant_pid: variant.pid,
            number: 1,
            schema_version: content_type.schema_version,
            body: &body,
            author: caller.actor(),
            restored_from: None,
            references: &references,
        },
    )
    .await?;
    let variant_pid = variant.pid;
    let mut active: entry_variants::ActiveModel = variant.into();
    active.current_revision_pid = ActiveValue::set(Some(revision.pid));
    active.update(&txn).await?;
    Audit::record(
        &txn,
        "variant",
        variant_pid,
        "created",
        caller.actor(),
        Some(serde_json::json!({ "entry": entry.key, "locale": payload.locale })),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "variant",
        "created",
        &variant_pid.to_string(),
        &body.title,
        caller.actor(),
        Some(serde_json::json!({ "locale": payload.locale })),
    )
    .await?;
    txn.commit().await?;
    record_save_metrics(sanitized);
    format::json(PidRef {
        pid: variant_pid.to_string(),
    })
}

/// `GET /api/entries/{pid}/variants/{locale}` — the variant with its
/// current revision in full, plus which revision is published (they are
/// different questions).
#[debug_handler]
async fn get_variant(
    State(ctx): State<AppContext>,
    Path((pid, locale)): Path<(String, String)>,
) -> Result<Response> {
    let entry = records::find_entry(&ctx.db, records::parse_pid(&pid)?).await?;
    let variant = records::find_variant_by_locale(&ctx.db, entry.pid, &locale).await?;
    let current = match variant.current_revision_pid {
        Some(pid) => Some(records::find_revision(&ctx.db, pid).await?),
        None => None,
    };
    format::json(serde_json::json!({
        "entry": entry,
        "variant": variant,
        "current_revision": current,
    }))
}

/// `POST /api/entries/{pid}/variants/{locale}/revisions` — save.
#[debug_handler]
async fn save_revision(
    State(ctx): State<AppContext>,
    Path((pid, locale)): Path<(String, String)>,
    caller: MaybeAuthUser,
    Json(mut payload): Json<SavePayload>,
) -> Result<Response> {
    let entry = records::find_entry(&ctx.db, records::parse_pid(&pid)?).await?;
    // The record-level pass, before any work: a policy that scopes a
    // persona to its own drafts or its own locales is decided here,
    // where the record is known, not by the coarse blanket guard.
    let existing = records::find_variant_by_locale(&ctx.db, entry.pid, &locale).await?;
    auth::authorize_record(
        &caller,
        Action::Write,
        &auth::variant_resource_attrs(&entry, &existing),
    )
    .map_err(authz_error)?;
    let content_type = content_type_of(&ctx.db, entry.site_pid, &entry.content_type_key).await?;
    let specs = specs_of(&content_type)?;
    let (sanitized, references) = prepare_body(&mut payload.body, &specs)?;

    let txn = ctx.db.begin().await?;
    let variant = lock_variant(&txn, entry.pid, &locale).await?;

    // The optimistic-concurrency check, inside the lock: if the variant
    // has advanced past what this edit was made from, refuse and hand
    // back the competing revision so a client can show a real conflict
    // rather than silently overwriting it.
    if variant.current_revision_pid != Some(payload.base_revision_pid) {
        let current = variant.current_revision_pid;
        txn.rollback().await?;
        let competing = match current {
            Some(pid) => Some(records::find_revision(&ctx.db, pid).await?),
            None => None,
        };
        return Err(Error::CustomError(
            axum::http::StatusCode::CONFLICT,
            loco_rs::controller::ErrorDetail::new(
                "stale_base_revision",
                format!(
                    "this edit was made from revision {} but the variant is now at {}",
                    payload.base_revision_pid,
                    competing
                        .as_ref()
                        .map_or_else(|| "none".to_string(), |r| r.pid.to_string())
                ),
            ),
        ));
    }

    let number = next_number(&txn, variant.pid).await?;
    let revision = insert_revision(
        &txn,
        NewRevision {
            variant_pid: variant.pid,
            number,
            schema_version: content_type.schema_version,
            body: &payload.body,
            author: caller.actor(),
            restored_from: None,
            references: &references,
        },
    )
    .await?;
    let variant_pid = variant.pid;
    let mut active: entry_variants::ActiveModel = variant.into();
    active.current_revision_pid = ActiveValue::set(Some(revision.pid));
    active.update(&txn).await?;
    Audit::record(
        &txn,
        "revision",
        revision.pid,
        "revision_created",
        caller.actor(),
        Some(serde_json::json!({
            "entry": entry.key,
            "locale": locale,
            "number": number,
            "note": payload.body.note,
            "blocks_sanitized": sanitized,
            "references": references.len(),
            "owner": entry.owner_ref,
        })),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "revision",
        "revision_created",
        &revision.pid.to_string(),
        &payload.body.title,
        caller.actor(),
        Some(serde_json::json!({ "variant": variant_pid.to_string(), "number": number })),
    )
    .await?;
    txn.commit().await?;
    record_save_metrics(sanitized);

    format::json(SavedView {
        revision_pid: revision.pid.to_string(),
        number,
        blocks_sanitized: sanitized,
        references: references.len(),
    })
}

/// `GET /api/entries/{pid}/variants/{locale}/revisions` — the history,
/// newest first, as summaries (the bodies are a separate read).
#[debug_handler]
async fn list_revisions(
    State(ctx): State<AppContext>,
    Path((pid, locale)): Path<(String, String)>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let entry = records::find_entry(&ctx.db, records::parse_pid(&pid)?).await?;
    let variant = records::find_variant_by_locale(&ctx.db, entry.pid, &locale).await?;
    let attrs = auth::variant_resource_attrs(&entry, &variant);
    let obligations = auth::authorize_record(&caller, Action::Read, &attrs).map_err(authz_error)?;
    let rows = revisions::Entity::find()
        .filter(revisions::Column::VariantPid.eq(variant.pid))
        .order_by_desc(revisions::Column::Number)
        .limit(500)
        .all(&ctx.db)
        .await?;
    let summaries: Vec<Value> = rows
        .iter()
        .map(|row| {
            auth::mask_if_required(
                serde_json::json!({
                "pid": row.pid,
                "number": row.number,
                "title": row.title,
                "author_ref": row.author_ref,
                "note": row.note,
                "restored_from_pid": row.restored_from_pid,
                "created_at": row.created_at,
                "is_current": Some(row.pid) == variant.current_revision_pid,
                "is_published": Some(row.pid) == variant.published_revision_pid,
                }),
                &obligations,
            )
        })
        .collect();
    format::json(summaries)
}

/// `GET /api/revisions/{pid}` — one revision in full.
///
/// The most sensitive read in the service: a revision body is
/// unpublished content by default. The record-level decision runs on
/// the revision's **variant**, and a `mask` obligation redacts the body
/// while leaving the structure — that the revision exists, its number,
/// when it was written — visible (CMS-R22).
#[debug_handler]
async fn get_revision(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let revision = records::find_revision(&ctx.db, records::parse_pid(&pid)?).await?;
    let variant = records::find_variant(&ctx.db, revision.variant_pid).await?;
    let entry = records::find_entry(&ctx.db, variant.entry_pid).await?;
    let attrs = auth::variant_resource_attrs(&entry, &variant);
    let obligations = auth::authorize_record(&caller, Action::Read, &attrs).map_err(authz_error)?;
    format::json(auth::mask_if_required(
        serde_json::to_value(revision)?,
        &obligations,
    ))
}

/// `GET /api/revisions/{from}/diff/{to}` — what changed between two
/// revisions.
#[debug_handler]
async fn diff_revisions(
    State(ctx): State<AppContext>,
    Path((from, to)): Path<(String, String)>,
) -> Result<Response> {
    let from = records::find_revision(&ctx.db, records::parse_pid(&from)?).await?;
    let to = records::find_revision(&ctx.db, records::parse_pid(&to)?).await?;
    if from.variant_pid != to.variant_pid {
        return Err(unprocessable(
            "revisions belong to different variants; a diff across variants would compare two different documents",
        ));
    }
    let (from_blocks, to_blocks) = (blocks_of(&from), blocks_of(&to));
    let (from_fields, to_fields) = (fields_of(&from), fields_of(&to));
    let result = diff::diff(
        diff::Side {
            title: &from.title,
            blocks: &from_blocks,
            fields: &from_fields,
            seo: &from.seo,
        },
        diff::Side {
            title: &to.title,
            blocks: &to_blocks,
            fields: &to_fields,
            seo: &to.seo,
        },
    );
    format::json(serde_json::json!({
        "from": { "pid": from.pid, "number": from.number },
        "to": { "pid": to.pid, "number": to.number },
        "diff": result,
    }))
}

/// A revision's blocks as a slice (an unreadable column yields none
/// rather than panicking).
fn blocks_of(revision: &revisions::Model) -> Vec<Value> {
    revision.blocks.as_array().cloned().unwrap_or_default()
}

/// A revision's field values as a map.
fn fields_of(revision: &revisions::Model) -> Map<String, Value> {
    revision.fields.as_object().cloned().unwrap_or_default()
}

/// `POST /api/entries/{pid}/variants/{locale}/restore` — restore an
/// earlier revision by writing a **new** one that copies it.
///
/// History is never rewound, only extended: the restore is itself a
/// recorded event, so "we reverted on Tuesday" stays visible (CMS-D3).
#[debug_handler]
async fn restore_revision(
    State(ctx): State<AppContext>,
    Path((pid, locale)): Path<(String, String)>,
    caller: MaybeAuthUser,
    Json(payload): Json<RestorePayload>,
) -> Result<Response> {
    let entry = records::find_entry(&ctx.db, records::parse_pid(&pid)?).await?;
    let content_type = content_type_of(&ctx.db, entry.site_pid, &entry.content_type_key).await?;
    let specs = specs_of(&content_type)?;
    let source = records::find_revision(&ctx.db, payload.revision_pid).await?;

    let txn = ctx.db.begin().await?;
    let variant = lock_variant(&txn, entry.pid, &locale).await?;
    if source.variant_pid != variant.pid {
        txn.rollback().await?;
        return Err(unprocessable(
            "that revision belongs to a different variant",
        ));
    }
    let mut body = Body {
        title: source.title.clone(),
        blocks: blocks_of(&source),
        fields: fields_of(&source),
        seo: source.seo.as_object().cloned().unwrap_or_default(),
        note: Some(
            payload
                .note
                .clone()
                .unwrap_or_else(|| format!("restored from revision {}", source.number)),
        ),
    };
    // Re-validate and re-extract: the content type may have tightened
    // since, and the restored body must be measured against today's
    // declaration rather than waved through because it was once valid.
    let (sanitized, references) = prepare_body(&mut body, &specs)?;
    let number = next_number(&txn, variant.pid).await?;
    let revision = insert_revision(
        &txn,
        NewRevision {
            variant_pid: variant.pid,
            number,
            schema_version: content_type.schema_version,
            body: &body,
            author: caller.actor(),
            restored_from: Some(source.pid),
            references: &references,
        },
    )
    .await?;
    let mut active: entry_variants::ActiveModel = variant.into();
    active.current_revision_pid = ActiveValue::set(Some(revision.pid));
    active.update(&txn).await?;
    Audit::record(
        &txn,
        "revision",
        revision.pid,
        "revision_restored",
        caller.actor(),
        Some(serde_json::json!({
            "entry": entry.key,
            "locale": locale,
            "number": number,
            "restored_from": source.pid,
            "restored_from_number": source.number,
            "reason": body.note,
        })),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "revision",
        "revision_restored",
        &revision.pid.to_string(),
        &body.title,
        caller.actor(),
        Some(serde_json::json!({ "restored_from": source.pid.to_string() })),
    )
    .await?;
    txn.commit().await?;
    record_save_metrics(sanitized);
    format::json(SavedView {
        revision_pid: revision.pid.to_string(),
        number,
        blocks_sanitized: sanitized,
        references: references.len(),
    })
}

/// `GET /api/entries/{pid}/usage` — where this entry is referenced.
#[debug_handler]
async fn entry_usage(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let entry = records::find_entry(&ctx.db, records::parse_pid(&pid)?).await?;
    let referrers =
        usage::live_referrers(&ctx.db, content_references::Column::ToEntryPid, entry.pid).await?;
    format::json(serde_json::json!({
        "entry_pid": entry.pid,
        "referrers": referrers,
        "counts_only_current_revisions": true,
    }))
}

/// `GET /api/assets/{pid}/usage` — where an asset is referenced.
#[debug_handler]
async fn asset_usage(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let asset = records::parse_pid(&pid)?;
    let referrers =
        usage::live_referrers(&ctx.db, content_references::Column::ToAssetPid, asset).await?;
    format::json(serde_json::json!({
        "asset_pid": asset,
        "referrers": referrers,
        "counts_only_current_revisions": true,
    }))
}

/// `DELETE /api/entries/{pid}` query — the reasoned override.
#[derive(Debug, Deserialize)]
struct DeleteParams {
    /// Delete despite live references. Requires `reason`.
    #[serde(default)]
    force: bool,
    /// Why — recorded in the audit row, alongside the references the
    /// deletion is knowingly breaking.
    #[serde(default)]
    reason: Option<String>,
}

/// `DELETE /api/entries/{pid}` — soft-delete, **refused** while a live
/// current revision still references this entry (CMS-D8).
///
/// The refusal can be overridden with `?force=true&reason=…`, because
/// sometimes a thing genuinely must go and the referrers must be fixed
/// afterwards. The override is deliberately awkward: it needs an
/// explicit flag *and* a reason, and the audit row records every
/// reference it broke, so the cleanup is a work-list rather than a
/// mystery. It does **not** override the published-variant check —
/// unpublishing is a separate, reversible decision, and taking a live
/// page down should be made on purpose rather than as a side effect of
/// a delete.
#[debug_handler]
async fn delete_entry(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    Query(params): Query<DeleteParams>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let entry = records::find_entry(&ctx.db, records::parse_pid(&pid)?).await?;
    let referrers =
        usage::live_referrers(&ctx.db, content_references::Column::ToEntryPid, entry.pid).await?;
    let forced = params.force && !referrers.is_empty();
    if !referrers.is_empty() {
        let keys: Vec<&str> = referrers.iter().map(|r| r.entry_key.as_str()).collect();
        if !params.force {
            return Err(conflict(&format!(
                "entry {} is still referenced by {} live revision(s): {keys:?} \
                 — delete anyway with ?force=true&reason=…",
                entry.key,
                referrers.len()
            )));
        }
        if params.reason.as_ref().is_none_or(|r| r.trim().is_empty()) {
            return Err(unprocessable(
                "a forced delete requires a reason: it knowingly breaks live references",
            ));
        }
    }
    let published = entry_variants::Entity::find()
        .filter(entry_variants::Column::EntryPid.eq(entry.pid))
        .filter(entry_variants::Column::DeletedAt.is_null())
        .filter(entry_variants::Column::PublishedRevisionPid.is_not_null())
        .count(&ctx.db)
        .await?;
    if published > 0 {
        return Err(conflict(&format!(
            "entry {} has {published} published variant(s); unpublish before deleting",
            entry.key
        )));
    }

    let txn = ctx.db.begin().await?;
    let entry_pid = entry.pid;
    let key = entry.key.clone();
    let mut active: entries::ActiveModel = entry.into();
    active.deleted_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    active.update(&txn).await?;
    // Variants go with the entry; revisions do not — the chain is
    // append-only and stays readable (CMS-D3).
    entry_variants::Entity::update_many()
        .col_expr(
            entry_variants::Column::DeletedAt,
            sea_orm::sea_query::Expr::current_timestamp(),
        )
        .filter(entry_variants::Column::EntryPid.eq(entry_pid))
        .filter(entry_variants::Column::DeletedAt.is_null())
        .exec(&txn)
        .await?;
    Audit::record(
        &txn,
        "entry",
        entry_pid,
        if forced { "force_deleted" } else { "deleted" },
        caller.actor(),
        forced.then(|| {
            serde_json::json!({
                "reason": params.reason,
                "broken_references": referrers,
            })
        }),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "entry",
        "deleted",
        &entry_pid.to_string(),
        &key,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::empty_json()
}

/// `GET /api/entries/{pid}/variants/{locale}/publish-check` — what
/// stands between this variant and publication (CMS-R11).
///
/// Exposed as a **read** now so an editor can see the list before the
/// publish transition exists (CMS-T12); that transition will call the
/// same [`crate::rules::gate`] function rather than reimplementing it,
/// so the preview cannot disagree with the gate.
///
/// The route check (a routable type needs a valid unique path) joins
/// when routes do (CMS-T16); it is absent here rather than faked.
/// Resolve the publish blockers for one revision of one variant
/// (CMS-R11).
///
/// Shared by the `publish-check` read and the publish transition, so
/// the preview an editor sees and the gate that refuses them cannot
/// disagree. The edges come from the stored reference index rather than
/// a re-walk of the document, so this also agrees with the
/// delete-refusal and the health findings by construction.
///
/// # Errors
///
/// When a lookup fails, or the content type's stored schema no longer
/// reads.
/// Generic over the connection so a caller **inside a transaction can
/// pass that transaction**. Asking the pool for a second connection
/// while holding one is a deadlock waiting for a busy pool — and, with
/// the single-connection test pool, an immediate one. (Found exactly
/// that way: the scheduled-publish sweep timed out acquiring a
/// connection it was itself holding.)
pub(crate) async fn publish_blockers_for<C: sea_orm::ConnectionTrait>(
    db: &C,
    entry: &entries::Model,
    revision: &revisions::Model,
) -> Result<Vec<gate::Blocker>> {
    let content_type = content_type_of(db, entry.site_pid, &entry.content_type_key).await?;
    let specs = specs_of(&content_type)?;
    let edges = content_references::Entity::find()
        .filter(content_references::Column::FromRevisionPid.eq(revision.pid))
        .limit(500)
        .all(db)
        .await?;
    let mut assets = Vec::new();
    let mut referenced_entries = Vec::new();
    for edge in edges {
        if let Some(asset_pid) = edge.to_asset_pid {
            let row = records::find_asset(db, asset_pid).await.ok();
            assets.push(gate::ReferencedAsset {
                pid: asset_pid,
                exists: row.is_some(),
                kind: row.as_ref().map(|a| a.kind.clone()),
                alt_text: row.and_then(|a| a.alt_text),
            });
        }
        if let Some(entry_pid) = edge.to_entry_pid {
            let row = records::find_entry(db, entry_pid).await.ok();
            referenced_entries.push(gate::ReferencedEntry {
                pid: entry_pid,
                exists: row.is_some(),
                key: row.map(|e| e.key),
            });
        }
    }
    // Does this variant have an address? Only asked for routable
    // types, and only answerable now that routes exist (CMS-T16).
    let has_path = crate::models::_entities::routes::Entity::find()
        .filter(crate::models::_entities::routes::Column::VariantPid.eq(revision.variant_pid))
        .filter(crate::models::_entities::routes::Column::IsCurrent.eq(true))
        .one(db)
        .await?
        .is_some();
    let fields = fields_of(revision);
    Ok(gate::publish_blockers(&gate::Candidate {
        routable: content_type.routable,
        has_path,
        title: &revision.title,
        fields: &fields,
        specs: &specs,
        assets: &assets,
        entries: &referenced_entries,
    }))
}

/// `GET /api/entries/{pid}/variants/{locale}/publish-check` — what
/// stands between this variant and publication (CMS-R11).
#[debug_handler]
async fn publish_check(
    State(ctx): State<AppContext>,
    Path((pid, locale)): Path<(String, String)>,
) -> Result<Response> {
    let entry = records::find_entry(&ctx.db, records::parse_pid(&pid)?).await?;
    let variant = records::find_variant_by_locale(&ctx.db, entry.pid, &locale).await?;
    let Some(revision_pid) = variant.current_revision_pid else {
        return Err(unprocessable("this variant has no revision to publish"));
    };
    let revision = records::find_revision(&ctx.db, revision_pid).await?;
    let blockers = publish_blockers_for(&ctx.db, &entry, &revision).await?;
    format::json(serde_json::json!({
        "entry_pid": entry.pid,
        "locale": locale,
        "revision_pid": revision.pid,
        "status": variant.status,
        "ready": blockers.is_empty(),
        "blockers": blockers,
    }))
}

/// The entry / variant / revision routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/sites/{pid}/entries", post(create_entry).get(list_entries))
        .add("/entries/{pid}", get(get_entry).delete(delete_entry))
        .add("/entries/{pid}/usage", get(entry_usage))
        .add("/entries/{pid}/variants", post(create_variant))
        .add("/entries/{pid}/variants/{locale}", get(get_variant))
        .add(
            "/entries/{pid}/variants/{locale}/revisions",
            post(save_revision).get(list_revisions),
        )
        .add(
            "/entries/{pid}/variants/{locale}/restore",
            post(restore_revision),
        )
        .add(
            "/entries/{pid}/variants/{locale}/publish-check",
            get(publish_check),
        )
        .add("/revisions/{pid}", get(get_revision))
        .add("/revisions/{from}/diff/{to}", get(diff_revisions))
        .add("/assets/{pid}/usage", get(asset_usage))
}
