//! Email magic-link authentication.
//!
//! Identity comes from a configured allowlist. The short-lived *magic*
//! token (the sign-in link) is a signed JWT (HS256, single-use, ~10 min) —
//! the short-lived signed-token case `agents/share/jwt.md` explicitly
//! tolerates. The **session is NOT a JWT**: per `jwt.md`, signing in mints
//! an **opaque server-side session id** held in the HttpOnly `cts_session`
//! cookie and backed by an in-process session store on [`AuthState`]. (A
//! durable Postgres-backed store is a roadmap upgrade — see `spec/auth.md`.)
//! The magic token's `aud` claim stops it being replayed as anything else.
//!
//! See `spec/auth.md` for the full flow and configuration matrix.

/// Email delivery of magic links (trait + log-only default impl).
pub mod mailer;

use axum::http::{HeaderMap, header};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use mailer::Mailer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

/// Name of the HttpOnly session cookie.
pub const SESSION_COOKIE: &str = "cts_session";

/// `aud` claim value for short-lived magic-link (sign-in) tokens. The
/// session is no longer a token, so this is the only audience.
const AUD_MAGIC: &str = "magic";

/// A resolved, authenticated person.
#[derive(Debug, Clone, Serialize)]
pub struct Identity {
    /// Canonical email address (the allowlist key, original casing).
    pub email: String,
    /// Human-readable display name.
    pub name: String,
    /// Optional role string (e.g. `"admin"`), if assigned in the allowlist.
    pub role: Option<String>,
}

/// Error returned by the auth layer. Deliberately opaque so failures do
/// not leak why a token was rejected.
#[derive(Debug)]
pub enum AuthError {
    /// The token could not be minted or failed validation (bad
    /// signature, wrong audience, or expired).
    Token,
}

/// JWT payload. Mixes the standard registered claims (`aud`, `iat`,
/// `exp`) with our own subject/name/role fields.
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    /// Subject — the identity's email address (registered `sub` claim).
    sub: String,
    /// Display name carried through so headers can resolve an identity
    /// without a second lookup.
    name: String,
    /// Optional role; omitted from the JSON when `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    /// Audience (registered `aud` claim) — `AUD_MAGIC` or `AUD_SESSION`.
    aud: String,
    /// Issued-at, Unix seconds (registered `iat` claim).
    iat: usize,
    /// Expiry, Unix seconds (registered `exp` claim); enforced on decode.
    exp: usize,
}

/// One opaque server-side session: the resolved identity plus the Unix
/// second at which it expires. Held in [`AuthState`]'s in-process store
/// and keyed by the opaque session id carried in the `cts_session` cookie.
struct SessionEntry {
    /// The signed-in identity this session authenticates.
    identity: Identity,
    /// Expiry, Unix seconds; the session is invalid at/after this instant.
    expires_at: i64,
}

/// One allowlist entry, as read from `settings.auth.allowlist`.
#[derive(Debug, Clone, Deserialize)]
pub struct AllowlistEntry {
    /// Email that may sign in (matched case-insensitively).
    pub email: String,
    /// Display name attached to the resolved identity.
    pub name: String,
    /// Optional role granted to this entry.
    #[serde(default)]
    pub role: Option<String>,
}

/// Deserialized from the `settings.auth` block of the Loco config.
#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    /// HS256 signing secret. Empty means insecure (dev only); see
    /// [`AuthConfig::secret_is_insecure`].
    #[serde(default)]
    pub secret: String,
    /// Lifetime of a magic-link token, in seconds (default 600 = 10 min).
    #[serde(default = "default_magic_ttl")]
    pub magic_link_ttl_seconds: i64,
    /// Lifetime of a session token, in seconds (default 86_400 = 1 day).
    #[serde(default = "default_session_ttl")]
    pub session_ttl_seconds: i64,
    /// When true, protected routes demand a valid session cookie/bearer.
    #[serde(default)]
    pub require_session: bool,
    /// When true, the magic link is returned in the API response (dev
    /// convenience; never enable in production).
    #[serde(default)]
    pub expose_magic_link: bool,
    /// When true, session cookies carry the `Secure` attribute (HTTPS).
    #[serde(default)]
    pub cookie_secure: bool,
    /// Front-end base URL used to build the magic-link callback.
    #[serde(default = "default_frontend")]
    pub frontend_url: String,
    /// The set of identities permitted to sign in.
    #[serde(default)]
    pub allowlist: Vec<AllowlistEntry>,
}

/// Default magic-link TTL: 600 seconds (10 minutes).
fn default_magic_ttl() -> i64 {
    600
}
/// Default session TTL: 86_400 seconds (24 hours).
fn default_session_ttl() -> i64 {
    86_400
}
/// Default front-end origin used when none is configured (Vite dev port).
fn default_frontend() -> String {
    "http://localhost:5173".to_string()
}

impl Default for AuthConfig {
    /// Dev-safe defaults: empty (insecure) secret, standard TTLs,
    /// session not required, no allowlist.
    fn default() -> Self {
        Self {
            secret: String::new(),
            magic_link_ttl_seconds: default_magic_ttl(),
            session_ttl_seconds: default_session_ttl(),
            require_session: false,
            expose_magic_link: false,
            cookie_secure: false,
            frontend_url: default_frontend(),
            allowlist: Vec::new(),
        }
    }
}

impl AuthConfig {
    /// True when no usable signing secret was configured — the caller
    /// should warn loudly (production must set `AUTH_SECRET`).
    pub fn secret_is_insecure(&self) -> bool {
        self.secret.trim().is_empty()
    }
}

/// Runtime auth state, injected as an Axum `Extension` and captured by
/// the session-guard middleware.
pub struct AuthState {
    /// The deserialized auth configuration (TTLs, flags, secret).
    config: AuthConfig,
    /// Lower-cased-email -> resolved identity, derived from the allowlist
    /// for O(1) case-insensitive lookups.
    allowlist: HashMap<String, Identity>,
    /// HS256 encoding key, built once from the configured secret.
    encoding: EncodingKey,
    /// HS256 decoding key, built once from the configured secret.
    decoding: DecodingKey,
    /// In-process opaque-session store: session id → entry. Replaces the
    /// former JWT session token (per `jwt.md`). Process-local — sessions do
    /// not survive a restart and are not shared across replicas; a durable
    /// Postgres-backed store is the roadmap upgrade (`spec/auth.md`).
    sessions: Mutex<HashMap<String, SessionEntry>>,
    /// Mailer used to deliver magic links (public so handlers can call it).
    pub mailer: Box<dyn Mailer>,
}

impl AuthState {
    /// Build runtime auth state from config and a mailer.
    ///
    /// Derives the HS256 encoding/decoding keys from `config.secret` and
    /// pre-indexes the allowlist by lower-cased email for fast lookup.
    ///
    /// # Parameters
    /// - `config`: the deserialized `settings.auth` block.
    /// - `mailer`: magic-link delivery backend.
    pub fn new(config: AuthConfig, mailer: Box<dyn Mailer>) -> Self {
        let encoding = EncodingKey::from_secret(config.secret.as_bytes());
        let decoding = DecodingKey::from_secret(config.secret.as_bytes());
        let allowlist = config
            .allowlist
            .iter()
            .map(|e| {
                (
                    e.email.trim().to_lowercase(),
                    Identity {
                        email: e.email.trim().to_string(),
                        name: e.name.clone(),
                        role: e.role.clone(),
                    },
                )
            })
            .collect();
        Self {
            config,
            allowlist,
            encoding,
            decoding,
            sessions: Mutex::new(HashMap::new()),
            mailer,
        }
    }

    /// Whether protected routes require an authenticated session.
    pub fn require_session(&self) -> bool {
        self.config.require_session
    }

    /// Whether the magic link should be echoed in API responses (dev only).
    pub fn expose_magic_link(&self) -> bool {
        self.config.expose_magic_link
    }

    /// Resolve a known identity by email (case-insensitive).
    pub fn identity_for_email(&self, email: &str) -> Option<Identity> {
        self.allowlist.get(&email.trim().to_lowercase()).cloned()
    }

    /// Current Unix time in seconds, used for `iat`/`exp`.
    fn now() -> i64 {
        chrono::Utc::now().timestamp()
    }

    /// Mint an HS256 JWT for `identity` scoped to `aud`, expiring `ttl`
    /// seconds from now.
    ///
    /// Shared by the magic and session minting helpers; the `aud`
    /// argument is what keeps the two token kinds non-interchangeable.
    ///
    /// # Errors
    /// Returns [`AuthError::Token`] if the JWT library fails to encode.
    fn encode_token(&self, identity: &Identity, aud: &str, ttl: i64) -> Result<String, AuthError> {
        let iat = Self::now();
        let claims = Claims {
            sub: identity.email.clone(),
            name: identity.name.clone(),
            role: identity.role.clone(),
            aud: aud.to_string(),
            iat: iat.max(0) as usize,
            exp: (iat + ttl).max(0) as usize,
        };
        encode(&Header::new(Algorithm::HS256), &claims, &self.encoding)
            .map_err(|_| AuthError::Token)
    }

    /// Verify an HS256 JWT, requiring it to carry the expected `aud`
    /// (and a present, unexpired `exp`), and return the embedded identity.
    ///
    /// # Errors
    /// Returns [`AuthError::Token`] for a bad signature, wrong audience,
    /// missing required claim, or expired token.
    fn decode_token(&self, token: &str, aud: &str) -> Result<Identity, AuthError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_audience(&[aud]);
        validation.set_required_spec_claims(&["exp", "aud"]);
        let data =
            decode::<Claims>(token, &self.decoding, &validation).map_err(|_| AuthError::Token)?;
        Ok(Identity {
            email: data.claims.sub,
            name: data.claims.name,
            role: data.claims.role,
        })
    }

    /// Mint a short-lived magic-link token (`aud = "magic"`).
    ///
    /// # Errors
    /// Returns [`AuthError::Token`] if encoding fails.
    pub fn mint_magic_token(&self, identity: &Identity) -> Result<String, AuthError> {
        self.encode_token(identity, AUD_MAGIC, self.config.magic_link_ttl_seconds)
    }

    /// Verify a magic-link token and resolve its identity.
    ///
    /// # Errors
    /// Returns [`AuthError::Token`] if the token is invalid, expired, or
    /// not a magic-audience token.
    pub fn verify_magic_token(&self, token: &str) -> Result<Identity, AuthError> {
        self.decode_token(token, AUD_MAGIC)
    }

    /// Establish a new opaque server-side session for `identity` and return
    /// its session id (the value placed in the `cts_session` cookie). The id
    /// is unguessable (UUIDv4) and is **not** a token — it carries no claims
    /// and is meaningless without the server-side store.
    pub fn create_session(&self, identity: &Identity) -> String {
        let sid = Uuid::new_v4().to_string();
        let expires_at = Self::now() + self.config.session_ttl_seconds;
        let mut store = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        // Opportunistically drop expired entries so the map can't grow
        // unbounded across the process lifetime.
        let now = Self::now();
        store.retain(|_, e| e.expires_at > now);
        store.insert(
            sid.clone(),
            SessionEntry {
                identity: identity.clone(),
                expires_at,
            },
        );
        sid
    }

    /// Resolve the identity for an opaque session id, or `None` when the id
    /// is unknown or the session has expired (expired entries are evicted).
    pub fn session_identity(&self, sid: &str) -> Option<Identity> {
        let mut store = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        match store.get(sid) {
            Some(entry) if entry.expires_at > Self::now() => Some(entry.identity.clone()),
            Some(_) => {
                store.remove(sid);
                None
            }
            None => None,
        }
    }

    /// Revoke an opaque session (sign-out). Idempotent — revoking an absent
    /// session is a no-op.
    pub fn revoke_session(&self, sid: &str) {
        let mut store = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        store.remove(sid);
    }

    /// Revoke whatever session the request presents (cookie or bearer), if
    /// any. Used by sign-out so the server-side session is dropped, not just
    /// the cookie.
    pub fn revoke_from_headers(&self, headers: &HeaderMap) {
        if let Some(sid) = cookie_token(headers).or_else(|| bearer_token(headers)) {
            self.revoke_session(&sid);
        }
    }

    /// Build the magic link a user clicks to sign in.
    pub fn magic_link(&self, token: &str) -> String {
        format!(
            "{}/auth/callback?token={}",
            self.config.frontend_url.trim_end_matches('/'),
            token
        )
    }

    /// `Set-Cookie` value that establishes the session.
    pub fn session_cookie(&self, token: &str) -> String {
        let mut c = format!(
            "{}={}; HttpOnly; SameSite=Lax; Path=/; Max-Age={}",
            SESSION_COOKIE, token, self.config.session_ttl_seconds
        );
        if self.config.cookie_secure {
            c.push_str("; Secure");
        }
        c
    }

    /// `Set-Cookie` value that clears the session.
    pub fn clear_cookie(&self) -> String {
        let mut c = format!("{SESSION_COOKIE}=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0");
        if self.config.cookie_secure {
            c.push_str("; Secure");
        }
        c
    }

    /// Resolve the signed-in identity from request headers — the session
    /// cookie first, then an `Authorization: Bearer` header. The presented
    /// value is an opaque session id looked up in the server-side store.
    pub fn identity_from_headers(&self, headers: &HeaderMap) -> Option<Identity> {
        let sid = cookie_token(headers).or_else(|| bearer_token(headers))?;
        self.session_identity(&sid)
    }
}

/// Extract a bearer token from the `Authorization` header, if present.
///
/// Accepts both `Bearer ` and lower-case `bearer ` prefixes and trims
/// surrounding whitespace. Returns `None` when the header is absent,
/// non-ASCII, or not a bearer scheme.
fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let token = value
        .strip_prefix("Bearer ")
        .or_else(|| value.strip_prefix("bearer "))?;
    Some(token.trim().to_string())
}

/// Extract the session token from the `Cookie` header, if present.
///
/// Splits the header on `;`, trims each part, and returns the value of
/// the `SESSION_COOKIE` pair. Returns `None` when the cookie is absent
/// or the header is non-ASCII.
fn cookie_token(headers: &HeaderMap) -> Option<String> {
    let cookie = headers.get(header::COOKIE)?.to_str().ok()?;
    let needle = format!("{SESSION_COOKIE}=");
    cookie
        .split(';')
        .map(str::trim)
        .find_map(|part| part.strip_prefix(&needle))
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    //! In-process pins for the opaque server-side session store. No DB and
    //! no network — the store lives on `AuthState`, so create / resolve /
    //! expire / revoke and the header extraction are all exercised here.
    use super::*;
    use crate::auth::mailer::LogMailer;

    /// Build an `AuthState` with the given session TTL (seconds). A negative
    /// TTL yields sessions that are already expired at creation.
    fn state(session_ttl: i64) -> AuthState {
        let config = AuthConfig {
            session_ttl_seconds: session_ttl,
            ..AuthConfig::default()
        };
        AuthState::new(config, Box::new(LogMailer))
    }

    fn alice() -> Identity {
        Identity {
            email: "alice@example.com".to_string(),
            name: "Alice".to_string(),
            role: Some("admin".to_string()),
        }
    }

    fn cookie_headers(sid: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(
            header::COOKIE,
            format!("{SESSION_COOKIE}={sid}").parse().unwrap(),
        );
        h
    }

    fn bearer_headers(sid: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(
            header::AUTHORIZATION,
            format!("Bearer {sid}").parse().unwrap(),
        );
        h
    }

    #[test]
    fn create_then_resolve_round_trips_identity() {
        let s = state(3600);
        let sid = s.create_session(&alice());
        let got = s.session_identity(&sid).expect("session resolves");
        assert_eq!(got.email, "alice@example.com");
        assert_eq!(got.role.as_deref(), Some("admin"));
    }

    #[test]
    fn unknown_session_id_resolves_to_none() {
        assert!(state(3600).session_identity("not-a-real-sid").is_none());
    }

    #[test]
    fn revoked_session_resolves_to_none() {
        let s = state(3600);
        let sid = s.create_session(&alice());
        s.revoke_session(&sid);
        assert!(s.session_identity(&sid).is_none());
    }

    #[test]
    fn expired_session_resolves_to_none() {
        // Negative TTL ⇒ expires_at is in the past at creation.
        let s = state(-10);
        let sid = s.create_session(&alice());
        assert!(s.session_identity(&sid).is_none());
    }

    #[test]
    fn identity_resolves_from_cookie_and_bearer() {
        let s = state(3600);
        let sid = s.create_session(&alice());
        assert_eq!(
            s.identity_from_headers(&cookie_headers(&sid))
                .unwrap()
                .email,
            "alice@example.com"
        );
        assert_eq!(
            s.identity_from_headers(&bearer_headers(&sid))
                .unwrap()
                .email,
            "alice@example.com"
        );
        assert!(s.identity_from_headers(&HeaderMap::new()).is_none());
    }

    #[test]
    fn revoke_from_headers_drops_the_session() {
        let s = state(3600);
        let sid = s.create_session(&alice());
        s.revoke_from_headers(&cookie_headers(&sid));
        assert!(s.session_identity(&sid).is_none());
    }

    #[test]
    fn session_id_is_opaque_not_a_token() {
        // An opaque UUID — no dots, so it is structurally not a JWT/PASETO
        // and carries no claims.
        let sid = state(3600).create_session(&alice());
        assert!(!sid.is_empty());
        assert!(!sid.contains('.'));
    }
}
