//! OIDC identity federation (EV-2): configuration loading and claim
//! mapping. Design: `agents/share/authentication-sessions.md` §7a.
//!
//! Magic link stays the default; this whole feature is opt-in per
//! deployment, gated at two levels: the Cargo feature `oidc` (a
//! deployment that never federates pulls in no extra HTTP/JWT stack
//! at all), and — once compiled in — [`OidcConfig::from_env`] returns
//! `None` unless the deployment has actually set
//! [`ISSUER_URL_ENV`], so a build with the feature compiled in still
//! behaves exactly as before until configured.

use std::collections::BTreeMap;

/// The `IdP`'s issuer URL, used for OIDC discovery
/// (`{issuer}/.well-known/openid-configuration`). Required to enable
/// federation at all.
pub const ISSUER_URL_ENV: &str = "AUTH_OIDC_ISSUER_URL";
/// This service's registered `OAuth2` client id at the `IdP`.
pub const CLIENT_ID_ENV: &str = "AUTH_OIDC_CLIENT_ID";
/// This service's client secret, inline. [`CLIENT_SECRET_FILE_ENV`]
/// takes precedence when both are set (file-sourcing outranks the
/// environment, family-wide).
pub const CLIENT_SECRET_ENV: &str = "AUTH_OIDC_CLIENT_SECRET";
/// Path to a file holding the client secret.
pub const CLIENT_SECRET_FILE_ENV: &str = "AUTH_OIDC_CLIENT_SECRET_FILE";
/// This service's own callback URL, registered with the `IdP`
/// (`.../api/auth/oidc/callback`).
pub const REDIRECT_URL_ENV: &str = "AUTH_OIDC_REDIRECT_URL";
/// Inline JSON claim-name → attribute-key map (e.g.
/// `{"groups":"dept","role":"access"}`).
/// [`CLAIM_MAP_FILE_ENV`] takes precedence when both are set.
pub const CLAIM_MAP_ENV: &str = "AUTH_OIDC_CLAIM_MAP";
/// Path to a file holding the same JSON shape as [`CLAIM_MAP_ENV`].
pub const CLAIM_MAP_FILE_ENV: &str = "AUTH_OIDC_CLAIM_MAP_FILE";
/// `1`/`true` ⇒ a first successful federated sign-in for an unknown
/// email auto-creates a passwordless account. Default (unset, or any
/// other value): **off** — the safer of the two documented leans in
/// §7a's open question, since auto-provisioning means the `IdP` now
/// controls account creation. A deployment opts into the more
/// convenient behaviour explicitly rather than inheriting it silently.
pub const JIT_PROVISIONING_ENV: &str = "AUTH_OIDC_JIT_PROVISIONING";

/// Resolved OIDC federation configuration for one deployment. Built
/// once per request from the environment (cheap: a handful of env
/// reads, no I/O) rather than cached — a deployment that edits its
/// federation config between requests (e.g. rotating a client secret)
/// takes effect immediately, matching the ABAC policy's own hot-reload
/// posture rather than the boot-time-only key-fetch posture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OidcConfig {
    /// The `IdP`'s issuer URL.
    pub issuer_url: String,
    /// This service's client id at the `IdP`.
    pub client_id: String,
    /// This service's client secret.
    pub client_secret: String,
    /// This service's own callback URL.
    pub redirect_url: String,
    /// Claim name → attribute key.
    pub claim_map: BTreeMap<String, String>,
    /// Whether an unknown email auto-provisions on first sign-in.
    pub jit_provisioning: bool,
}

impl OidcConfig {
    /// Load from the environment. `None` when [`ISSUER_URL_ENV`],
    /// [`CLIENT_ID_ENV`], the client secret, or [`REDIRECT_URL_ENV`]
    /// is unset — federation needs all four to mean anything, and a
    /// partially-configured deployment should behave as unconfigured
    /// (never a half-working federation flow) rather than erroring at
    /// boot for a feature nobody asked to enable yet.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let issuer_url = std::env::var(ISSUER_URL_ENV)
            .ok()
            .filter(|v| !v.is_empty())?;
        let client_id = std::env::var(CLIENT_ID_ENV)
            .ok()
            .filter(|v| !v.is_empty())?;
        let client_secret = client_secret_from_env()?;
        let redirect_url = std::env::var(REDIRECT_URL_ENV)
            .ok()
            .filter(|v| !v.is_empty())?;
        let claim_map = claim_map_from_env().unwrap_or_default();
        let jit_provisioning = std::env::var(JIT_PROVISIONING_ENV)
            .ok()
            .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
        Some(Self {
            issuer_url,
            client_id,
            client_secret,
            redirect_url,
            claim_map,
            jit_provisioning,
        })
    }
}

fn client_secret_from_env() -> Option<String> {
    if let Ok(path) = std::env::var(CLIENT_SECRET_FILE_ENV) {
        return std::fs::read_to_string(path)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
    }
    std::env::var(CLIENT_SECRET_ENV)
        .ok()
        .filter(|v| !v.is_empty())
}

fn claim_map_from_env() -> Option<BTreeMap<String, String>> {
    let raw = if let Ok(path) = std::env::var(CLAIM_MAP_FILE_ENV) {
        std::fs::read_to_string(path).ok()?
    } else {
        std::env::var(CLAIM_MAP_ENV).ok()?
    };
    serde_json::from_str(&raw).ok()
}

/// Map an ID token's extra claims into an ABAC attrs map via
/// `claim_map` (claim name → attribute key), every mapped key/value
/// pushed through **exactly** the same validation an operator's CLI
/// or admin-API attribute assignment goes through
/// (`crate::tasks::attributes::{validate_key, validate_value,
/// vocabulary}`) — an `IdP` claim is no more trustworthy than an
/// operator's typo (§7a).
///
/// A claim absent from the token, or whose value is not a string or
/// array of strings, or whose mapped key/values fail validation or
/// the configured vocabulary, is **skipped, not fatal**: the sign-in
/// still succeeds with whatever attributes did map cleanly, and every
/// skip is named in the returned list so a misconfigured mapping is
/// visible in the audit trail rather than silently inert.
#[must_use]
pub fn map_claims(
    claims_json: &serde_json::Value,
    claim_map: &BTreeMap<String, String>,
) -> (BTreeMap<String, Vec<String>>, Vec<String>) {
    let mut attrs = BTreeMap::new();
    let mut skipped = Vec::new();
    for (claim_name, attr_key) in claim_map {
        let Some(value) = claims_json.get(claim_name) else {
            continue;
        };
        let values: Vec<String> = match value {
            serde_json::Value::String(s) => vec![s.clone()],
            serde_json::Value::Array(arr) => arr
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect(),
            _ => {
                skipped.push(format!("{claim_name}: not a string or an array of strings"));
                continue;
            }
        };
        if values.is_empty() {
            skipped.push(format!("{claim_name}: no usable string values"));
            continue;
        }
        if let Err(reason) = crate::tasks::attributes::validate_key(attr_key) {
            skipped.push(format!("{attr_key}: {reason}"));
            continue;
        }
        if let Some(reason) = values
            .iter()
            .find_map(|v| crate::tasks::attributes::validate_value(v).err())
        {
            skipped.push(format!("{attr_key}: {reason}"));
            continue;
        }
        if let Err(reason) = crate::tasks::attributes::vocabulary().check(attr_key, &values) {
            skipped.push(format!("{attr_key}: {reason}"));
            continue;
        }
        attrs.insert(attr_key.clone(), values);
    }
    (attrs, skipped)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_env_is_none_when_unconfigured() {
        // No test in this binary sets `AUTH_OIDC_ISSUER_URL`, and
        // `#![forbid(unsafe_code)]` rules out `remove_var` here to
        // force the point — this simply trusts the default test
        // environment leaves it unset, exactly as a deployment that
        // has not opted into federation does.
        assert!(OidcConfig::from_env().is_none());
    }

    #[test]
    fn map_claims_extracts_string_and_array_claims() {
        let claims = serde_json::json!({
            "dept": "cardiology",
            "groups": ["clinician", "on_call"],
            "sub": "user-123",
        });
        let mut map = BTreeMap::new();
        map.insert("dept".to_string(), "dept".to_string());
        map.insert("groups".to_string(), "access".to_string());
        let (attrs, skipped) = map_claims(&claims, &map);
        assert_eq!(attrs.get("dept"), Some(&vec!["cardiology".to_string()]));
        assert_eq!(
            attrs.get("access"),
            Some(&vec!["clinician".to_string(), "on_call".to_string()])
        );
        assert!(skipped.is_empty(), "{skipped:?}");
    }

    #[test]
    fn map_claims_skips_an_absent_claim_without_failing_the_rest() {
        let claims = serde_json::json!({ "dept": "cardiology" });
        let mut map = BTreeMap::new();
        map.insert("dept".to_string(), "dept".to_string());
        map.insert("missing_claim".to_string(), "access".to_string());
        let (attrs, skipped) = map_claims(&claims, &map);
        assert_eq!(attrs.len(), 1);
        assert!(
            skipped.is_empty(),
            "an absent claim is skipped silently, not reported"
        );
    }

    #[test]
    fn map_claims_skips_and_names_a_non_string_claim() {
        let claims = serde_json::json!({ "level": 5 });
        let mut map = BTreeMap::new();
        map.insert("level".to_string(), "access".to_string());
        let (attrs, skipped) = map_claims(&claims, &map);
        assert!(attrs.is_empty());
        assert_eq!(skipped.len(), 1);
        assert!(skipped[0].contains("level"));
    }

    #[test]
    fn map_claims_skips_and_names_an_invalid_attribute_key() {
        let claims = serde_json::json!({ "Weird Claim!": "value" });
        let mut map = BTreeMap::new();
        // An uppercase/space-containing mapped key is invalid per
        // `validate_key`'s own short-lowercase-token rule.
        map.insert("Weird Claim!".to_string(), "Not Lowercase".to_string());
        let (attrs, skipped) = map_claims(&claims, &map);
        assert!(attrs.is_empty());
        assert_eq!(skipped.len(), 1);
    }
}
