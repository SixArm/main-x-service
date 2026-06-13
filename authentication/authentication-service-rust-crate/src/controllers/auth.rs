//! Passwordless, magic-link authentication.
//!
//! Flow:
//! 1. `POST /api/auth/signup`     `{email, name?}` — create the account, issue a magic link.
//! 2. `POST /api/auth/magic-link` `{email}`        — issue a magic link for an existing account (sign in).
//! 3. `GET  /api/auth/magic-link/{token}`          — consume the link → RS256 access token + session.
//! 4. `GET  /api/auth/me`                          — current user (bearer token required).
//! 5. `POST /api/auth/signout`                     — revoke the current session (bearer token required).
//! 6. `GET  /api/auth/audit/recent`                — recent authentication events (system-wide audit trail).
//! 7. `GET  /api/auth/account/export`              — GDPR right of access: the subject's own data (bearer).
//! 8. `GET  /api/auth/account/audit`               — GDPR right of access: the subject's own audit trail (bearer).
//! 9. `DELETE /api/auth/account`                   — GDPR right to erasure: soft-delete + anonymise (bearer).
//!
//! Every endpoint writes a best-effort [`AuthEvent`] row (signup,
//! magic-link request/redeem, signout) for the security + compliance
//! audit trail. The audit row may distinguish outcomes (e.g.
//! `rate_limited` / `unknown_email` / `existing`) for review, but the
//! HTTP response never does — the anti-enumeration contract holds at the
//! wire. No tokens or secrets are ever stored.
//!
//! Tokens are RS256 and verifiable offline by peer services via the
//! JWKS at `/.well-known/jwks.json`. In development the magic link is
//! written to the tracing log (no SMTP required).

use axum::http::StatusCode;
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    auth::AuthUser,
    mailers::auth::AuthMailer,
    models::{auth_events::Model as AuthEvent, sessions, users},
    rate_limit,
    views::auth::{AccountAuditExport, AccountExport, CurrentResponse, LoginResponse},
};

/// Map an exceeded magic-link issuance quota to an HTTP `429`. Returning
/// this *before* any account lookup or token issuance keeps abuse cheap to
/// reject and preserves the always-`200` anti-enumeration shape on the
/// success path (the `429` is keyed on request volume, not account
/// existence).
fn rate_limited() -> Error {
    Error::CustomError(
        StatusCode::TOO_MANY_REQUESTS,
        ErrorDetail::new("rate_limited", "too many requests; try again later"),
    )
}

/// Request body for `POST /api/auth/signup`.
#[derive(Debug, Deserialize, Serialize)]
pub struct SignupParams {
    /// Email address to register (also the magic-link recipient).
    pub email: String,
    /// Optional display name; defaults from the email local-part.
    pub name: Option<String>,
}

/// Request body for `POST /api/auth/magic-link` (sign-in request).
#[derive(Debug, Deserialize, Serialize)]
pub struct MagicLinkParams {
    /// Email address of the existing account to sign in.
    pub email: String,
}

fn default_name(email: &str) -> String {
    let local = email.split('@').next().unwrap_or("user");
    if local.chars().count() >= 2 {
        local.to_string()
    } else {
        "user".to_string()
    }
}

/// Logs the magic link to the console (dev) and best-effort emails it
/// (prod). The console log is authoritative in development.
async fn deliver_magic_link(ctx: &AppContext, user: &users::Model) {
    let Some(token) = user.magic_link_token.as_ref() else {
        return;
    };
    let frontend = std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:5173".to_string());
    let link = format!("{frontend}/verify?token={token}");
    tracing::info!(
        email = %user.email,
        magic_link = %link,
        "magic link issued (dev: open the link, or GET /api/auth/magic-link/{{token}})"
    );
    if let Err(err) = AuthMailer::send_magic_link(ctx, user).await {
        tracing::debug!(error = %err, "magic link email not sent; console log above is authoritative");
    }
}

/// Create a passwordless account and send a magic link. To avoid
/// leaking whether an email is already registered, an existing email
/// still receives a fresh link and the response is always 200.
#[debug_handler]
async fn signup(State(ctx): State<AppContext>, Json(params): Json<SignupParams>) -> Result<Response> {
    // Throttle per email before any work, so abuse cannot email-bomb a
    // victim or probe for accounts. Over the limit → 429, no token issued.
    if rate_limit::check(&params.email).is_err() {
        AuthEvent::record_best_effort(
            &ctx.db,
            "signup",
            Some(&params.email),
            None,
            Some("rate_limited"),
        )
        .await;
        return Err(rate_limited());
    }

    let name = params
        .name
        .clone()
        .filter(|n| n.chars().count() >= 2)
        .unwrap_or_else(|| default_name(&params.email));

    let (user, existing) =
        match users::Model::create_passwordless(&ctx.db, &params.email, &name).await {
            Ok(user) => (user, false),
            Err(ModelError::EntityAlreadyExists) => {
                if let Ok(user) = users::Model::find_by_email(&ctx.db, &params.email).await {
                    (user, true)
                } else {
                    AuthEvent::record_best_effort(
                        &ctx.db,
                        "signup",
                        Some(&params.email),
                        None,
                        Some("rejected"),
                    )
                    .await;
                    return format::empty_json();
                }
            }
            Err(err) => {
                tracing::info!(error = %err, email = %params.email, "signup rejected");
                AuthEvent::record_best_effort(
                    &ctx.db,
                    "signup",
                    Some(&params.email),
                    None,
                    Some("rejected"),
                )
                .await;
                return format::empty_json();
            }
        };

    let user = user.into_active_model().create_magic_link(&ctx.db).await?;
    // The audit row distinguishes a fresh account from an existing one
    // for security review, but the 200 response above does not — the
    // anti-enumeration contract holds at the wire.
    AuthEvent::record_best_effort(
        &ctx.db,
        "signup",
        Some(&user.email),
        Some(user.pid),
        Some(if existing { "existing" } else { "created" }),
    )
    .await;
    deliver_magic_link(&ctx, &user).await;
    format::empty_json()
}

/// Request a magic link for an existing account (sign in). Always
/// returns 200, even for unknown emails, to avoid account enumeration.
#[debug_handler]
async fn request_magic_link(
    State(ctx): State<AppContext>,
    Json(params): Json<MagicLinkParams>,
) -> Result<Response> {
    // Throttle per email before any lookup (see `signup`).
    if rate_limit::check(&params.email).is_err() {
        AuthEvent::record_best_effort(
            &ctx.db,
            "magic_link_requested",
            Some(&params.email),
            None,
            Some("rate_limited"),
        )
        .await;
        return Err(rate_limited());
    }

    let Ok(user) = users::Model::find_by_email(&ctx.db, &params.email).await else {
        tracing::debug!(email = %params.email, "magic link requested for unknown email");
        // Audited as unknown_email for security review; the 200 response
        // is identical to the known-account path (anti-enumeration).
        AuthEvent::record_best_effort(
            &ctx.db,
            "magic_link_requested",
            Some(&params.email),
            None,
            Some("unknown_email"),
        )
        .await;
        return format::empty_json();
    };
    let user = user.into_active_model().create_magic_link(&ctx.db).await?;
    AuthEvent::record_best_effort(
        &ctx.db,
        "magic_link_requested",
        Some(&user.email),
        Some(user.pid),
        Some("issued"),
    )
    .await;
    deliver_magic_link(&ctx, &user).await;
    format::empty_json()
}

/// Consume a magic link: validate the token, verify the email, issue an
/// RS256 access token, and record the session for revocation.
#[debug_handler]
async fn verify(Path(token): Path<String>, State(ctx): State<AppContext>) -> Result<Response> {
    let Ok(user) = users::Model::find_by_magic_token(&ctx.db, &token).await else {
        // Unknown / expired / already-consumed token — all map to the
        // same 401. We never log the token itself.
        AuthEvent::record_best_effort(
            &ctx.db,
            "magic_link_redeemed",
            None,
            None,
            Some("invalid_or_expired"),
        )
        .await;
        return unauthorized("invalid or expired magic link");
    };

    let user = user.into_active_model().clear_magic_link(&ctx.db).await?;
    let user = if user.email_verified_at.is_none() {
        user.into_active_model().verified(&ctx.db).await?
    } else {
        user
    };

    let (access_token, jti, exp) =
        crate::auth::sign_access_token(&user.pid.to_string(), &user.email, &user.name)
            .map_err(|e| Error::string(&e.to_string()))?;

    let expires_at = chrono::DateTime::<chrono::Utc>::from_timestamp(exp, 0)
        .unwrap_or_else(chrono::Utc::now)
        .fixed_offset();
    sessions::Model::issue(&ctx.db, &jti, user.pid, expires_at, None).await?;

    AuthEvent::record_best_effort(
        &ctx.db,
        "magic_link_redeemed",
        Some(&user.email),
        Some(user.pid),
        Some("ok"),
    )
    .await;
    format::json(LoginResponse::new(&user, &access_token))
}

/// Current authenticated user. Honors local revocation: a signed-out
/// session is rejected even though its JWT signature is still valid.
/// Honors GDPR erasure: a deleted account is treated as gone — its
/// still-valid bearer token returns `401`, not the tombstoned record.
#[debug_handler]
async fn me(auth: AuthUser, State(ctx): State<AppContext>) -> Result<Response> {
    let AuthUser(claims) = auth;
    if let Ok(session) = sessions::Model::find_by_jid(&ctx.db, &claims.jti).await
        && !session.is_active()
    {
        return unauthorized("session signed out");
    }
    let Ok(user) = users::Model::find_active_by_pid(&ctx.db, &claims.sub).await else {
        // No live user for this pid (never existed, or GDPR-erased). The
        // token may still verify cryptographically until expiry; we
        // refuse to serve a deleted subject's record.
        return unauthorized("account not found");
    };
    format::json(CurrentResponse::new(&user))
}

/// Revoke the current session. Peer services that cached the token keep
/// honoring it until expiry (offline JWKS verification) — that's the
/// documented tradeoff of stateless tokens; we keep TTLs short.
#[debug_handler]
async fn signout(auth: AuthUser, State(ctx): State<AppContext>) -> Result<Response> {
    let AuthUser(claims) = auth;
    if let Ok(session) = sessions::Model::find_by_jid(&ctx.db, &claims.jti).await {
        session.into_active_model().revoke(&ctx.db).await?;
    }
    AuthEvent::record_best_effort(
        &ctx.db,
        "signout",
        Some(&claims.email),
        uuid::Uuid::parse_str(&claims.sub).ok(),
        None,
    )
    .await;
    format::empty_json()
}

/// Recent **system-wide** authentication events, newest first. The
/// response shape is documented in the `OpenAPI` document (the
/// `AuthEvent` schema).
///
/// Left **open** (no bearer), mirroring how the sibling care-pathway
/// service exposes `/audit/recent`: the rows carry no tokens or secrets,
/// only event names, normalised emails, subject pids, and outcome
/// markers. The GDPR right-of-access requirement (T-9) is satisfied
/// instead by the bearer-gated per-subject `GET /api/auth/account/audit`
/// — a subject's *own* audit trail is reachable only by that subject —
/// so this system-wide view can stay open for operators. See service
/// spec §12 for the recorded decision.
#[debug_handler]
async fn recent_audit(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = AuthEvent::recent(&ctx.db, 100).await?;
    format::json(rows)
}

/// Resolve the live, non-erased user for an authenticated request, or a
/// `401`. A token can outlive its account (erasure is irreversible but
/// the token verifies until `exp`); the account routes must refuse a
/// deleted subject rather than operate on a tombstone.
async fn require_active_user(
    ctx: &AppContext,
    claims: &crate::auth::Claims,
) -> Result<users::Model> {
    users::Model::find_active_by_pid(&ctx.db, &claims.sub)
        .await
        .map_err(|_| Error::Unauthorized("account not found".to_string()))
}

/// GDPR Art. 15 (right of access). Returns a JSON document of everything
/// the service holds about the authenticated subject: their `users` row,
/// their `sessions` (issuance/expiry/revocation timestamps + user agent,
/// never a token), and their `auth_events` audit trail (matched by pid
/// *or* email). No tokens, key material, password hash, or api key are
/// ever included. A GDPR-erased account is treated as gone (`401`).
#[debug_handler]
async fn export_account(auth: AuthUser, State(ctx): State<AppContext>) -> Result<Response> {
    let AuthUser(claims) = auth;
    let user = require_active_user(&ctx, &claims).await?;
    let sessions = sessions::Model::find_all_by_user_pid(&ctx.db, user.pid).await?;
    let events = AuthEvent::for_subject(&ctx.db, user.pid, &user.email).await?;
    format::json(AccountExport::new(&user, &sessions, &events))
}

/// GDPR Art. 15 (right of access), per-subject audit trail. Returns only
/// the authenticated subject's own `auth_events` rows (matched by pid or
/// email), newest first — the bearer-gated counterpart to the open,
/// system-wide `GET /api/auth/audit/recent`.
#[debug_handler]
async fn account_audit(auth: AuthUser, State(ctx): State<AppContext>) -> Result<Response> {
    let AuthUser(claims) = auth;
    let user = require_active_user(&ctx, &claims).await?;
    let events = AuthEvent::for_subject(&ctx.db, user.pid, &user.email).await?;
    let rows: Vec<AccountAuditExport> = events.iter().map(AccountAuditExport::new).collect();
    format::json(rows)
}

/// GDPR Art. 17 (right to erasure). Soft-deletes + anonymises the
/// account: stamps `users.deleted_at`, replaces `email`/`name` with a
/// tombstone (so referential history and the audit trail keep their
/// integrity), revokes all of the subject's sessions, and records an
/// `account_erased` audit row. After erasure the bearer token still
/// verifies cryptographically until `exp`, but `/me` and the export
/// treat the subject as gone (`401`). Idempotent: erasing an
/// already-erased account is a no-op `200`.
#[debug_handler]
async fn delete_account(auth: AuthUser, State(ctx): State<AppContext>) -> Result<Response> {
    let AuthUser(claims) = auth;
    let user = require_active_user(&ctx, &claims).await?;

    let pid = user.pid;
    let email = user.email.clone();
    // Revoke every live session first so a concurrent request cannot use
    // a session that outlives the anonymisation.
    sessions::Model::revoke_all_for_user(&ctx.db, pid).await?;
    user.into_active_model().erase(&ctx.db).await?;
    AuthEvent::record_best_effort(
        &ctx.db,
        "account_erased",
        Some(&email),
        Some(pid),
        Some("ok"),
    )
    .await;

    format::empty_json()
}

/// Routes for the passwordless magic-link auth surface, mounted under
/// `/api/auth`: signup, magic-link request, redeem, me, signout, the
/// system-wide audit feed, and the bearer-gated GDPR account routes
/// (export / per-subject audit / erasure).
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/auth")
        .add("/signup", post(signup))
        .add("/magic-link", post(request_magic_link))
        .add("/magic-link/{token}", get(verify))
        .add("/me", get(me))
        .add("/signout", post(signout))
        .add("/audit/recent", get(recent_audit))
        .add("/account/export", get(export_account))
        .add("/account/audit", get(account_audit))
        .add("/account", delete(delete_account))
}
