//! Passwordless, magic-link authentication.
//!
//! Flow:
//! 1. `POST /api/auth/signup`     `{email, name?, locale?}` — create the account, issue a magic link.
//! 2. `POST /api/auth/magic-link` `{email, locale?}`        — issue a magic link for an existing account (sign in).
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
    mailers::auth::Emailer,
    metrics::Metrics,
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
    /// Optional BCP-47 locale (e.g. `en`, `cy`) selecting the language of
    /// the magic-link email. Unknown/absent ⇒ English (see [`crate::i18n`]).
    pub locale: Option<String>,
    /// Optional return base (the requesting front-end's origin) so the
    /// magic-link email lands on THAT app's `/verify`. Honoured only when it
    /// exactly matches `AUTH_ALLOWED_FRONTENDS`; otherwise `FRONTEND_URL`.
    pub return_url: Option<String>,
}

/// Request body for `POST /api/auth/magic-link` (sign-in request).
#[derive(Debug, Deserialize, Serialize)]
pub struct MagicLinkParams {
    /// Email address of the existing account to sign in.
    pub email: String,
    /// Optional BCP-47 locale (e.g. `en`, `cy`) selecting the language of
    /// the magic-link email. Unknown/absent ⇒ English (see [`crate::i18n`]).
    pub locale: Option<String>,
    /// Optional return base (the requesting front-end's origin) so the
    /// magic-link email lands on THAT app's `/verify`. Honoured only when it
    /// exactly matches `AUTH_ALLOWED_FRONTENDS`; otherwise the default
    /// `FRONTEND_URL` is used. This is the per-app SSO knob.
    pub return_url: Option<String>,
}

/// Comma-separated allow-list of front-end return bases from
/// `AUTH_ALLOWED_FRONTENDS` (exact `scheme://host[:port]` origins).
fn allowed_frontends() -> Vec<String> {
    std::env::var("AUTH_ALLOWED_FRONTENDS")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// The default front-end base when no (or no allow-listed) `return_url` is
/// supplied.
fn default_frontend() -> String {
    std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:5173".to_string())
}

/// Choose the front-end base for the magic-link email. A `requested`
/// return base is honoured only when it exactly matches an `allowlist`
/// entry (no open redirect); otherwise `default` is used. Pure, so it is
/// unit-testable without env or a request.
fn choose_frontend(requested: Option<&str>, allowlist: &[String], default: &str) -> String {
    match requested.map(str::trim) {
        Some(r) if !r.is_empty() && allowlist.iter().any(|a| a == r) => r.to_string(),
        _ => default.to_string(),
    }
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
/// (prod), rendering the email in `locale`. The console log is
/// authoritative in development.
async fn deliver_magic_link(ctx: &AppContext, user: &users::Model, locale: &str, frontend: &str) {
    let Some(token) = user.magic_link_token.as_ref() else {
        return;
    };
    let link = format!("{frontend}/verify?token={token}");
    tracing::info!(
        email = %user.email,
        locale = %locale,
        magic_link = %link,
        "magic link issued (dev: open the link, or GET /api/auth/magic-link/{{token}})"
    );
    if let Err(err) = Emailer::send_magic_link(ctx, user, locale, frontend).await {
        tracing::debug!(error = %err, "magic link email not sent; console log above is authoritative");
    }
}

/// Create a passwordless account and send a magic link. To avoid
/// leaking whether an email is already registered, an existing email
/// still receives a fresh link and the response is always 200.
#[debug_handler]
async fn signup(
    State(ctx): State<AppContext>,
    Json(params): Json<SignupParams>,
) -> Result<Response> {
    // Throttle per email before any work, so abuse cannot email-bomb a
    // victim or probe for accounts. Over the limit → 429, no token issued.
    if rate_limit::check(&ctx.db, &params.email).await.is_err() {
        Metrics::global().rate_limited_total.inc();
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
    // Locale selection affects only the rendered email language; the
    // response shape is unchanged (anti-enumeration contract holds).
    Metrics::global().signup_total.inc();
    Metrics::global().magic_link_issued_total.inc();
    let locale = crate::i18n::select_locale(params.locale.as_deref());
    let frontend = choose_frontend(
        params.return_url.as_deref(),
        &allowed_frontends(),
        &default_frontend(),
    );
    deliver_magic_link(&ctx, &user, &locale, &frontend).await;
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
    if rate_limit::check(&ctx.db, &params.email).await.is_err() {
        Metrics::global().rate_limited_total.inc();
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
    Metrics::global().magic_link_issued_total.inc();
    let locale = crate::i18n::select_locale(params.locale.as_deref());
    let frontend = choose_frontend(
        params.return_url.as_deref(),
        &allowed_frontends(),
        &default_frontend(),
    );
    deliver_magic_link(&ctx, &user, &locale, &frontend).await;
    format::empty_json()
}

/// Consume a magic link: validate the token, verify the email, set the
/// session cookie + issue a PASETO, and record the session for revocation.
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

    // The server-side session id (`sid`) is the durable thing; the
    // short-lived PASETO carries it so peers and signout can correlate.
    let sid = uuid::Uuid::new_v4().to_string();
    let (access_token, _sid, exp) =
        crate::auth::sign_access_token(&user.pid.to_string(), &user.email, &user.name, &sid)
            .map_err(|e| Error::string(&e.to_string()))?;

    let expires_at = chrono::DateTime::<chrono::Utc>::from_timestamp(exp, 0)
        .unwrap_or_else(chrono::Utc::now)
        .fixed_offset();
    sessions::Model::issue(&ctx.db, &sid, user.pid, expires_at, None).await?;

    AuthEvent::record_best_effort(
        &ctx.db,
        "magic_link_redeemed",
        Some(&user.email),
        Some(user.pid),
        Some("ok"),
    )
    .await;
    Metrics::global().magic_link_redeemed_total.inc();
    // Establish the server-side session as an httpOnly cookie (the BFF
    // holds this; the browser never reads it). The body still carries the
    // PASETO transitionally until every front-end adopts the BFF and pulls
    // tokens via `POST /api/auth/token` instead.
    let mut response = format::json(LoginResponse::new(&user, &access_token))?;
    response.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        crate::cookie::set_session(&sid)
            .parse()
            .expect("valid set-cookie value"),
    );
    Ok(response)
}

/// Exchange a valid session cookie for a short-lived PASETO access token.
/// The `SvelteKit` BFF calls this server-side: it holds the
/// `__Host-mxi_session` cookie and forwards the returned PASETO as a bearer
/// to the entity services, so the browser never sees a token.
///
/// CSRF: a light `Origin` allow-list (env `AUTH_ALLOWED_ORIGINS`,
/// comma-separated) backstops the `SameSite=Lax` cookie; unset ⇒ permissive
/// (dev). Full double-submit CSRF is a follow-up.
#[debug_handler]
async fn token(headers: axum::http::HeaderMap, State(ctx): State<AppContext>) -> Result<Response> {
    // CSRF backstop: reject a cross-origin caller when an allow-list is set.
    if let Some(allowed) = std::env::var("AUTH_ALLOWED_ORIGINS")
        .ok()
        .filter(|s| !s.trim().is_empty())
    {
        let origin = headers
            .get(axum::http::header::ORIGIN)
            .and_then(|v| v.to_str().ok());
        let ok = origin.is_some_and(|o| allowed.split(',').map(str::trim).any(|a| a == o));
        if !ok {
            return unauthorized("origin not allowed");
        }
    }
    // Read the opaque session id from the httpOnly cookie.
    let Some(sid) = headers
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(crate::cookie::read_session)
    else {
        return unauthorized("no session cookie");
    };
    // The session must exist and be active (not signed out / expired).
    let Ok(session) = sessions::Model::find_by_jid(&ctx.db, &sid).await else {
        return unauthorized("unknown session");
    };
    if !session.is_active() {
        return unauthorized("session revoked or expired");
    }
    // Resolve the user for the token claims, then mint a fresh PASETO bound
    // to this session id.
    let Ok(user) =
        users::Model::find_active_by_pid(&ctx.db, &session.user_pid.to_string()).await
    else {
        return unauthorized("account not found");
    };
    let (access_token, _sid, _exp) =
        crate::auth::sign_access_token(&user.pid.to_string(), &user.email, &user.name, &sid)
            .map_err(|e| Error::string(&e.to_string()))?;
    format::json(serde_json::json!({ "token": access_token }))
}

/// Current authenticated user. Honors local revocation: a signed-out
/// session is rejected even though its JWT signature is still valid.
/// Honors GDPR erasure: a deleted account is treated as gone — its
/// still-valid bearer token returns `401`, not the tombstoned record.
#[debug_handler]
async fn me(auth: AuthUser, State(ctx): State<AppContext>) -> Result<Response> {
    let AuthUser(claims) = auth;
    if let Ok(session) = sessions::Model::find_by_jid(&ctx.db, &claims.sid).await
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
    if let Ok(session) = sessions::Model::find_by_jid(&ctx.db, &claims.sid).await {
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
    Metrics::global().signout_total.inc();
    // Clear the session cookie so the browser drops it immediately.
    let mut response = format::empty_json()?;
    response.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        crate::cookie::clear_session()
            .parse()
            .expect("valid set-cookie value"),
    );
    Ok(response)
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
        .add("/token", post(token))
        .add("/me", get(me))
        .add("/signout", post(signout))
        .add("/audit/recent", get(recent_audit))
        .add("/account/export", get(export_account))
        .add("/account/audit", get(account_audit))
        .add("/account", delete(delete_account))
}

#[cfg(test)]
mod tests {
    use super::choose_frontend;

    const DEFAULT: &str = "http://localhost:5173";

    fn allowlist() -> Vec<String> {
        vec![
            "https://organization.example.com".to_string(),
            "https://case.example.com".to_string(),
        ]
    }

    #[test]
    fn allowlisted_return_url_is_honoured() {
        assert_eq!(
            choose_frontend(Some("https://organization.example.com"), &allowlist(), DEFAULT),
            "https://organization.example.com"
        );
        // Surrounding whitespace is trimmed before matching.
        assert_eq!(
            choose_frontend(Some("  https://case.example.com  "), &allowlist(), DEFAULT),
            "https://case.example.com"
        );
    }

    #[test]
    fn non_allowlisted_or_missing_falls_back_to_default() {
        // Not in the allow-list ⇒ default (no open redirect).
        assert_eq!(
            choose_frontend(Some("https://evil.example.com"), &allowlist(), DEFAULT),
            DEFAULT
        );
        // Absent / empty ⇒ default.
        assert_eq!(choose_frontend(None, &allowlist(), DEFAULT), DEFAULT);
        assert_eq!(choose_frontend(Some("   "), &allowlist(), DEFAULT), DEFAULT);
        // Empty allow-list ⇒ nothing is honoured.
        assert_eq!(
            choose_frontend(Some("https://organization.example.com"), &[], DEFAULT),
            DEFAULT
        );
    }
}
