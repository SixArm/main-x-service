//! The asset library (CMS-R6–R8): uploads, metadata, declared
//! renditions, replace, orphan reporting, and the delete-refusal that
//! rides on the reference index.
//!
//! The upload path is the security-sensitive one, so its order is
//! deliberate and worth reading:
//!
//! 1. **Cap the bytes** — a body limit on the route, and an explicit
//!    check, so an oversize upload is refused rather than buffered.
//! 2. **Type from the bytes**, not from what the caller declared; a
//!    declaration that disagrees is a refusal
//!    ([`crate::rules::media`]).
//! 3. **Check the site's quota** before storing anything.
//! 4. **Address by content** — the storage key is the SHA-256 of the
//!    bytes, so identical uploads deduplicate and an uploader can never
//!    choose where their bytes land.
//! 5. **Store, then record.** The filename the caller sent is metadata
//!    for display only; it never becomes a path.
//!
//! On the way out, bytes are served with `X-Content-Type-Options:
//! nosniff` and a disposition chosen by kind — so a stored file cannot
//! be coaxed into being interpreted as something executable.

use axum::extract::{Multipart, Query, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response as AxumResponse};
use loco_rs::prelude::*;
use sea_orm::{QueryOrder, QuerySelect, TransactionTrait};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::{conflict, ensure_valid, unprocessable};
use crate::auth::MaybeAuthUser;
use crate::models::_entities::{assets, content_references, renditions};
use crate::models::audit_logs::Model as Audit;
use crate::models::{records, usage};
use crate::rules::media::{self, Verdict};
use crate::rules::tokens;
use crate::storage::{self, ArtifactStore};
use crate::streaming;
use crate::validation::Problems;

/// Default per-upload byte cap (`CMS_MAX_UPLOAD_BYTES`).
const DEFAULT_MAX_UPLOAD_BYTES: usize = 25 * 1024 * 1024;
/// Default per-site quota (`CMS_SITE_QUOTA_BYTES`).
const DEFAULT_SITE_QUOTA_BYTES: i64 = 1024 * 1024 * 1024;

/// The per-upload byte cap.
#[must_use]
pub fn max_upload_bytes() -> usize {
    std::env::var("CMS_MAX_UPLOAD_BYTES")
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(DEFAULT_MAX_UPLOAD_BYTES)
}

/// The per-site storage quota.
fn site_quota_bytes() -> i64 {
    std::env::var("CMS_SITE_QUOTA_BYTES")
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(DEFAULT_SITE_QUOTA_BYTES)
}

/// The process-wide artifact store, built once.
async fn store() -> Result<&'static dyn ArtifactStore> {
    static STORE: tokio::sync::OnceCell<Box<dyn ArtifactStore>> =
        tokio::sync::OnceCell::const_new();
    let boxed = STORE
        .get_or_try_init(|| async { storage::from_env().await })
        .await?;
    Ok(boxed.as_ref())
}

/// The parts of a multipart upload this endpoint understands.
#[derive(Debug, Default)]
struct Upload {
    bytes: Vec<u8>,
    filename: Option<String>,
    declared_mime: Option<String>,
    title: Option<String>,
    alt_text: Option<String>,
    caption: Option<String>,
    credit: Option<String>,
    licence: Option<String>,
    tags: Vec<String>,
    on_duplicate: Option<String>,
}

/// Read the upload, refusing anything past the byte cap.
async fn read_upload(mut multipart: Multipart) -> Result<Upload> {
    let cap = max_upload_bytes();
    let mut upload = Upload::default();
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| unprocessable(&format!("malformed upload: {e}")))?
    {
        let name = field.name().unwrap_or_default().to_string();
        if name == "file" {
            upload.filename = field.file_name().map(ToString::to_string);
            upload.declared_mime = field.content_type().map(ToString::to_string);
            let bytes = field
                .bytes()
                .await
                .map_err(|e| unprocessable(&format!("could not read the uploaded file: {e}")))?;
            if bytes.len() > cap {
                return Err(Error::CustomError(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    loco_rs::controller::ErrorDetail::new(
                        "too_large",
                        &format!(
                            "the upload is {} bytes; the limit is {cap} (CMS_MAX_UPLOAD_BYTES)",
                            bytes.len()
                        ),
                    ),
                ));
            }
            upload.bytes = bytes.to_vec();
            continue;
        }
        let text = field.text().await.unwrap_or_default();
        match name.as_str() {
            "title" => upload.title = Some(text),
            "alt_text" => upload.alt_text = Some(text),
            "caption" => upload.caption = Some(text),
            "credit" => upload.credit = Some(text),
            "licence" => upload.licence = Some(text),
            "on_duplicate" => upload.on_duplicate = Some(text),
            "tags" => {
                upload.tags = text
                    .split(',')
                    .map(|t| t.trim().to_string())
                    .filter(|t| !t.is_empty())
                    .collect();
            }
            _ => {}
        }
    }
    if upload.bytes.is_empty() {
        return Err(unprocessable("the upload has no `file` part"));
    }
    Ok(upload)
}

/// The SHA-256 content address, hex-encoded.
fn checksum(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().fold(String::new(), |mut acc, byte| {
        use std::fmt::Write as _;
        let _ = write!(acc, "{byte:02x}");
        acc
    })
}

/// The sharded storage key for a checksum. Sharding keeps a local
/// directory from accumulating a million sibling files, which some
/// filesystems handle badly and every `ls` handles worse.
fn storage_key(checksum: &str) -> String {
    let (head, rest) = checksum.split_at(2.min(checksum.len()));
    let (mid, tail) = rest.split_at(2.min(rest.len()));
    format!("sha256/{head}/{mid}/{tail}")
}

/// Validate the metadata fields that accompany an upload.
fn validate_metadata(upload: &Upload) -> Vec<String> {
    let mut problems = Problems::new();
    problems.cap_opt("title", upload.title.as_deref());
    problems.cap_opt("alt_text", upload.alt_text.as_deref());
    problems.cap_opt("caption", upload.caption.as_deref());
    problems.cap_opt("credit", upload.credit.as_deref());
    problems.cap_opt("licence", upload.licence.as_deref());
    problems.cap_list("tags", &upload.tags);
    if let Some(mode) = &upload.on_duplicate {
        problems.require_token("on_duplicate", &["reuse", "new_record"], mode);
    }
    problems.into_vec()
}

/// The site's current stored bytes.
async fn site_usage_bytes(db: &DatabaseConnection, site_pid: Uuid) -> Result<i64> {
    let rows = assets::Entity::find()
        .filter(assets::Column::SitePid.eq(site_pid))
        .filter(assets::Column::DeletedAt.is_null())
        .all(db)
        .await?;
    Ok(rows.iter().map(|row| row.byte_size).sum())
}

/// `POST /api/sites/{pid}/assets` — upload.
#[debug_handler]
#[allow(clippy::too_many_lines)] // the ordered upload pipeline, read top to bottom
async fn upload(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    caller: MaybeAuthUser,
    multipart: Multipart,
) -> Result<Response> {
    let site = records::find_site(&ctx.db, records::parse_pid(&pid)?).await?;
    let upload = read_upload(multipart).await?;
    ensure_valid(&validate_metadata(&upload))?;

    // Type from the bytes; a disagreeing declaration is the signal.
    let media = match media::classify(&upload.bytes, upload.declared_mime.as_deref()) {
        Verdict::Accepted(media) => media,
        Verdict::Refused(reason) => return Err(unprocessable(&reason)),
    };

    let byte_size = i64::try_from(upload.bytes.len()).unwrap_or(i64::MAX);
    let quota = site_quota_bytes();
    let used = site_usage_bytes(&ctx.db, site.pid).await?;
    if used.saturating_add(byte_size) > quota {
        return Err(Error::CustomError(
            StatusCode::PAYLOAD_TOO_LARGE,
            loco_rs::controller::ErrorDetail::new(
                "quota_exceeded",
                &format!(
                    "site {} has used {used} of {quota} bytes; this upload of {byte_size} would \
                     exceed the quota (CMS_SITE_QUOTA_BYTES)",
                    site.key
                ),
            ),
        ));
    }

    let checksum = checksum(&upload.bytes);
    let reuse = upload.on_duplicate.as_deref() != Some("new_record");
    if reuse
        && let Some(existing) = assets::Entity::find()
            .filter(assets::Column::SitePid.eq(site.pid))
            .filter(assets::Column::ChecksumSha256.eq(checksum.clone()))
            .filter(assets::Column::DeletedAt.is_null())
            .one(&ctx.db)
            .await?
    {
        // Identical bytes already here. Returning the existing asset is
        // the default because the alternative — a second row pointing at
        // the same object — quietly splits an asset's usage history in
        // two, and an editor re-uploading the same logo means "use this
        // logo", not "make another one".
        return format!(
            "{{\"pid\":\"{}\",\"deduplicated\":true,\"checksum_sha256\":\"{checksum}\"}}",
            existing.pid
        )
        .parse::<Value>()
        .map_err(|e| Error::Message(e.to_string()))
        .and_then(format::json);
    }

    let key = storage_key(&checksum);
    let storage_ref = store().await?.put(&key, &upload.bytes).await?;
    let (width, height) = media::dimensions(&upload.bytes, media.mime).unzip();

    let txn = ctx.db.begin().await?;
    let row = assets::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        site_pid: ActiveValue::set(Some(site.pid)),
        kind: ActiveValue::set(media.kind.to_string()),
        mime: ActiveValue::set(media.mime.to_string()),
        byte_size: ActiveValue::set(byte_size),
        checksum_sha256: ActiveValue::set(checksum.clone()),
        storage_ref: ActiveValue::set(storage_ref),
        // Metadata for display only: the stored key is the checksum, so
        // a filename can never be a path (CMS-D9).
        original_filename: ActiveValue::set(upload.filename.clone()),
        title: ActiveValue::set(upload.title.clone()),
        alt_text: ActiveValue::set(upload.alt_text.clone()),
        caption: ActiveValue::set(upload.caption.clone()),
        credit: ActiveValue::set(upload.credit.clone()),
        licence: ActiveValue::set(upload.licence.clone()),
        tags: ActiveValue::set(serde_json::json!(upload.tags)),
        width: ActiveValue::set(width.and_then(|w| i32::try_from(w).ok())),
        height: ActiveValue::set(height.and_then(|h| i32::try_from(h).ok())),
        duration_ms: ActiveValue::set(None),
        uploaded_by_ref: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(
        &txn,
        "asset",
        row.pid,
        "uploaded",
        caller.actor(),
        Some(serde_json::json!({
            "site": site.key,
            "mime": row.mime,
            "byte_size": row.byte_size,
            "checksum_sha256": checksum,
            "original_filename": row.original_filename,
        })),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "asset",
        "asset_uploaded",
        &row.pid.to_string(),
        row.title.as_deref().unwrap_or(media.mime),
        caller.actor(),
        Some(serde_json::json!({ "mime": row.mime, "kind": row.kind })),
    )
    .await?;
    txn.commit().await?;
    crate::metrics::Metrics::global().asset_uploaded_total.inc();

    format::json(serde_json::json!({
        "pid": row.pid.to_string(),
        "kind": row.kind,
        "mime": row.mime,
        "byte_size": row.byte_size,
        "checksum_sha256": checksum,
        "width": row.width,
        "height": row.height,
        "deduplicated": false,
    }))
}

/// `GET /api/sites/{pid}/assets` — the library, filterable.
#[derive(Debug, Deserialize)]
struct ListParams {
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    tag: Option<String>,
    /// Free-text over title, caption, and original filename.
    #[serde(default)]
    q: Option<String>,
}

#[debug_handler]
async fn list(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    Query(params): Query<ListParams>,
) -> Result<Response> {
    let site = records::find_site(&ctx.db, records::parse_pid(&pid)?).await?;
    let mut query = assets::Entity::find()
        .filter(assets::Column::SitePid.eq(site.pid))
        .filter(assets::Column::DeletedAt.is_null());
    if let Some(kind) = params.kind.as_deref() {
        query = query.filter(assets::Column::Kind.eq(kind));
    }
    let rows = query
        .order_by_desc(assets::Column::Id)
        .limit(1000)
        .all(&ctx.db)
        .await?;
    // Tag and text filtering happen in Rust: `tags` is JSONB and the
    // candidate set is already site-scoped and capped, so this stays a
    // small in-memory filter rather than a backend-specific JSON query.
    let needle = params.q.map(|q| q.trim().to_lowercase());
    let rows: Vec<assets::Model> = rows
        .into_iter()
        .filter(|row| {
            params.tag.as_ref().is_none_or(|tag| {
                row.tags
                    .as_array()
                    .is_some_and(|tags| tags.iter().any(|t| t.as_str() == Some(tag.as_str())))
            })
        })
        .filter(|row| {
            needle.as_ref().is_none_or(|needle| {
                [&row.title, &row.caption, &row.original_filename]
                    .iter()
                    .filter_map(|field| field.as_deref())
                    .any(|text| text.to_lowercase().contains(needle))
            })
        })
        .collect();
    format::json(rows)
}

/// `GET /api/assets/{pid}` — the asset with its declared renditions.
#[debug_handler]
async fn show(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let asset = records::find_asset(&ctx.db, records::parse_pid(&pid)?).await?;
    let rendition_rows = renditions::Entity::find()
        .filter(renditions::Column::AssetPid.eq(asset.pid))
        .filter(renditions::Column::DeletedAt.is_null())
        .order_by_asc(renditions::Column::Id)
        .all(&ctx.db)
        .await?;
    format::json(serde_json::json!({
        "asset": asset,
        "renditions": rendition_rows,
        // Say which renditions can actually be fetched, so a channel
        // picks rather than guessing a URL pattern (spec `assets.md`).
        "available_renditions": rendition_rows
            .iter()
            .filter(|r| r.state == "produced" && r.storage_ref.is_some())
            .map(|r| r.key.clone())
            .collect::<Vec<_>>(),
    }))
}

/// `GET /api/assets/{pid}/content` — the bytes.
///
/// Served with `nosniff` and a disposition chosen by kind: images,
/// audio, and video inline; everything else as an attachment. A PDF can
/// carry script in a viewer, so it downloads rather than rendering in
/// the page's origin.
#[debug_handler]
async fn content(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<AxumResponse> {
    let Ok(asset) = records::find_asset(&ctx.db, records::parse_pid(&pid)?).await else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    let bytes = match store().await {
        Ok(store) => match store.get(&asset.storage_ref).await {
            Ok(bytes) => bytes,
            Err(error) => {
                tracing::warn!(%error, asset = %asset.pid, "asset bytes are unreadable");
                return Ok(StatusCode::NOT_FOUND.into_response());
            }
        },
        Err(error) => {
            tracing::error!(%error, "artifact store unavailable");
            return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }
    };
    let inline = matches!(asset.kind.as_str(), "image" | "audio" | "video");
    let filename = asset
        .original_filename
        .as_deref()
        .unwrap_or("download")
        .replace(['"', '\\', '\r', '\n'], "_");
    let disposition = if inline {
        format!("inline; filename=\"{filename}\"")
    } else {
        format!("attachment; filename=\"{filename}\"")
    };
    let mut response = bytes.into_response();
    let headers = response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(&asset.mime) {
        headers.insert(header::CONTENT_TYPE, value);
    }
    headers.insert(
        header::HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    if let Ok(value) = HeaderValue::from_str(&disposition) {
        headers.insert(header::CONTENT_DISPOSITION, value);
    }
    // The bytes are immutable, but this URL is not: `replace` swaps the
    // content behind the same pid, so a long-lived cache would serve the
    // old logo forever.
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-cache"),
    );
    Ok(response)
}

/// `PUT /api/assets/{pid}` — metadata only; the bytes are immutable
/// (use `replace`).
#[derive(Debug, Deserialize)]
struct MetadataPayload {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    alt_text: Option<String>,
    #[serde(default)]
    caption: Option<String>,
    #[serde(default)]
    credit: Option<String>,
    #[serde(default)]
    licence: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

#[debug_handler]
async fn update_metadata(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    caller: MaybeAuthUser,
    Json(payload): Json<MetadataPayload>,
) -> Result<Response> {
    let asset = records::find_asset(&ctx.db, records::parse_pid(&pid)?).await?;
    let mut problems = Problems::new();
    problems.cap_opt("title", payload.title.as_deref());
    problems.cap_opt("alt_text", payload.alt_text.as_deref());
    problems.cap_opt("caption", payload.caption.as_deref());
    problems.cap_opt("credit", payload.credit.as_deref());
    problems.cap_opt("licence", payload.licence.as_deref());
    problems.cap_list("tags", &payload.tags);
    ensure_valid(&problems.into_vec())?;

    let txn = ctx.db.begin().await?;
    let asset_pid = asset.pid;
    let mut active: assets::ActiveModel = asset.into();
    active.title = ActiveValue::set(payload.title.clone());
    active.alt_text = ActiveValue::set(payload.alt_text.clone());
    active.caption = ActiveValue::set(payload.caption.clone());
    active.credit = ActiveValue::set(payload.credit.clone());
    active.licence = ActiveValue::set(payload.licence.clone());
    active.tags = ActiveValue::set(serde_json::json!(payload.tags));
    let row = active.update(&txn).await?;
    Audit::record(
        &txn,
        "asset",
        asset_pid,
        "updated",
        caller.actor(),
        Some(serde_json::json!({ "alt_text_present": row.alt_text.is_some() })),
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `POST /api/assets/{pid}/replace` — new bytes, same asset identity.
///
/// The operation that "delete and re-upload" silently botches: every
/// reference keeps pointing at this asset, so fixing a logo everywhere
/// is one call rather than a hunt through every page that used it.
#[debug_handler]
async fn replace(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    caller: MaybeAuthUser,
    multipart: Multipart,
) -> Result<Response> {
    let asset = records::find_asset(&ctx.db, records::parse_pid(&pid)?).await?;
    let upload = read_upload(multipart).await?;
    let media = match media::classify(&upload.bytes, upload.declared_mime.as_deref()) {
        Verdict::Accepted(media) => media,
        Verdict::Refused(reason) => return Err(unprocessable(&reason)),
    };
    // Replacing an image with a PDF would break every layout that shows
    // it; a replacement has to be the same kind of thing.
    if media.kind != asset.kind {
        return Err(unprocessable(&format!(
            "cannot replace a {} with a {} ({}); upload a new asset instead",
            asset.kind, media.kind, media.mime
        )));
    }
    let checksum = checksum(&upload.bytes);
    let key = storage_key(&checksum);
    let storage_ref = store().await?.put(&key, &upload.bytes).await?;
    let (width, height) = media::dimensions(&upload.bytes, media.mime).unzip();

    let txn = ctx.db.begin().await?;
    let asset_pid = asset.pid;
    let previous = asset.checksum_sha256.clone();
    let mut active: assets::ActiveModel = asset.into();
    active.mime = ActiveValue::set(media.mime.to_string());
    active.byte_size = ActiveValue::set(i64::try_from(upload.bytes.len()).unwrap_or(i64::MAX));
    active.checksum_sha256 = ActiveValue::set(checksum.clone());
    active.storage_ref = ActiveValue::set(storage_ref);
    active.original_filename = ActiveValue::set(upload.filename.clone());
    active.width = ActiveValue::set(width.and_then(|w| i32::try_from(w).ok()));
    active.height = ActiveValue::set(height.and_then(|h| i32::try_from(h).ok()));
    let row = active.update(&txn).await?;
    // Declared renditions describe the *old* bytes, so they go back to
    // `declared`: claiming a produced rendition of content that no
    // longer exists would serve the previous image at a stale URL.
    renditions::Entity::update_many()
        .col_expr(
            renditions::Column::State,
            sea_orm::sea_query::Expr::value("declared"),
        )
        .col_expr(
            renditions::Column::StorageRef,
            sea_orm::sea_query::Expr::value(Option::<String>::None),
        )
        .filter(renditions::Column::AssetPid.eq(asset_pid))
        .filter(renditions::Column::DeletedAt.is_null())
        .exec(&txn)
        .await?;
    Audit::record(
        &txn,
        "asset",
        asset_pid,
        "replaced",
        caller.actor(),
        Some(serde_json::json!({
            "checksum_from": previous,
            "checksum_to": checksum,
            "mime": row.mime,
        })),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "asset",
        "asset_replaced",
        &asset_pid.to_string(),
        row.title.as_deref().unwrap_or(&row.mime),
        caller.actor(),
        Some(serde_json::json!({ "checksum_sha256": checksum })),
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `DELETE /api/assets/{pid}` query — the reasoned override.
#[derive(Debug, Deserialize)]
struct DeleteParams {
    #[serde(default)]
    force: bool,
    #[serde(default)]
    reason: Option<String>,
}

/// `DELETE /api/assets/{pid}` — soft-delete, **refused** while a live
/// current revision still uses it (CMS-D8), overridable with a reason
/// that records every reference it breaks.
///
/// The stored bytes are **not** removed. Other assets may share the
/// same content address, and a soft delete is meant to be reversible;
/// reclaiming storage is a separate, deliberate sweep.
#[debug_handler]
async fn remove(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    Query(params): Query<DeleteParams>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let asset = records::find_asset(&ctx.db, records::parse_pid(&pid)?).await?;
    let referrers =
        usage::live_referrers(&ctx.db, content_references::Column::ToAssetPid, asset.pid).await?;
    let forced = params.force && !referrers.is_empty();
    if !referrers.is_empty() {
        if !params.force {
            let keys: Vec<&str> = referrers.iter().map(|r| r.entry_key.as_str()).collect();
            return Err(conflict(&format!(
                "asset is still used by {} live revision(s): {keys:?} — delete anyway with \
                 ?force=true&reason=…",
                referrers.len()
            )));
        }
        if params.reason.as_ref().is_none_or(|r| r.trim().is_empty()) {
            return Err(unprocessable(
                "a forced delete requires a reason: it knowingly breaks live references",
            ));
        }
    }

    let txn = ctx.db.begin().await?;
    let asset_pid = asset.pid;
    let label = asset.title.clone().unwrap_or_else(|| asset.mime.clone());
    let mut active: assets::ActiveModel = asset.into();
    active.deleted_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    active.update(&txn).await?;
    Audit::record(
        &txn,
        "asset",
        asset_pid,
        if forced { "force_deleted" } else { "deleted" },
        caller.actor(),
        forced.then(
            || serde_json::json!({ "reason": params.reason, "broken_references": referrers }),
        ),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "asset",
        "asset_deleted",
        &asset_pid.to_string(),
        &label,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::empty_json()
}

/// `POST /api/assets/{pid}/renditions` — declare a derived variant.
#[derive(Debug, Deserialize)]
struct RenditionPayload {
    key: String,
    #[serde(default)]
    width: Option<i32>,
    #[serde(default)]
    height: Option<i32>,
    #[serde(default = "default_format")]
    format: String,
}

fn default_format() -> String {
    "webp".to_string()
}

#[debug_handler]
async fn declare_rendition(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    caller: MaybeAuthUser,
    Json(payload): Json<RenditionPayload>,
) -> Result<Response> {
    let asset = records::find_asset(&ctx.db, records::parse_pid(&pid)?).await?;
    let mut problems = Problems::new();
    problems.require_key("key", &payload.key);
    problems.require_token("format", &["webp", "jpeg", "png", "avif"], &payload.format);
    if payload.width.is_some_and(|w| w <= 0) || payload.height.is_some_and(|h| h <= 0) {
        problems.push("width and height must be positive".to_string());
    }
    ensure_valid(&problems.into_vec())?;
    if asset.kind != "image" {
        return Err(unprocessable(&format!(
            "renditions are an image concern; this asset is a {}",
            asset.kind
        )));
    }
    if renditions::Entity::find()
        .filter(renditions::Column::AssetPid.eq(asset.pid))
        .filter(renditions::Column::Key.eq(payload.key.clone()))
        .filter(renditions::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await?
        .is_some()
    {
        return Err(conflict(&format!(
            "rendition {:?} is already declared for this asset",
            payload.key
        )));
    }
    let txn = ctx.db.begin().await?;
    let row = renditions::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        asset_pid: ActiveValue::set(asset.pid),
        key: ActiveValue::set(payload.key.clone()),
        width: ActiveValue::set(payload.width),
        height: ActiveValue::set(payload.height),
        format: ActiveValue::set(payload.format.clone()),
        storage_ref: ActiveValue::set(None),
        // Declared, not produced: v1 records the intent, and the
        // producing worker is a documented seam (spec `assets.md`).
        state: ActiveValue::set("declared".to_string()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(
        &txn,
        "rendition",
        row.pid,
        "declared",
        caller.actor(),
        Some(serde_json::json!({ "asset": asset.pid, "key": row.key })),
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `PUT /api/renditions/{pid}` — record the outcome of producing one.
#[derive(Debug, Deserialize)]
struct RenditionOutcome {
    state: String,
    #[serde(default)]
    storage_ref: Option<String>,
}

#[debug_handler]
async fn update_rendition(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    caller: MaybeAuthUser,
    Json(payload): Json<RenditionOutcome>,
) -> Result<Response> {
    let rendition = records::find_rendition(&ctx.db, records::parse_pid(&pid)?).await?;
    let mut problems = Problems::new();
    problems.require_token("state", tokens::RENDITION_STATES, &payload.state);
    ensure_valid(&problems.into_vec())?;
    // A rendition is only "produced" if there are bytes to serve. Saying
    // otherwise would put a URL in a delivery payload that 404s.
    if payload.state == "produced" && payload.storage_ref.is_none() {
        return Err(unprocessable(
            "a produced rendition needs a storage_ref; without bytes it is still declared",
        ));
    }
    let txn = ctx.db.begin().await?;
    let rendition_pid = rendition.pid;
    let mut active: renditions::ActiveModel = rendition.into();
    active.state = ActiveValue::set(payload.state.clone());
    active.storage_ref = ActiveValue::set(payload.storage_ref.clone());
    let row = active.update(&txn).await?;
    Audit::record(
        &txn,
        "rendition",
        rendition_pid,
        &format!("rendition_{}", payload.state),
        caller.actor(),
        None,
    )
    .await?;
    if payload.state == "produced" {
        streaming::emit_on(
            &txn,
            "rendition",
            "rendition_produced",
            &rendition_pid.to_string(),
            &row.key,
            caller.actor(),
            None,
        )
        .await?;
    }
    txn.commit().await?;
    format::json(row)
}

/// `GET /api/sites/{pid}/assets/orphans` — assets nothing references.
///
/// Reported, never auto-deleted: "unreferenced today" and "safe to
/// destroy" are different claims, and only a person can make the second
/// one (spec `assets.md`).
#[debug_handler]
async fn orphans(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let site = records::find_site(&ctx.db, records::parse_pid(&pid)?).await?;
    let rows = assets::Entity::find()
        .filter(assets::Column::SitePid.eq(site.pid))
        .filter(assets::Column::DeletedAt.is_null())
        .order_by_asc(assets::Column::Id)
        .limit(1000)
        .all(&ctx.db)
        .await?;
    let mut orphans = Vec::new();
    let mut bytes_reclaimable: i64 = 0;
    for row in rows {
        let referrers =
            usage::live_referrers(&ctx.db, content_references::Column::ToAssetPid, row.pid).await?;
        if referrers.is_empty() {
            bytes_reclaimable = bytes_reclaimable.saturating_add(row.byte_size);
            orphans.push(serde_json::json!({
                "pid": row.pid,
                "title": row.title,
                "mime": row.mime,
                "byte_size": row.byte_size,
                "uploaded_at": row.created_at,
            }));
        }
    }
    format::json(serde_json::json!({
        "as_of": chrono::Utc::now(),
        "rule": "no reference from the current revision of any live variant",
        "orphans": orphans,
        "bytes_reclaimable": bytes_reclaimable,
        "auto_deleted": false,
    }))
}

/// A quota summary, so an editor learns about the ceiling before
/// hitting it rather than through a failed upload.
#[derive(Debug, Serialize)]
struct QuotaView {
    used_bytes: i64,
    quota_bytes: i64,
    max_upload_bytes: usize,
    accepted_types: Vec<&'static str>,
}

/// `GET /api/sites/{pid}/assets/quota`.
#[debug_handler]
async fn quota(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let site = records::find_site(&ctx.db, records::parse_pid(&pid)?).await?;
    format::json(QuotaView {
        used_bytes: site_usage_bytes(&ctx.db, site.pid).await?,
        quota_bytes: site_quota_bytes(),
        max_upload_bytes: max_upload_bytes(),
        accepted_types: media::ACCEPTED.iter().map(|m| m.mime).collect(),
    })
}

/// The asset routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/sites/{pid}/assets", post(upload).get(list))
        .add("/sites/{pid}/assets/orphans", get(orphans))
        .add("/sites/{pid}/assets/quota", get(quota))
        .add(
            "/assets/{pid}",
            get(show).put(update_metadata).delete(remove),
        )
        .add("/assets/{pid}/content", get(content))
        .add("/assets/{pid}/replace", post(replace))
        .add("/assets/{pid}/renditions", post(declare_rendition))
        .add("/renditions/{pid}", put(update_rendition))
}
