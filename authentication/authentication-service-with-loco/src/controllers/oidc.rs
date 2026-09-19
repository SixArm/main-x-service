//! OIDC identity federation (EV-2) — an **alternative front door** onto
//! the exact same session §3 already establishes for a magic link,
//! reached via the magic-link **bridge** described below rather than a
//! parallel session-establishment path: same `sessions` table, same
//! `__Host-mxi_session` cookie, same PASETO minting, just arrived at
//! through the existing consume-token route instead of a duplicate of
//! it. Design: `agents/share/authentication-sessions.md` §7a.
//!
//! | Method | Path | Purpose |
//! |---|---|---|
//! | `GET` | `/api/auth/oidc/login` | Redirect to the configured `IdP`'s authorization endpoint (PKCE + state + nonce). |
//! | `GET` | `/api/auth/oidc/callback` | Exchange the authorization code, verify the ID token, map claims, bridge to the front end. |
//!
//! Both routes are `404` (not merely inert) unless
//! [`crate::oidc::OidcConfig::from_env`] resolves — this whole surface
//! stays invisible to a deployment that has not opted in, exactly as
//! §7a requires ("federation is opt-in per deployment").
//!
//! **Why "bridge", not "establish the session directly".** The
//! `__Host-mxi_session` cookie (`crate::cookie`) is host-locked to
//! *this* service's own origin — correct for the magic-link flow, whose
//! consuming request (`GET /api/auth/magic-link/{token}`) is a
//! server-to-server call the front-end's own BFF makes on the front
//! end's behalf (see `authentication-front-end-with-svelte`'s
//! `src/routes/verify/+page.server.ts`). OIDC is different: the
//! **browser itself** must navigate to the `IdP` and back, so the only
//! response this service can hand back is one addressed to *this*
//! origin — setting the session cookie here and redirecting the browser
//! to `FRONTEND_URL` would leave the cookie stranded on an origin the
//! front end never talks to. The fix reuses the existing magic-link
//! bridge verbatim: mint a single-use, short-lived
//! [`crate::models::users::Model::create_magic_link`] token for the
//! now-federated-and-verified user and redirect the browser to
//! `{frontend}/verify?token=…` — the front end's already-tested
//! `/verify` BFF route (a server-to-server call to `GET
//! /api/auth/magic-link/{token}`) does the actual session establishment
//! and cookie re-hosting, identically to a magic-link sign-in. The
//! trade: the audit trail records the OIDC identity event
//! (`oidc_sign_in`) and the generic session-establishment mechanics
//! (`magic_link_redeemed`) as two adjacent rows rather than one
//! OIDC-named row — correlatable by `pid`/`source_ip`/timestamp, and
//! documented here rather than silently accepted.
//!
//! **Fail-closed metadata fetch (mirrors SEC-V1).** OIDC discovery and
//! the JWKS fetch both go through `openidconnect`'s own HTTP client,
//! built once with a request timeout and no redirect-following — the
//! same MITM-injected-document defence `authentication-verifier`'s own
//! boot-time key-set fetch already applies, extended here to an `IdP`'s
//! metadata document.
//!
//! **Provisioning.** A first successful federated sign-in for an email
//! with no existing account is refused (`403`) unless the deployment
//! opts into [`crate::oidc::JIT_PROVISIONING_ENV`] — the safer of §7a's
//! two documented leans: auto-provisioning would let the `IdP` control
//! account creation, which a deployment should opt into, not inherit.

use std::net::SocketAddr;
use std::sync::OnceLock;
use std::time::Duration;

use axum::extract::{ConnectInfo, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Redirect;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use openidconnect::core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
};
use serde::{Deserialize, Serialize};

use crate::controllers::auth::{allowed_frontends, choose_frontend, default_frontend};
use crate::models::{auth_events::Model as AuthEvent, sessions, users};
use crate::oidc::OidcConfig;

/// Short-lived flow-state cookie: carries the CSRF state, nonce, and
/// PKCE verifier across the redirect to the `IdP` and back. `HttpOnly` +
/// `Secure` + host-locked (`__Host-`), never read by JS; the browser
/// merely carries it there and back.
const FLOW_COOKIE: &str = "__Host-mxi_oidc_flow";
/// Generous enough for a human to complete an `IdP` login, tight enough
/// that an abandoned flow cannot be replayed hours later.
const FLOW_COOKIE_MAX_AGE_SECS: i64 = 600;

fn unauthorized_oidc(message: &str) -> Error {
    Error::CustomError(
        StatusCode::UNAUTHORIZED,
        ErrorDetail::new("unauthorized", message),
    )
}

fn forbidden_oidc(message: &str) -> Error {
    Error::CustomError(
        StatusCode::FORBIDDEN,
        ErrorDetail::new("forbidden", message),
    )
}

/// The shared, **non-redirecting**, timeout-bounded HTTP client for
/// OIDC discovery, JWKS fetch, and token exchange (SEC-V1 posture):
/// the `IdP` is a configured, trusted origin, so a `3xx` from it bouncing
/// to an attacker-chosen host must not be followed.
fn http_client() -> &'static openidconnect::reqwest::Client {
    static CLIENT: OnceLock<openidconnect::reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        openidconnect::reqwest::ClientBuilder::new()
            .redirect(openidconnect::reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(10))
            .build()
            .expect("build the non-redirecting OIDC HTTP client")
    })
}

/// Discover the `IdP`'s provider metadata (the `.well-known` document).
/// Returned separately from the configured client — building the
/// client via `.set_redirect_uri(...)` changes its type-state, and the
/// two call sites (`login` needs `authorize_url`, `callback` needs
/// `exchange_code`) each build the client locally so the compiler can
/// infer the right concrete (large, generic) type rather than this
/// function needing to name it.
async fn discover(config: &OidcConfig) -> Result<CoreProviderMetadata> {
    let issuer = IssuerUrl::new(config.issuer_url.clone())
        .map_err(|e| Error::string(&format!("invalid AUTH_OIDC_ISSUER_URL: {e}")))?;
    CoreProviderMetadata::discover_async(issuer, http_client())
        .await
        .map_err(|e| Error::string(&format!("OIDC discovery failed: {e}")))
}

/// Build the `OAuth2` client id/secret pair + redirect URL from config.
/// Pure string wrapping, shared by both handlers.
fn client_parts(config: &OidcConfig) -> Result<(ClientId, Option<ClientSecret>, RedirectUrl)> {
    let redirect = RedirectUrl::new(config.redirect_url.clone())
        .map_err(|e| Error::string(&format!("invalid AUTH_OIDC_REDIRECT_URL: {e}")))?;
    Ok((
        ClientId::new(config.client_id.clone()),
        Some(ClientSecret::new(config.client_secret.clone())),
        redirect,
    ))
}

/// The flow cookie's payload. `frontend` is the return base resolved at
/// `/login` time (`return_url` validated against `AUTH_ALLOWED_FRONTENDS`,
/// exactly as the magic-link `return_url` knob works) and carried across
/// the round trip to the `IdP` so `/callback` bridges to the same app
/// that started the sign-in, not always the family-wide default.
#[derive(Debug, Serialize, Deserialize)]
struct FlowState {
    csrf_state: String,
    nonce: String,
    pkce_verifier: String,
    frontend: String,
}

fn set_flow_cookie(state: &FlowState) -> Result<String> {
    let json = serde_json::to_string(state).map_err(|e| Error::string(&e.to_string()))?;
    let encoded = URL_SAFE_NO_PAD.encode(json.as_bytes());
    Ok(format!(
        "{FLOW_COOKIE}={encoded}; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age={FLOW_COOKIE_MAX_AGE_SECS}"
    ))
}

fn clear_flow_cookie() -> String {
    format!("{FLOW_COOKIE}=; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=0")
}

fn read_flow_cookie(headers: &HeaderMap) -> Option<FlowState> {
    let raw = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    let prefix = format!("{FLOW_COOKIE}=");
    let encoded = raw
        .split(';')
        .map(str::trim)
        .find_map(|pair| pair.strip_prefix(&prefix))?;
    let bytes = URL_SAFE_NO_PAD.decode(encoded).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// `GET /api/auth/oidc/login` query.
#[derive(Debug, Deserialize)]
struct LoginQuery {
    /// Optional return base (the requesting front-end's origin), honoured
    /// only when it exactly matches `AUTH_ALLOWED_FRONTENDS` — the same
    /// per-app SSO knob `MagicLinkParams::return_url` already gives the
    /// magic-link flow (`controllers::auth::choose_frontend`).
    return_url: Option<String>,
}

/// `GET /api/auth/oidc/login` — redirect to the `IdP`'s authorization
/// endpoint.
#[debug_handler]
async fn login(Query(query): Query<LoginQuery>) -> Result<axum::response::Response> {
    let Some(config) = OidcConfig::from_env() else {
        return Err(Error::NotFound);
    };
    let metadata = discover(&config).await?;
    let (client_id, client_secret, redirect) = client_parts(&config)?;
    let client = CoreClient::from_provider_metadata(metadata, client_id, client_secret)
        .set_redirect_uri(redirect);
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let (auth_url, csrf_token, nonce) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    let frontend = choose_frontend(
        query.return_url.as_deref(),
        &allowed_frontends(),
        &default_frontend(),
    );
    let flow = FlowState {
        csrf_state: csrf_token.secret().clone(),
        nonce: nonce.secret().clone(),
        pkce_verifier: pkce_verifier.secret().clone(),
        frontend,
    };
    let mut response = Redirect::to(auth_url.as_str()).into_response();
    response.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        set_flow_cookie(&flow)?
            .parse()
            .expect("valid set-cookie value"),
    );
    Ok(response)
}

/// `GET /api/auth/oidc/callback` query.
#[derive(Debug, Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

/// Exchange the authorization code, verify the ID token against the
/// nonce, and return the verified email plus the token's raw claims
/// (for [`crate::oidc::map_claims`]).
async fn exchange_and_verify(
    config: &OidcConfig,
    flow: FlowState,
    code: String,
) -> Result<(String, serde_json::Value)> {
    let metadata = discover(config).await?;
    let (client_id, client_secret, redirect) = client_parts(config)?;
    let client = CoreClient::from_provider_metadata(metadata, client_id, client_secret)
        .set_redirect_uri(redirect);
    let token_response = client
        .exchange_code(AuthorizationCode::new(code))
        .map_err(|e| unauthorized_oidc(&format!("token exchange rejected: {e}")))?
        .set_pkce_verifier(PkceCodeVerifier::new(flow.pkce_verifier))
        .request_async(http_client())
        .await
        .map_err(|e| unauthorized_oidc(&format!("token exchange failed: {e}")))?;

    let id_token = token_response
        .id_token()
        .ok_or_else(|| unauthorized_oidc("the identity provider returned no ID token"))?;
    let nonce = Nonce::new(flow.nonce);
    let verifier = client.id_token_verifier();
    let claims = id_token
        .claims(&verifier, &nonce)
        .map_err(|e| unauthorized_oidc(&format!("ID token verification failed: {e}")))?;
    let email = claims
        .email()
        .map(|e| e.as_str().to_string())
        .ok_or_else(|| unauthorized_oidc("the identity provider returned no email claim"))?;

    // Re-decode the already-verified ID token's payload as plain JSON
    // for claim mapping: `claims` above is the typed, **cryptographically
    // verified** view (subject/email/nonce all checked); this is the
    // same bytes, read a second time to reach claims the typed view
    // does not name (the deployment-configured claim_map). Nothing
    // between the two reads is untrusted — it is the same signed
    // payload, decoded twice.
    let raw_claims = decode_id_token_payload(id_token)
        .map_err(|e| Error::string(&format!("could not read ID token claims: {e}")))?;
    Ok((email, raw_claims))
}

/// Find the local account for `email`, or — only when
/// [`OidcConfig::jit_provisioning`] opts in — provision one. Any other
/// outcome (no account, JIT off) is a named, audited refusal.
async fn find_or_provision_user(
    ctx: &AppContext,
    email: &str,
    config: &OidcConfig,
    source_ip: &str,
) -> Result<users::Model> {
    match users::Model::find_by_email(&ctx.db, email).await {
        Ok(user) if !user.is_deleted() => Ok(user),
        _ if config.jit_provisioning => users::Model::create_passwordless(&ctx.db, email, email)
            .await
            .map_err(|e| Error::string(&e.to_string())),
        _ => {
            AuthEvent::record_best_effort(
                &ctx.db,
                "oidc_sign_in",
                Some(email),
                None,
                Some("no_account"),
                Some(source_ip),
            )
            .await;
            Err(forbidden_oidc(
                "no account exists for this identity; ask an administrator to provision one",
            ))
        }
    }
}

/// Bridge the now-verified `IdP` identity onto the front end's own
/// origin by minting a single-use, short-lived magic-link token
/// ([`users::Model::create_magic_link`] — SEC-A9 hash-at-rest, SEC-A4
/// atomic single-use consume, ~5-minute expiry: exactly the properties
/// a session-establishment bridge needs) and redirecting the browser to
/// `{frontend}/verify?token=…`. The front end's existing `/verify` BFF
/// route consumes it via `GET /api/auth/magic-link/{token}`
/// (`controllers::auth::verify`), which does the actual session
/// creation and re-hosts the resulting cookie on the front end's own
/// origin — see this module's doc comment for why a session cannot be
/// established directly here.
async fn bridge_to_frontend(
    ctx: &AppContext,
    user: &users::Model,
    frontend: &str,
    source_ip: &str,
) -> Result<axum::response::Response> {
    let bridged = user
        .clone()
        .into_active_model()
        .create_magic_link(&ctx.db)
        .await
        .map_err(|e| Error::string(&e.to_string()))?;
    let Some(token) = bridged.magic_link_token else {
        return Err(Error::string("failed to mint a session-bridge token"));
    };

    AuthEvent::record_best_effort(
        &ctx.db,
        "oidc_sign_in",
        Some(&user.email),
        Some(user.pid),
        Some("ok"),
        Some(source_ip),
    )
    .await;

    let destination = format!("{frontend}/verify?token={token}");
    let mut response = Redirect::to(&destination).into_response();
    response.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        clear_flow_cookie().parse().expect("valid set-cookie value"),
    );
    Ok(response)
}

/// `GET /api/auth/oidc/callback` — exchange the code, verify the ID
/// token, map claims, find-or-refuse the local account, establish the
/// session exactly as a magic-link redemption does.
#[debug_handler]
async fn callback(
    State(ctx): State<AppContext>,
    connect_info: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(query): Query<CallbackQuery>,
) -> Result<axum::response::Response> {
    let source_ip = connect_info.0.ip().to_string();
    let Some(config) = OidcConfig::from_env() else {
        return Err(Error::NotFound);
    };

    if let Some(error) = &query.error {
        AuthEvent::record_best_effort(
            &ctx.db,
            "oidc_sign_in",
            None,
            None,
            Some(&format!("idp_error:{error}")),
            Some(&source_ip),
        )
        .await;
        return Err(unauthorized_oidc(
            "the identity provider declined the sign-in",
        ));
    }

    let Some(flow) = read_flow_cookie(&headers) else {
        return Err(unauthorized_oidc("missing or expired sign-in flow"));
    };
    let (Some(code), Some(state)) = (query.code.clone(), query.state.clone()) else {
        return Err(unauthorized_oidc("missing code or state"));
    };
    if state != flow.csrf_state {
        AuthEvent::record_best_effort(
            &ctx.db,
            "oidc_sign_in",
            None,
            None,
            Some("state_mismatch"),
            Some(&source_ip),
        )
        .await;
        return Err(unauthorized_oidc("state mismatch"));
    }

    let frontend = flow.frontend.clone();
    let (email, raw_claims) = exchange_and_verify(&config, flow, code).await?;
    let (attrs, skipped_claims) = crate::oidc::map_claims(&raw_claims, &config.claim_map);
    if !skipped_claims.is_empty() {
        tracing::warn!(
            claims = ?skipped_claims,
            "OIDC claim mapping skipped one or more configured claims"
        );
    }

    let user = find_or_provision_user(&ctx, &email, &config, &source_ip).await?;
    let updated = user
        .into_active_model()
        .set_attributes(&ctx.db, users::attributes_to_value(&attrs))
        .await
        .map_err(|e| Error::string(&e.to_string()))?;
    // SEC-A8 (same reasoning as the admin-API attribute path): a live
    // session already snapshotted the OLD attrs, so revoke before the
    // bridge below can lead to a new one being established.
    sessions::Model::revoke_all_for_user(&ctx.db, updated.pid)
        .await
        .map_err(|e| Error::string(&e.to_string()))?;

    let response = bridge_to_frontend(&ctx, &updated, &frontend, &source_ip).await?;
    Ok(response)
}

/// Decode an ID token's payload segment (the middle third of the
/// compact JWT serialization) as plain JSON. The token has **already**
/// been cryptographically verified by the caller (`id_token.claims`
/// above, checked against the `IdP`'s published key and the nonce)
/// before this ever runs — this is a second, unauthenticated read of
/// the same already-trusted bytes, purely to reach claim names the
/// crate's typed view does not surface, never a substitute for that
/// verification.
fn decode_id_token_payload(
    id_token: &openidconnect::core::CoreIdToken,
) -> std::result::Result<serde_json::Value, String> {
    let compact = id_token.to_string();
    let payload = compact
        .split('.')
        .nth(1)
        .ok_or_else(|| "malformed ID token".to_string())?;
    let bytes = URL_SAFE_NO_PAD.decode(payload).map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

/// The OIDC federation routes. `#[cfg(feature = "oidc")]`d in
/// `app.rs` — this whole surface does not exist in a build without the
/// feature.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/auth/oidc")
        .add("/login", get(login))
        .add("/callback", get(callback))
}
