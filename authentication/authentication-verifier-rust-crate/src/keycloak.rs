//! Keycloak (OIDC) access-token verification: the `keycloak` feature (off
//! by default).
//!
//! A Keycloak-issued JWT is verified against the realm's published keys
//! and mapped onto the same [`Claims`] a PASETO token yields, so the ABAC
//! engine ([`crate::abac`]) and every service's guard treat both alike.
//!
//! The heavy lifting (OIDC discovery, JWKS caching, signature and
//! audience checks) is [`axum-keycloak-auth`], the engine that
//! `loco-keycloak-auth` wraps; that wrapper is written against loco 0.15
//! types and cannot be used from a loco 1.x service, so this module uses
//! the engine directly.
//!
//! What this module adds on top, fail-closed:
//!
//! - **HTTPS only** for the Keycloak URL (loopback excepted), like
//!   `Verifier::from_paseto_keys_url`.
//! - **Issuer check.** The engine checks signature and audience but not
//!   `iss`; the token's issuer must equal `{url}/realms/{realm}`.
//! - **At least one audience** is mandatory: an empty list would accept a
//!   token minted for any client of the realm.
//! - **Verified email.** When the token carries an `email`, it must have
//!   `email_verified: true` (unless disabled): `$email` templates in ABAC
//!   policies would otherwise be spoofable.
//! - **Explicit role map.** Keycloak roles become ABAC attributes only
//!   through [`KeycloakSettings::role_map`]; an unmapped role grants
//!   nothing, so an unconfigured realm yields read-only callers under the
//!   default policy rather than an accidental `access=admin`.
//!
//! Build a [`KeycloakVerifier`] inside a Tokio runtime (it starts OIDC
//! discovery in the background) and call [`KeycloakVerifier::verify`] per
//! request.
//!
//! [`axum-keycloak-auth`]: https://crates.io/crates/axum-keycloak-auth

use std::collections::{BTreeMap, BTreeSet};

use axum_keycloak_auth::{
    Url,
    decode::KeycloakToken,
    instance::{KeycloakAuthInstance, KeycloakConfig},
    layer::KeycloakAuthLayer,
    role::KeycloakRole,
};
use serde::Deserialize;

use crate::{Claims, VerifyError};

/// Operator configuration for [`KeycloakVerifier`].
#[derive(Debug, Clone, Deserialize)]
pub struct KeycloakSettings {
    /// Keycloak server base URL, e.g. `https://keycloak.example.com`
    /// (no `/realms/...` suffix). Must be `https` unless it is loopback.
    pub url: String,
    /// Realm name, e.g. `mxi`.
    pub realm: String,
    /// Accepted `aud` values (typically this service's Keycloak client
    /// id). Must not be empty.
    pub audiences: Vec<String>,
    /// Keycloak role → ABAC attributes. A realm role is keyed by its name
    /// (`"editor"`); a client role by `"<client>:<role>"`
    /// (`"mxi-web:editor"`). Each value is `{ "<attr key>": ["<value>", ...] }`,
    /// e.g. `{"editor": {"access": ["write"]}, "root": {"access": ["admin"]}}`.
    /// Attribute keys and values must be short lowercase tokens.
    #[serde(default)]
    pub role_map: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    /// Require `email_verified` when an `email` claim is present.
    /// Defaults to `true`.
    #[serde(default = "default_true")]
    pub require_verified_email: bool,
}

fn default_true() -> bool {
    true
}

/// The Keycloak claims beyond the standard set that we read. Every field
/// is optional so a token minted without the `email` / `profile` scopes
/// still verifies (the engine's own default extras would reject it).
#[derive(Debug, Clone, Deserialize)]
pub struct KeycloakExtra {
    /// `email` claim.
    #[serde(default)]
    pub email: Option<String>,
    /// `email_verified` claim.
    #[serde(default)]
    pub email_verified: bool,
    /// `name` claim (display name).
    #[serde(default)]
    pub name: Option<String>,
    /// `preferred_username` claim (fallback display name).
    #[serde(default)]
    pub preferred_username: Option<String>,
    /// `sid` claim — the Keycloak session id.
    #[serde(default)]
    pub sid: Option<String>,
}

impl KeycloakSettings {
    /// Read `<PREFIX>_KEYCLOAK_*` from the process environment, the way
    /// every service configures itself (`PREFIX` is the service's entity
    /// prefix, e.g. `ORGANIZATION`). See [`settings_from_lookup`].
    ///
    /// # Errors
    ///
    /// [`VerifyError::Keycloak`] when Keycloak is configured (the URL is
    /// set) but a variable is malformed.
    pub fn from_env(prefix: &str) -> Result<Option<Self>, VerifyError> {
        settings_from_lookup(prefix, &|name| std::env::var(name).ok())
    }
}

/// Build [`KeycloakSettings`] from `<PREFIX>_KEYCLOAK_*` variables read
/// through `lookup` (a closure so tests need not mutate the process
/// environment).
///
/// | Variable | Meaning |
/// |---|---|
/// | `<PREFIX>_KEYCLOAK_URL` | server base URL; **unset or blank ⇒ Keycloak disabled** (`Ok(None)`) |
/// | `<PREFIX>_KEYCLOAK_REALM` | realm name (required once the URL is set) |
/// | `<PREFIX>_KEYCLOAK_AUDIENCES` | comma-separated accepted `aud` values (required) |
/// | `<PREFIX>_KEYCLOAK_ROLE_MAP_FILE` / `_ROLE_MAP` | JSON role → attributes map; the file wins |
/// | `<PREFIX>_KEYCLOAK_REQUIRE_VERIFIED_EMAIL` | `false`/`0`/`no`/`off` disables the check (default on) |
///
/// # Errors
///
/// [`VerifyError::Keycloak`] when the URL is set but the realm or
/// audiences are missing, the role-map file is unreadable, or the role
/// map is not valid JSON of the documented shape. Settings are also
/// [validated](KeycloakSettings::validate) (HTTPS, tokens).
pub fn settings_from_lookup(
    prefix: &str,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Result<Option<KeycloakSettings>, VerifyError> {
    let var = |suffix: &str| {
        lookup(&format!("{prefix}_KEYCLOAK_{suffix}"))
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    };
    let Some(url) = var("URL") else {
        return Ok(None);
    };
    let realm = var("REALM")
        .ok_or_else(|| VerifyError::Keycloak(format!("{prefix}_KEYCLOAK_REALM is not set")))?;
    let audiences: Vec<String> = var("AUDIENCES")
        .map(|v| {
            v.split(',')
                .map(str::trim)
                .filter(|a| !a.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let role_map_json = match var("ROLE_MAP_FILE") {
        Some(path) => Some(std::fs::read_to_string(&path).map_err(|e| {
            VerifyError::Keycloak(format!("{prefix}_KEYCLOAK_ROLE_MAP_FILE {path:?}: {e}"))
        })?),
        None => var("ROLE_MAP"),
    };
    let role_map = match role_map_json {
        Some(json) => serde_json::from_str(&json)
            .map_err(|e| VerifyError::Keycloak(format!("{prefix}_KEYCLOAK_ROLE_MAP: {e}")))?,
        None => BTreeMap::new(),
    };
    let require_verified_email = var("REQUIRE_VERIFIED_EMAIL").is_none_or(|v| {
        !matches!(
            v.to_ascii_lowercase().as_str(),
            "false" | "0" | "no" | "off"
        )
    });
    let settings = KeycloakSettings {
        url,
        realm,
        audiences,
        role_map,
        require_verified_email,
    };
    settings.validate()?;
    Ok(Some(settings))
}

/// Verifies Keycloak access tokens and yields [`Claims`].
pub struct KeycloakVerifier {
    layer: KeycloakAuthLayer<String, KeycloakExtra>,
    issuer: String,
    settings: KeycloakSettings,
}

impl std::fmt::Debug for KeycloakVerifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeycloakVerifier")
            .field("issuer", &self.issuer)
            .field("audiences", &self.settings.audiences)
            .finish_non_exhaustive()
    }
}

/// `true` for `localhost`, `127.0.0.1` and `::1` hosts, which may use
/// plain HTTP for local development.
fn is_loopback(url: &Url) -> bool {
    matches!(
        url.host_str(),
        Some("localhost" | "127.0.0.1" | "[::1]" | "::1")
    )
}

/// A short lowercase ABAC token: the shape attribute keys and values take
/// everywhere else in the family.
fn is_token(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'-' | b'.')
        })
}

impl KeycloakSettings {
    /// The issuer a token must carry: `{url}/realms/{realm}` with no
    /// trailing slash on `url`.
    #[must_use]
    pub fn expected_issuer(&self) -> String {
        format!("{}/realms/{}", self.url.trim_end_matches('/'), self.realm)
    }

    /// Validate the settings without any I/O.
    ///
    /// # Errors
    ///
    /// [`VerifyError::Keycloak`] for a malformed or non-HTTPS URL, an empty
    /// realm, no audiences, or a role map whose attribute keys or values
    /// are not short lowercase tokens.
    pub fn validate(&self) -> Result<Url, VerifyError> {
        let url = Url::parse(&self.url)
            .map_err(|e| VerifyError::Keycloak(format!("invalid url {:?}: {e}", self.url)))?;
        if url.scheme() != "https" && !(url.scheme() == "http" && is_loopback(&url)) {
            return Err(VerifyError::Keycloak(format!(
                "url must be https (loopback excepted), got {:?}",
                self.url
            )));
        }
        if self.realm.trim().is_empty() {
            return Err(VerifyError::Keycloak("realm must not be empty".into()));
        }
        if self.audiences.is_empty() || self.audiences.iter().any(|a| a.trim().is_empty()) {
            return Err(VerifyError::Keycloak(
                "at least one non-empty audience is required".into(),
            ));
        }
        for (role, attrs) in &self.role_map {
            for (key, values) in attrs {
                if !is_token(key) || values.iter().any(|v| !is_token(v)) {
                    return Err(VerifyError::Keycloak(format!(
                        "role_map[{role:?}]: attribute {key:?} and its values must be short lowercase tokens"
                    )));
                }
            }
        }
        Ok(url)
    }
}

impl KeycloakVerifier {
    /// Build a verifier and start OIDC discovery in the background. Must
    /// be called from within a Tokio runtime.
    ///
    /// Until discovery succeeds, every token is rejected (fail-closed);
    /// the service still boots.
    ///
    /// # Errors
    ///
    /// [`VerifyError::Keycloak`] when [`KeycloakSettings::validate`] fails.
    pub fn new(settings: KeycloakSettings) -> Result<Self, VerifyError> {
        let server = settings.validate()?;
        let instance = KeycloakAuthInstance::new(
            KeycloakConfig::builder()
                .server(server)
                .realm(settings.realm.clone())
                .build(),
        );
        let layer = KeycloakAuthLayer::<String, KeycloakExtra>::builder()
            .instance(instance)
            .expected_audiences(settings.audiences.clone())
            .build();
        Ok(Self {
            layer,
            issuer: settings.expected_issuer(),
            settings,
        })
    }

    /// Verify a Keycloak access token (signature, audience, expiry via the
    /// engine; issuer and verified email here) and map it onto [`Claims`].
    ///
    /// # Errors
    ///
    /// [`VerifyError::Keycloak`] for any engine rejection (malformed,
    /// bad signature, expired, wrong audience, realm unreachable) and
    /// [`VerifyError::Claim`] for a wrong issuer or an unverified email.
    pub async fn verify(&self, token: &str) -> Result<Claims, VerifyError> {
        let (_, kt) = self
            .layer
            .validate_raw_token(token)
            .await
            .map_err(|e| VerifyError::Keycloak(e.to_string()))?;
        claims_from_token(&kt, &self.settings)
    }
}

/// Map a verified [`KeycloakToken`] onto [`Claims`], applying the issuer
/// and verified-email checks and the role map. Pure: no I/O.
///
/// # Errors
///
/// [`VerifyError::Claim`] for a wrong issuer or an unverified email.
pub fn claims_from_token(
    token: &KeycloakToken<String, KeycloakExtra>,
    settings: &KeycloakSettings,
) -> Result<Claims, VerifyError> {
    let expected_issuer = settings.expected_issuer();
    if token.issuer != expected_issuer {
        return Err(VerifyError::Claim(format!(
            "issuer {:?} is not {expected_issuer:?}",
            token.issuer
        )));
    }
    if settings.require_verified_email
        && token.extra.email.as_deref().is_some_and(|e| !e.is_empty())
        && !token.extra.email_verified
    {
        return Err(VerifyError::Claim("email is not verified".into()));
    }

    let mut attrs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for role in &token.roles {
        let key = match role {
            KeycloakRole::Realm { role } => role.clone(),
            KeycloakRole::Client { client, role } => format!("{client}:{role}"),
        };
        if let Some(mapped) = settings.role_map.get(&key) {
            for (attr, values) in mapped {
                attrs
                    .entry(attr.clone())
                    .or_default()
                    .extend(values.iter().cloned());
            }
        }
    }

    // The configured audience the token actually carried (the engine has
    // already required at least one); fall back to the first configured.
    let aud = settings
        .audiences
        .iter()
        .find(|a| token.audience.contains(a))
        .or_else(|| settings.audiences.first())
        .cloned()
        .unwrap_or_default();

    Ok(Claims {
        sub: token.subject.clone(),
        email: token.extra.email.clone().unwrap_or_default(),
        name: token
            .extra
            .name
            .clone()
            .or_else(|| token.extra.preferred_username.clone())
            .unwrap_or_default(),
        iss: token.issuer.clone(),
        aud,
        exp: token.expires_at.unix_timestamp(),
        iat: token.issued_at.unix_timestamp(),
        nbf: None,
        sid: token
            .extra
            .sid
            .clone()
            .unwrap_or_else(|| token.jwt_id.clone()),
        scope: Vec::new(),
        roles: Vec::new(),
        attrs: attrs
            .into_iter()
            .map(|(k, v)| (k, v.into_iter().collect()))
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> KeycloakSettings {
        serde_json::from_value(serde_json::json!({
            "url": "https://kc.example.com/",
            "realm": "mxi",
            "audiences": ["mxi-api"],
            "role_map": {
                "editor": { "access": ["write"] },
                "root": { "access": ["admin"], "svc": ["true"] },
                "mxi-web:viewer": { "dept": ["care"] }
            }
        }))
        .unwrap()
    }

    fn token(roles: Vec<KeycloakRole<String>>) -> KeycloakToken<String, KeycloakExtra> {
        KeycloakToken {
            expires_at: time::OffsetDateTime::from_unix_timestamp(2_000_000_000).unwrap(),
            issued_at: time::OffsetDateTime::from_unix_timestamp(1_999_999_000).unwrap(),
            jwt_id: "jti-1".into(),
            issuer: "https://kc.example.com/realms/mxi".into(),
            audience: vec!["account".into(), "mxi-api".into()],
            subject: "0c4f1e2a-0000-4000-8000-000000000001".into(),
            authorized_party: "mxi-web".into(),
            roles,
            extra: KeycloakExtra {
                email: Some("a@example.com".into()),
                email_verified: true,
                name: Some("Ada".into()),
                preferred_username: Some("ada".into()),
                sid: Some("sess-9".into()),
            },
        }
    }

    fn realm(r: &str) -> KeycloakRole<String> {
        KeycloakRole::Realm { role: r.into() }
    }

    #[test]
    fn issuer_has_no_double_slash() {
        assert_eq!(
            settings().expected_issuer(),
            "https://kc.example.com/realms/mxi"
        );
    }

    #[test]
    fn maps_realm_and_client_roles_to_attrs() {
        let t = token(vec![
            realm("editor"),
            realm("root"),
            KeycloakRole::Client {
                client: "mxi-web".into(),
                role: "viewer".into(),
            },
        ]);
        let c = claims_from_token(&t, &settings()).unwrap();
        assert_eq!(
            c.attrs["access"],
            vec!["admin".to_string(), "write".to_string()]
        );
        assert_eq!(c.attrs["svc"], vec!["true".to_string()]);
        assert_eq!(c.attrs["dept"], vec!["care".to_string()]);
        assert_eq!(
            (c.sub.as_str(), c.aud.as_str(), c.sid.as_str()),
            ("0c4f1e2a-0000-4000-8000-000000000001", "mxi-api", "sess-9")
        );
        assert_eq!(
            (c.email.as_str(), c.name.as_str(), c.exp),
            ("a@example.com", "Ada", 2_000_000_000)
        );
    }

    #[test]
    fn unmapped_roles_grant_nothing() {
        let t = token(vec![realm("admin"), realm("offline_access")]);
        let c = claims_from_token(&t, &settings()).unwrap();
        assert!(
            c.attrs.is_empty(),
            "an unmapped role must not become an attribute"
        );
    }

    #[test]
    fn wrong_issuer_is_rejected() {
        let mut t = token(vec![]);
        t.issuer = "https://evil.example.com/realms/mxi".into();
        assert!(matches!(
            claims_from_token(&t, &settings()),
            Err(VerifyError::Claim(_))
        ));
    }

    #[test]
    fn unverified_email_is_rejected_unless_disabled() {
        let mut t = token(vec![]);
        t.extra.email_verified = false;
        assert!(matches!(
            claims_from_token(&t, &settings()),
            Err(VerifyError::Claim(_))
        ));
        let mut s = settings();
        s.require_verified_email = false;
        assert!(claims_from_token(&t, &s).is_ok());
    }

    #[test]
    fn a_token_without_email_is_accepted() {
        let mut t = token(vec![]);
        t.extra.email = None;
        t.extra.email_verified = false;
        assert_eq!(claims_from_token(&t, &settings()).unwrap().email, "");
    }

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: BTreeMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        move |k| map.get(k).cloned()
    }

    #[test]
    fn from_env_is_none_when_unconfigured_or_blank() {
        assert!(settings_from_lookup("ORG", &env(&[])).unwrap().is_none());
        assert!(
            settings_from_lookup("ORG", &env(&[("ORG_KEYCLOAK_URL", "  ")]))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn from_env_reads_every_variable() {
        let s = settings_from_lookup(
            "ORG",
            &env(&[
                ("ORG_KEYCLOAK_URL", "https://kc.example.com"),
                ("ORG_KEYCLOAK_REALM", "mxi"),
                ("ORG_KEYCLOAK_AUDIENCES", "mxi-api, mxi-web ,"),
                (
                    "ORG_KEYCLOAK_ROLE_MAP",
                    r#"{"editor":{"access":["write"]}}"#,
                ),
                ("ORG_KEYCLOAK_REQUIRE_VERIFIED_EMAIL", "off"),
            ]),
        )
        .unwrap()
        .unwrap();
        assert_eq!(s.audiences, vec!["mxi-api", "mxi-web"]);
        assert_eq!(s.role_map["editor"]["access"], vec!["write"]);
        assert!(!s.require_verified_email);
    }

    #[test]
    fn from_env_fails_closed_on_a_half_configured_realm() {
        let base = [("ORG_KEYCLOAK_URL", "https://kc.example.com")];
        assert!(
            settings_from_lookup("ORG", &env(&base)).is_err(),
            "no realm"
        );
        let mut with_realm = base.to_vec();
        with_realm.push(("ORG_KEYCLOAK_REALM", "mxi"));
        assert!(
            settings_from_lookup("ORG", &env(&with_realm)).is_err(),
            "no audiences"
        );
        with_realm.push(("ORG_KEYCLOAK_AUDIENCES", "a"));
        with_realm.push(("ORG_KEYCLOAK_ROLE_MAP", "not json"));
        assert!(
            settings_from_lookup("ORG", &env(&with_realm)).is_err(),
            "bad role map"
        );
    }

    #[test]
    fn settings_validation_is_fail_closed() {
        let mut s = settings();
        s.url = "http://kc.example.com".into();
        assert!(s.validate().is_err(), "plain http to a remote host");
        s.url = "http://localhost:8080".into();
        assert!(s.validate().is_ok(), "loopback http is allowed");
        let mut s = settings();
        s.audiences.clear();
        assert!(s.validate().is_err(), "no audiences");
        let mut s = settings();
        s.realm = " ".into();
        assert!(s.validate().is_err(), "blank realm");
        let mut s = settings();
        s.role_map.insert(
            "x".into(),
            [("Access".to_string(), vec!["write".to_string()])].into(),
        );
        assert!(s.validate().is_err(), "uppercase attribute key");
    }
}
