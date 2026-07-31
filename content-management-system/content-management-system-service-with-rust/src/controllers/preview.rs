//! Preview tokens (CMS-R22): sharing unpublished content without
//! sharing an editor's credential.
//!
//! Every property here exists because pre-publication disclosure is
//! this service's signature harm:
//!
//! - The token is **scoped to one (variant, revision)**, so a share
//!   cannot follow the content forward into something nobody meant to
//!   send.
//! - It is **short-lived** (15 minutes by default, a day at most) and
//!   **revocable** immediately.
//! - Only its **hash** is stored; the token itself appears once, in the
//!   response that issued it, and never in a log or an audit row.
//! - Issue **and use** are audited — a share is a disclosure, and
//!   "who saw the embargoed page, and when" is what an incident review
//!   asks.
//! - Preview responses are `no-store`, and a previewed revision never
//!   reaches a sitemap (only published ones do).
//!
//! The refusal message is uniform across expiry, revocation, an unknown
//! token, and a wrong-revision token, so the endpoint cannot be used to
//! probe whether a guessed token ever existed.

use axum::extract::{Path as AxumPath, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response as AxumResponse};
use loco_rs::prelude::*;
use sea_orm::{QueryOrder, QuerySelect, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::unprocessable;
use crate::auth::MaybeAuthUser;
use crate::models::_entities::preview_tokens;
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
use crate::rules::preview;

/// `POST …/preview` body.
#[derive(Debug, Deserialize, Default)]
struct IssuePayload {
    /// Lifetime in seconds; clamped to [60, 86400].
    #[serde(default)]
    ttl_secs: Option<i64>,
    /// The revision to share; defaults to the variant's current one.
    #[serde(default)]
    revision_pid: Option<Uuid>,
}

/// The one response that carries the raw token.
#[derive(Debug, Serialize)]
struct IssuedView {
    pid: String,
    /// The token — **shown once**. Only its hash is stored.
    token: String,
    url: String,
    revision_pid: String,
    expires_at: chrono::DateTime<chrono::FixedOffset>,
    note: &'static str,
}

/// `POST /api/entries/{pid}/variants/{locale}/preview` — mint a share.
#[debug_handler]
async fn issue(
    State(ctx): State<AppContext>,
    Path((pid, locale)): Path<(String, String)>,
    caller: MaybeAuthUser,
    Json(payload): Json<IssuePayload>,
) -> Result<Response> {
    let entry = records::find_entry(&ctx.db, records::parse_pid(&pid)?).await?;
    let site = records::find_site(&ctx.db, entry.site_pid).await?;
    let variant = records::find_variant_by_locale(&ctx.db, entry.pid, &locale).await?;
    let revision_pid = payload
        .revision_pid
        .or(variant.current_revision_pid)
        .ok_or_else(|| unprocessable("this variant has no revision to preview"))?;
    let revision = records::find_revision(&ctx.db, revision_pid).await?;
    if revision.variant_pid != variant.pid {
        return Err(unprocessable(
            "that revision belongs to a different variant",
        ));
    }

    let token = preview::mint();
    let ttl = preview::clamp_ttl(payload.ttl_secs);
    let expires_at: chrono::DateTime<chrono::FixedOffset> =
        (chrono::Utc::now() + chrono::Duration::seconds(ttl)).into();

    let txn = ctx.db.begin().await?;
    let row = preview_tokens::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        token_hash: ActiveValue::set(preview::hash(&token)),
        site_pid: ActiveValue::set(site.pid),
        variant_pid: ActiveValue::set(variant.pid),
        revision_pid: ActiveValue::set(revision.pid),
        issued_by: ActiveValue::set(caller.actor().map(ToString::to_string)),
        expires_at: ActiveValue::set(expires_at),
        revoked_at: ActiveValue::set(None),
        used_count: ActiveValue::set(0),
        last_used_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    // The audit row records the share — never the token itself
    // (security invariant 9: no secret in logs or audit rows).
    Audit::record(
        &txn,
        "preview_token",
        row.pid,
        "preview_issued",
        caller.actor(),
        Some(serde_json::json!({
            "entry": entry.key,
            "locale": locale,
            "revision_pid": revision.pid,
            "revision_number": revision.number,
            "expires_at": expires_at,
            "ttl_secs": ttl,
        })),
    )
    .await?;
    txn.commit().await?;

    format::json(IssuedView {
        pid: row.pid.to_string(),
        url: format!("/delivery/{}/preview/{token}", site.key),
        token,
        revision_pid: revision.pid.to_string(),
        expires_at,
        note: "this token is shown once and stored only as a hash; it is scoped to this one \
               revision and expires",
    })
}

/// `GET /api/entries/{pid}/variants/{locale}/preview` — the live shares
/// for a variant, so an editor can see what is outstanding and withdraw
/// it. Never includes the tokens.
#[debug_handler]
async fn list(
    State(ctx): State<AppContext>,
    Path((pid, locale)): Path<(String, String)>,
) -> Result<Response> {
    let entry = records::find_entry(&ctx.db, records::parse_pid(&pid)?).await?;
    let variant = records::find_variant_by_locale(&ctx.db, entry.pid, &locale).await?;
    let rows = preview_tokens::Entity::find()
        .filter(preview_tokens::Column::VariantPid.eq(variant.pid))
        .order_by_desc(preview_tokens::Column::Id)
        .limit(200)
        .all(&ctx.db)
        .await?;
    let now = chrono::Utc::now();
    let shares: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "pid": row.pid,
                "revision_pid": row.revision_pid,
                "issued_by": row.issued_by,
                "issued_at": row.created_at,
                "expires_at": row.expires_at,
                "revoked_at": row.revoked_at,
                "used_count": row.used_count,
                "last_used_at": row.last_used_at,
                "live": row.revoked_at.is_none() && row.expires_at.to_utc() > now,
            })
        })
        .collect();
    format::json(serde_json::json!({
        "as_of": now,
        "shares": shares,
        "note": "tokens are never listed — only their hashes are stored",
    }))
}

/// `DELETE /api/preview-tokens/{pid}` — withdraw a share immediately.
#[debug_handler]
async fn revoke(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let target = records::parse_pid(&pid)?;
    let row = preview_tokens::Entity::find()
        .filter(preview_tokens::Column::Pid.eq(target))
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;
    let txn = ctx.db.begin().await?;
    let mut active: preview_tokens::ActiveModel = row.into();
    active.revoked_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    active.update(&txn).await?;
    Audit::record(
        &txn,
        "preview_token",
        target,
        "preview_revoked",
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::empty_json()
}

/// `GET /delivery/{site}/preview/{token}` — render the shared revision.
///
/// Deliberately **not** part of the public delivery allow-list logic:
/// this path carries its own credential, so it is checked here rather
/// than by site visibility. The response is `no-store` and the use is
/// audited.
#[debug_handler]
async fn render(
    State(ctx): State<AppContext>,
    AxumPath((site_key, token)): AxumPath<(String, String)>,
    _headers: HeaderMap,
) -> Result<AxumResponse> {
    let refuse = || -> AxumResponse {
        (
            StatusCode::NOT_FOUND,
            preview::Refusal::Unknown.public_message(),
        )
            .into_response()
    };
    let Ok(site) = records::find_site_by_key(&ctx.db, &site_key).await else {
        return Ok(refuse());
    };
    let stored = preview_tokens::Entity::find()
        .filter(preview_tokens::Column::TokenHash.eq(preview::hash(&token)))
        .filter(preview_tokens::Column::SitePid.eq(site.pid))
        .one(&ctx.db)
        .await?;
    let Some(stored) = stored else {
        return Ok(refuse());
    };
    let check = preview::check(
        Some(preview::Stored {
            expires_at: stored.expires_at,
            revoked: stored.revoked_at.is_some(),
            revision_pid: stored.revision_pid,
        }),
        stored.revision_pid,
        chrono::Utc::now().into(),
    );
    if let Err(refusal) = check {
        // The refusal is recorded (an expired link being tried is worth
        // knowing about) but the caller learns nothing beyond "no".
        let txn = ctx.db.begin().await?;
        Audit::record(
            &txn,
            "preview_token",
            stored.pid,
            "preview_refused",
            None,
            Some(serde_json::json!({ "refusal": refusal })),
        )
        .await?;
        txn.commit().await?;
        return Ok(refuse());
    }

    let revision = records::find_revision(&ctx.db, stored.revision_pid).await?;
    let variant = records::find_variant(&ctx.db, stored.variant_pid).await?;
    let entry = records::find_entry(&ctx.db, variant.entry_pid).await?;

    let txn = ctx.db.begin().await?;
    let token_pid = stored.pid;
    let used = stored.used_count.saturating_add(1);
    let mut active: preview_tokens::ActiveModel = stored.into();
    active.used_count = ActiveValue::set(used);
    active.last_used_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    active.update(&txn).await?;
    // Sensitive read: someone looked at unpublished content.
    Audit::record(
        &txn,
        "preview_token",
        token_pid,
        "preview_used",
        None,
        Some(serde_json::json!({
            "entry": entry.key,
            "locale": variant.locale,
            "revision_pid": revision.pid,
            "used_count": used,
        })),
    )
    .await?;
    txn.commit().await?;

    let payload = serde_json::json!({
        "as_of": chrono::Utc::now(),
        "preview": true,
        "site": site.key,
        "entry": { "pid": entry.pid, "key": entry.key, "content_type": entry.content_type_key },
        "locale": variant.locale,
        "status": variant.status,
        "revision": {
            "pid": revision.pid,
            "number": revision.number,
            "title": revision.title,
            "blocks": revision.blocks,
            "fields": revision.fields,
            "seo": revision.seo,
        },
        "is_published_revision": variant.published_revision_pid == Some(revision.pid),
        "note": "a preview renders one specific revision; it is not what delivery serves",
    });
    let mut response = axum::Json(payload).into_response();
    let headers = response.headers_mut();
    // Never cached, never indexed: this is unpublished content.
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    headers.insert(
        header::HeaderName::from_static("x-robots-tag"),
        HeaderValue::from_static("noindex, nofollow, noarchive"),
    );
    Ok(response)
}

/// The preview routes.
pub fn routes() -> Routes {
    Routes::new()
        .add(
            "/api/entries/{pid}/variants/{locale}/preview",
            post(issue).get(list),
        )
        .add("/api/preview-tokens/{pid}", delete(revoke))
        .add("/delivery/{site}/preview/{token}", get(render))
}
