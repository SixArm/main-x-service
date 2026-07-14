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

use loco_rs::environment::Environment;

use crate::{
    auth::{AuthUser, Claims},
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
    // SEC-A3: the magic-link URL embeds the live login token — a ~5-minute
    // account-takeover primitive if it reaches logs. Emit the token ONLY in
    // development (no SMTP there, so the console log is the authoritative
    // way to open the link); in every other environment log the issuance
    // without the token/URL. The email path still delivers it to the user.
    if log_magic_link_url(&ctx.environment) {
        let link = format!("{frontend}/verify?token={token}");
        tracing::info!(
            email = %user.email,
            locale = %locale,
            magic_link = %link,
            "magic link issued (dev: open the link, or GET /api/auth/magic-link/{{token}})"
        );
    } else {
        tracing::info!(email = %user.email, locale = %locale, "magic link issued");
    }
    if let Err(err) = Emailer::send_magic_link(ctx, user, locale, frontend).await {
        tracing::debug!(error = %err, "magic link email not sent");
    }
}

/// Whether the magic-link **token/URL** may be written to the log — only in
/// the `development` environment (SEC-A3). Pure, so the gate is unit-tested.
fn log_magic_link_url(env: &Environment) -> bool {
    matches!(env, Environment::Development)
}

/// SEC-A5 — perform one Argon2 password hash and discard it.
///
/// `signup`'s **new-account** path pays for exactly one deliberately-slow
/// Argon2 hash (inside `create_passwordless`, to fill the unusable
/// `password` column). The **already-registered** path skips that work, so
/// it returns measurably faster — a **timing oracle for account
/// enumeration** despite the identical always-`200` response. Running one
/// equivalent hash on the existing-email path keeps signup latency
/// indistinguishable between a new and an existing email. Returns the hash
/// so the work is observable to a test; the caller discards it.
#[must_use]
fn constant_work_hash() -> String {
    loco_rs::hash::hash_password(&uuid::Uuid::new_v4().to_string()).unwrap_or_default()
}

/// SEC-A10 — the CSRF decision for `POST /token` once the session is loaded.
/// Pure, so the full matrix is unit-tested.
///
/// - A session that carries a CSRF **synchroniser token** must echo it in
///   `X-CSRF-Token` (constant-time compared) — the primary defence.
/// - A **legacy** session (predating CSRF, no stored token) cannot do the
///   double-submit check, so it must instead prove **same-origin** (an
///   `Origin` on the `AUTH_ALLOWED_ORIGINS` allow-list, `origin_ok`). Without
///   that proof it is trusted only in development; in production it is
///   refused, so a legacy session can no longer bypass **both** the CSRF and
///   the origin checks.
fn csrf_token_gate(
    is_production: bool,
    origin_ok: bool,
    session_csrf: Option<&str>,
    provided_csrf: &str,
) -> std::result::Result<(), (StatusCode, &'static str)> {
    if let Some(expected) = session_csrf {
        if crate::csrf::matches(expected, provided_csrf) {
            Ok(())
        } else {
            Err((StatusCode::FORBIDDEN, "missing or invalid CSRF token"))
        }
    } else if origin_ok || !is_production {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            "CSRF token required (legacy session; set AUTH_ALLOWED_ORIGINS)",
        ))
    }
}

/// SEC-A10 — warn (once) that the `Origin` CSRF backstop is off because
/// `AUTH_ALLOWED_ORIGINS` is unset in production. A production deployment
/// should set it so cross-origin `POST /token` callers are rejected even
/// with `SameSite` cookies.
fn warn_missing_allowed_origins() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        tracing::warn!(
            "AUTH_ALLOWED_ORIGINS is unset in production: the Origin CSRF backstop \
             on POST /api/auth/token is disabled. Set it to your front-end origins."
        );
    });
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
                // SEC-A5: the new-account path just paid for one Argon2 hash
                // in `create_passwordless`; run an equivalent hash here so
                // the already-registered branch is not measurably faster (a
                // timing oracle for account enumeration).
                let _ = constant_work_hash();
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
    // SEC-A4: atomically consume the token — the clear-and-return is a single
    // UPDATE, so two concurrent redemptions of the same link cannot both
    // succeed (only one gets a row; the loser gets `EntityNotFound` → 401).
    let Ok(user) = users::Model::consume_magic_token(&ctx.db, &token).await else {
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

    // The token is already cleared by `consume_magic_token`.
    let user = if user.email_verified_at.is_none() {
        user.into_active_model().verified(&ctx.db).await?
    } else {
        user
    };

    // The server-side session id (`sid`) is the durable thing; the
    // short-lived PASETO carries it so peers and signout can correlate.
    let sid = uuid::Uuid::new_v4().to_string();
    // Per-session CSRF synchroniser token: stored in the session and
    // delivered to the client in the readable `__Host-mxi_csrf` cookie,
    // echoed back in `X-CSRF-Token` on mutating cookie-authed requests.
    let csrf_token = crate::csrf::generate_token();
    let (access_token, _sid, _exp) = crate::auth::sign_access_token(
        &user.pid.to_string(),
        &user.email,
        &user.name,
        &sid,
        user.attrs(),
    )
    .map_err(|e| Error::string(&e.to_string()))?;

    // Session establishment copies the user's ABAC attributes into the
    // session payload (shared authorization-attributes.md §6), so token
    // minting reads them from the session, not the users row. The session
    // gets its own idle/absolute TTLs (independent of the ~5-min token
    // exp) — see `sessions::Model::issue`.
    sessions::Model::issue(
        &ctx.db,
        &sid,
        user.pid,
        None,
        sessions::session_data(&user.attributes, &csrf_token),
    )
    .await?;

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
    // Second Set-Cookie: the readable CSRF token (append, not insert, so
    // it does not overwrite the session cookie).
    response.headers_mut().append(
        axum::http::header::SET_COOKIE,
        crate::csrf::set_csrf(&csrf_token)
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
/// CSRF (SEC-A10): a session carrying a synchroniser token must echo it in
/// `X-CSRF-Token` (`csrf_token_gate`, the primary defence). An `Origin`
/// allow-list (env `AUTH_ALLOWED_ORIGINS`, comma-separated) backstops the
/// `SameSite=Lax` cookie and is the sole proof a legacy (token-less) session
/// can offer — such a session is refused in production without it, so it
/// cannot bypass both the CSRF and the origin checks. Unset allow-list stays
/// permissive in development and warns once in production.
#[debug_handler]
async fn token(headers: axum::http::HeaderMap, State(ctx): State<AppContext>) -> Result<Response> {
    // SEC-A10: compute the origin decision once. `origin_ok` = an `Origin`
    // header present on the `AUTH_ALLOWED_ORIGINS` allow-list; `false` when
    // no allow-list is configured (an origin can't be *proven*).
    let allowed_origins = std::env::var("AUTH_ALLOWED_ORIGINS")
        .ok()
        .filter(|s| !s.trim().is_empty());
    let request_origin = headers
        .get(axum::http::header::ORIGIN)
        .and_then(|v| v.to_str().ok());
    let origin_ok = allowed_origins.as_deref().is_some_and(|list| {
        request_origin.is_some_and(|o| list.split(',').map(str::trim).any(|a| a == o))
    });
    let is_production = matches!(ctx.environment, Environment::Production);

    // Backstop: when an allow-list is set, reject a disallowed origin outright.
    if allowed_origins.is_some() && !origin_ok {
        return unauthorized("origin not allowed");
    }
    // Warn (once) if the backstop is off in production.
    if is_production && allowed_origins.is_none() {
        warn_missing_allowed_origins();
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
    // SEC-A10: this is a cookie-authenticated mutating request. A session
    // that carries a CSRF synchroniser token must echo it in `X-CSRF-Token`;
    // a legacy session with no stored token must instead prove same-origin
    // (and is refused in production without an allow-list) so it cannot
    // bypass both the CSRF and the origin checks. A failure is `403`,
    // distinct from the `401`s above.
    let provided_raw = headers
        .get(crate::csrf::CSRF_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    // SEC-A9: the session stores only the *hash* of its CSRF token, so
    // compare against the hash of the presented header. An absent/empty
    // header stays empty (never hashed) so it can never match a real token.
    let provided = if provided_raw.is_empty() {
        String::new()
    } else {
        crate::secret_hash::hash(provided_raw)
    };
    if let Err((status, reason)) =
        csrf_token_gate(is_production, origin_ok, session.csrf(), &provided)
    {
        return Err(Error::CustomError(status, ErrorDetail::new("csrf", reason)));
    }
    // Resolve the user for the token claims, then mint a fresh PASETO bound
    // to this session id. The ABAC `attrs` claim comes from the session's
    // copied attributes (shared authorization-attributes.md §6) — the
    // users read is only for the identity claims + erasure check.
    let Ok(user) = users::Model::find_active_by_pid(&ctx.db, &session.user_pid.to_string()).await
    else {
        return unauthorized("account not found");
    };
    let (access_token, _sid, _exp) = crate::auth::sign_access_token(
        &user.pid.to_string(),
        &user.email,
        &user.name,
        &sid,
        session.attrs(),
    )
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
    if let Ok(session) = sessions::Model::find_by_jid(&ctx.db, &claims.sid).await {
        if !session.is_active() {
            return unauthorized("session signed out");
        }
        // Slide the idle window on use (best-effort — a touch failure
        // must not break the read).
        if let Err(err) = session.touch(&ctx.db).await {
            tracing::warn!(error = %err, "failed to slide session idle window");
        }
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
    // Clear the session and CSRF cookies so the browser drops them.
    let mut response = format::empty_json()?;
    response.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        crate::cookie::clear_session()
            .parse()
            .expect("valid set-cookie value"),
    );
    response.headers_mut().append(
        axum::http::header::SET_COOKIE,
        crate::csrf::clear_csrf()
            .parse()
            .expect("valid set-cookie value"),
    );
    Ok(response)
}

/// Recent **system-wide** authentication events, newest first. The
/// response shape is documented in the `OpenAPI` document (the
/// `AuthEvent` schema).
///
/// **Admin-gated** (SEC-A2). The rows carry registered **emails** plus
/// outcome markers (`created` vs `existing`, `unknown_email` vs `issued`,
/// `rate_limited`). Left open, this system-wide trail is an
/// account-enumeration oracle — an attacker triggers a signup for a target
/// email and reads back the outcome by timing — which would undo the
/// always-`200` anti-enumeration contract the unauthenticated endpoints
/// carefully preserve. It now requires a valid PASETO bearer whose
/// attributes include `access=admin` (`401` without a token, `403` for a
/// non-admin). A subject's *own* trail stays reachable via the
/// session-gated `GET /api/auth/account/audit` (GDPR right-of-access, T-9).
#[debug_handler]
async fn recent_audit(auth: AuthUser, State(ctx): State<AppContext>) -> Result<Response> {
    let AuthUser(claims) = auth;
    if !claims_have_admin(&claims) {
        return Err(Error::CustomError(
            StatusCode::FORBIDDEN,
            ErrorDetail::new(
                "forbidden",
                "admin attribute required (access=admin) to read the system-wide audit trail",
            ),
        ));
    }
    let rows = AuthEvent::recent(&ctx.db, 100).await?;
    format::json(rows)
}

/// Whether `claims` carry the `access=admin` attribute. Pure, unit-tested;
/// mirrors the admin controller's gate (SEC-A2).
fn claims_have_admin(claims: &Claims) -> bool {
    claims
        .attrs
        .get("access")
        .is_some_and(|values| values.iter().any(|v| v == "admin"))
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
    // SEC-A7: erasing the `users` row is not enough — the subject's email
    // also survives in the audit trail (`auth_events.email`, incl. pre-account
    // `unknown_email` rows) and in `sessions.user_agent`. Scrub both so the
    // GDPR erasure is complete.
    AuthEvent::scrub_subject_email(&ctx.db, pid, &email).await?;
    sessions::Model::scrub_user_agent_for_user(&ctx.db, pid).await?;
    // The final `account_erased` audit row carries only the pid — writing the
    // email here would re-introduce the address we just scrubbed.
    AuthEvent::record_best_effort(&ctx.db, "account_erased", None, Some(pid), Some("ok")).await;

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
    use super::{
        choose_frontend, claims_have_admin, constant_work_hash, csrf_token_gate,
        log_magic_link_url,
    };
    use axum::http::StatusCode;
    use crate::auth::Claims;
    use loco_rs::environment::Environment;
    use std::collections::BTreeMap;

    /// SEC-A10: the `POST /token` CSRF gate. A session with a synchroniser
    /// token must echo it; a legacy (token-less) session must prove
    /// same-origin, and — critically — **cannot bypass both** the CSRF and
    /// the origin checks in production.
    #[test]
    fn csrf_gate_matrix() {
        // A token-carrying session: correct token allows, wrong/absent 403.
        assert!(csrf_token_gate(true, false, Some("tok"), "tok").is_ok());
        assert_eq!(
            csrf_token_gate(true, false, Some("tok"), "wrong").unwrap_err().0,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            csrf_token_gate(true, true, Some("tok"), "").unwrap_err().0,
            StatusCode::FORBIDDEN,
            "a matching origin does not excuse a bad CSRF token"
        );

        // Legacy (token-less) session — the SEC-A10 bypass being closed:
        // production + no proven origin ⇒ rejected.
        assert_eq!(
            csrf_token_gate(true, false, None, "").unwrap_err().0,
            StatusCode::FORBIDDEN,
            "legacy session with no CSRF token and no allowed origin must be refused in production"
        );
        // Legacy session with a proven same-origin ⇒ allowed.
        assert!(csrf_token_gate(true, true, None, "").is_ok());
        // Development stays permissive for a legacy session.
        assert!(csrf_token_gate(false, false, None, "").is_ok());
    }

    /// SEC-A5: the constant-work hash performs a **real** Argon2 hash (a
    /// `$argon2` PHC string), so the already-registered signup branch pays
    /// the same deliberately-slow cost as the new-account path and cannot be
    /// distinguished by latency. Two calls hash distinct random inputs, so
    /// (with Argon2's random salt) they never collide — proving actual work
    /// ran, not a cached/constant return.
    #[test]
    fn constant_work_hash_performs_a_real_argon2_hash() {
        let h = constant_work_hash();
        assert!(
            h.starts_with("$argon2"),
            "expected an argon2 PHC hash, got {h:?}"
        );
        assert_ne!(
            constant_work_hash(),
            h,
            "each call must perform a fresh Argon2 hash"
        );
    }

    const DEFAULT: &str = "http://localhost:5173";

    fn claims_with_access(values: &[&str]) -> Claims {
        let mut attrs = BTreeMap::new();
        if !values.is_empty() {
            attrs.insert(
                "access".to_string(),
                values.iter().map(|v| (*v).to_string()).collect(),
            );
        }
        Claims {
            sub: "11111111-1111-1111-1111-111111111111".to_string(),
            email: "a@example.com".to_string(),
            name: "A".to_string(),
            iss: "authentication-service".to_string(),
            aud: "main-x-service".to_string(),
            exp: 0,
            iat: 0,
            nbf: None,
            sid: "test-sid".to_string(),
            scope: Vec::new(),
            roles: Vec::new(),
            attrs,
        }
    }

    /// SEC-A2: only an `access=admin` caller may read the system-wide audit
    /// trail; write-tier and attribute-less callers are refused.
    #[test]
    fn recent_audit_requires_admin() {
        assert!(claims_have_admin(&claims_with_access(&["admin"])));
        assert!(claims_have_admin(&claims_with_access(&["write", "admin"])));
        assert!(!claims_have_admin(&claims_with_access(&["write"])));
        assert!(!claims_have_admin(&claims_with_access(&[])));
    }

    /// SEC-A3: the magic-link token/URL is logged ONLY in development, so it
    /// never lands in production (or test) logs.
    #[test]
    fn magic_link_url_logged_only_in_development() {
        assert!(log_magic_link_url(&Environment::Development));
        assert!(!log_magic_link_url(&Environment::Production));
        assert!(!log_magic_link_url(&Environment::Test));
        assert!(!log_magic_link_url(&Environment::Any(
            "staging".to_string()
        )));
    }

    fn allowlist() -> Vec<String> {
        vec![
            "https://organization.example.com".to_string(),
            "https://case.example.com".to_string(),
        ]
    }

    #[test]
    fn allowlisted_return_url_is_honoured() {
        assert_eq!(
            choose_frontend(
                Some("https://organization.example.com"),
                &allowlist(),
                DEFAULT
            ),
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
