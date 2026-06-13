//! Email magic-link authentication with **stateless signed tokens**.
//!
//! No auth tables: identity comes from a configured allowlist, and both
//! the short-lived *magic* token and the longer-lived *session* token
//! are signed JWTs (HS256). The `aud` claim separates the two so a magic
//! token can never be replayed as a session and vice-versa.
//!
//! See `spec/auth.md` for the full flow and configuration matrix.

pub mod mailer;

use axum::http::{header, HeaderMap};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use mailer::Mailer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Name of the HttpOnly session cookie.
pub const SESSION_COOKIE: &str = "cts_session";

const AUD_MAGIC: &str = "magic";
const AUD_SESSION: &str = "session";

/// A resolved, authenticated person.
#[derive(Debug, Clone, Serialize)]
pub struct Identity {
    pub email: String,
    pub name: String,
    pub role: Option<String>,
}

#[derive(Debug)]
pub enum AuthError {
    /// The token could not be minted or failed validation (bad
    /// signature, wrong audience, or expired).
    Token,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    aud: String,
    iat: usize,
    exp: usize,
}

/// One allowlist entry, as read from `settings.auth.allowlist`.
#[derive(Debug, Clone, Deserialize)]
pub struct AllowlistEntry {
    pub email: String,
    pub name: String,
    #[serde(default)]
    pub role: Option<String>,
}

/// Deserialized from the `settings.auth` block of the Loco config.
#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    #[serde(default)]
    pub secret: String,
    #[serde(default = "default_magic_ttl")]
    pub magic_link_ttl_seconds: i64,
    #[serde(default = "default_session_ttl")]
    pub session_ttl_seconds: i64,
    #[serde(default)]
    pub require_session: bool,
    #[serde(default)]
    pub expose_magic_link: bool,
    #[serde(default)]
    pub cookie_secure: bool,
    #[serde(default = "default_frontend")]
    pub frontend_url: String,
    #[serde(default)]
    pub allowlist: Vec<AllowlistEntry>,
}

fn default_magic_ttl() -> i64 {
    600
}
fn default_session_ttl() -> i64 {
    86_400
}
fn default_frontend() -> String {
    "http://localhost:5173".to_string()
}

impl Default for AuthConfig {
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
    config: AuthConfig,
    allowlist: HashMap<String, Identity>,
    encoding: EncodingKey,
    decoding: DecodingKey,
    pub mailer: Box<dyn Mailer>,
}

impl AuthState {
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
            mailer,
        }
    }

    pub fn require_session(&self) -> bool {
        self.config.require_session
    }

    pub fn expose_magic_link(&self) -> bool {
        self.config.expose_magic_link
    }

    /// Resolve a known identity by email (case-insensitive).
    pub fn identity_for_email(&self, email: &str) -> Option<Identity> {
        self.allowlist.get(&email.trim().to_lowercase()).cloned()
    }

    fn now() -> i64 {
        chrono::Utc::now().timestamp()
    }

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

    pub fn mint_magic_token(&self, identity: &Identity) -> Result<String, AuthError> {
        self.encode_token(identity, AUD_MAGIC, self.config.magic_link_ttl_seconds)
    }

    pub fn verify_magic_token(&self, token: &str) -> Result<Identity, AuthError> {
        self.decode_token(token, AUD_MAGIC)
    }

    pub fn mint_session_token(&self, identity: &Identity) -> Result<String, AuthError> {
        self.encode_token(identity, AUD_SESSION, self.config.session_ttl_seconds)
    }

    pub fn verify_session_token(&self, token: &str) -> Result<Identity, AuthError> {
        self.decode_token(token, AUD_SESSION)
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
    /// cookie first, then an `Authorization: Bearer` header.
    pub fn identity_from_headers(&self, headers: &HeaderMap) -> Option<Identity> {
        let token = cookie_token(headers).or_else(|| bearer_token(headers))?;
        self.verify_session_token(&token).ok()
    }
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let token = value
        .strip_prefix("Bearer ")
        .or_else(|| value.strip_prefix("bearer "))?;
    Some(token.trim().to_string())
}

fn cookie_token(headers: &HeaderMap) -> Option<String> {
    let cookie = headers.get(header::COOKIE)?.to_str().ok()?;
    let needle = format!("{SESSION_COOKIE}=");
    cookie
        .split(';')
        .map(str::trim)
        .find_map(|part| part.strip_prefix(&needle))
        .map(str::to_string)
}
