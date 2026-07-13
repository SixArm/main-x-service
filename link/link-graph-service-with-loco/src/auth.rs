//! Offline PASETO v4.public verification (spec T-19) + the `case ↔ person`
//! governance decision (spec T-16 / design §10).
//!
//! The aggregator verifies bearer tokens **offline** against the auth
//! service's published Ed25519 key set (the family pattern — see
//! [`agents/share/authentication-sessions.md`]), then uses the shared ABAC
//! engine to decide whether a caller may see **high-sensitivity** edges.
//!
//! The load-bearing invariant (design §10): a `subject_of` / `about` edge
//! asserts that a person is the subject of a government case, so an
//! unauthorised caller must **not learn the edge exists** — it is removed
//! from every read response ([`conceal_governed`]), never 403'd in a way
//! that reveals it. Governance is gated on `LINK_GRAPH_REQUIRE_AUTH` (the
//! family's default-off enforcement flag); a deployment handling real case
//! data MUST turn it on (and configure a policy that grants case-read only
//! to authorised callers).
//!
//! Config (all optional; unset ⇒ no keys / default policy / off):
//! - `LINK_GRAPH_PASETO_KEYS` — the Ed25519 key set (JSON) to verify against.
//! - `LINK_GRAPH_TOKEN_ISSUER` / `LINK_GRAPH_TOKEN_AUDIENCE` — expected
//!   claims (default `authentication-service` / `main-x-service`).
//! - `LINK_GRAPH_ABAC_POLICY` / `LINK_GRAPH_ABAC_POLICY_FILE` — the ABAC
//!   policy (else the built-in default: read-allow / mutation-deny).
//! - `LINK_GRAPH_REQUIRE_AUTH` — truthy activates governance concealment.
//!
//! [`agents/share/authentication-sessions.md`]: https://github.com/sixarm/main-x-service

use std::sync::OnceLock;

use authentication_verifier::{Action, Claims, Policy, Verifier};
use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum::http::{HeaderMap, StatusCode};
use entity_ref::{EdgeKind, Sensitivity};

/// Default token issuer expected in the `iss` claim.
const DEFAULT_ISSUER: &str = "authentication-service";
/// Default token audience expected in the `aud` claim.
const DEFAULT_AUDIENCE: &str = "main-x-service";
/// The resource entity a caller must be able to **read** (via the shared
/// ABAC policy) to see this crate's governed `subject_of` edges — the same
/// `case` gate the case service applies to the underlying record (§10).
const GOVERNED_ENTITY: &str = "case";
/// The aggregator's own entity, as seen by the blanket read guard's ABAC
/// decision (the `entity` pseudo-attribute in policy `when` clauses).
const GRAPH_ENTITY: &str = "link_graph";

/// Read an env var, falling back to `default` when unset/blank.
fn env_or(name: &str, default: &str) -> String {
    std::env::var(name)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

/// Truthy env flag (`1` / `true` / `yes` / `on`, case-insensitive).
fn parse_bool(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// Build the verifier from `LINK_GRAPH_PASETO_KEYS`; an unset/blank/invalid
/// key set yields an empty verifier that rejects every token (so the
/// service always boots — a missing key set fails closed, not open).
fn build_from_env() -> Verifier {
    let issuer = env_or("LINK_GRAPH_TOKEN_ISSUER", DEFAULT_ISSUER);
    let audience = env_or("LINK_GRAPH_TOKEN_AUDIENCE", DEFAULT_AUDIENCE);
    let keys = std::env::var("LINK_GRAPH_PASETO_KEYS")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .unwrap_or_else(|| serde_json::json!({ "keys": [] }));
    Verifier::from_paseto_keys_value(&keys, &issuer, &audience).unwrap_or_else(|_| {
        Verifier::from_paseto_keys_value(&serde_json::json!({ "keys": [] }), &issuer, &audience)
            .expect("empty key set always builds")
    })
}

/// The process-wide verifier, built once from the environment.
#[must_use]
pub fn verifier() -> &'static Verifier {
    static VERIFIER: OnceLock<Verifier> = OnceLock::new();
    VERIFIER.get_or_init(build_from_env)
}

/// Load the ABAC policy from `LINK_GRAPH_ABAC_POLICY[_FILE]`, else the
/// built-in default. A bad policy warns and falls back (the service always
/// boots).
fn policy_from_env() -> Policy {
    let source = std::env::var("LINK_GRAPH_ABAC_POLICY")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| {
            let path = std::env::var("LINK_GRAPH_ABAC_POLICY_FILE")
                .ok()
                .filter(|v| !v.trim().is_empty())?;
            std::fs::read_to_string(path.trim()).ok()
        });
    match source {
        Some(json) => Policy::from_json(&json).unwrap_or_else(|error| {
            tracing::warn!(%error, "ABAC policy invalid; using the built-in default policy");
            Policy::default_policy()
        }),
        None => Policy::default_policy(),
    }
}

/// The process-wide ABAC policy, built once from the environment.
#[must_use]
pub fn policy() -> &'static Policy {
    static POLICY: OnceLock<Policy> = OnceLock::new();
    POLICY.get_or_init(policy_from_env)
}

/// Whether governance concealment is active (`LINK_GRAPH_REQUIRE_AUTH`).
/// Off by default — behaviour-neutral until a deployment activates it.
#[must_use]
pub fn require_auth() -> bool {
    static REQUIRE: OnceLock<bool> = OnceLock::new();
    *REQUIRE
        .get_or_init(|| parse_bool(&std::env::var("LINK_GRAPH_REQUIRE_AUTH").unwrap_or_default()))
}

/// Extract + verify the bearer token from request headers. Pure (the
/// verifier is passed in), so it is unit-testable without the global.
///
/// # Errors
///
/// `401` when the `Authorization` header is missing, is not a bearer
/// token, or the token fails PASETO signature / issuer / audience / expiry.
pub fn bearer_claims(
    headers: &HeaderMap,
    verifier: &Verifier,
) -> Result<Claims, (StatusCode, String)> {
    let header = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "missing authorization header".to_string(),
        ))?;
    let token = header
        .strip_prefix("Bearer ")
        .or_else(|| header.strip_prefix("bearer "))
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "expected bearer token".to_string(),
        ))?;
    verifier
        .verify(token.trim())
        .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))
}

/// A read request's optional caller identity: `Some(claims)` when a valid
/// bearer token is present, `None` otherwise (never rejects — the read
/// endpoints are not blanket-gated in v1; governance is edge-level).
pub struct MaybeAuthUser(pub Option<Claims>);

impl MaybeAuthUser {
    /// The verified claims, if a valid token was presented.
    #[must_use]
    pub fn claims(&self) -> Option<&Claims> {
        self.0.as_ref()
    }
}

impl<S: Send + Sync> FromRequestParts<S> for MaybeAuthUser {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(bearer_claims(&parts.headers, verifier()).ok()))
    }
}

/// Is this edge kind **governed** (high-sensitivity — `subject_of`)? Such
/// edges are concealed from callers without case-read authorisation
/// (design §9/§10). Keyed on the registry's [`Sensitivity`], so a future
/// high-sensitivity kind is covered automatically.
#[must_use]
pub fn is_governed(kind: EdgeKind) -> bool {
    kind.sensitivity() == Sensitivity::High
}

/// The governance decision (design §10 / spec T-16): may this caller see
/// governed `subject_of` edges?
///
/// - Enforcement **off** ⇒ `true` (behaviour-neutral dev default).
/// - Enforcement **on** ⇒ only a caller whose verified claims the ABAC
///   policy grants `read` on `case` — the same gate the case service
///   applies to the underlying record. An unauthenticated caller (no
///   claims) is denied.
#[must_use]
pub fn may_see_governed(claims: Option<&Claims>) -> bool {
    if !require_auth() {
        return true;
    }
    match claims {
        Some(c) => policy().evaluate(c, Action::Read, GOVERNED_ENTITY).allowed,
        None => false,
    }
}

/// Remove governed (`subject_of`) edges from a response unless the caller
/// may see them — the concealment primitive (design §10). Pure over the
/// kind-token predicate, so it applies uniformly to `neighbors` / `edges`
/// rows and to `single-view` affiliations. `kind_of` maps each item to its
/// [`EdgeKind`] (`None` ⇒ an unparseable kind, kept — it cannot be a known
/// governed kind).
pub fn conceal_governed<T>(
    items: Vec<T>,
    may_see: bool,
    kind_of: impl Fn(&T) -> Option<EdgeKind>,
) -> Vec<T> {
    if may_see {
        return items;
    }
    items
        .into_iter()
        .filter(|it| kind_of(it).is_none_or(|k| !is_governed(k)))
        .collect()
}

/// Paths that stay public even when enforcement is on: the loco health /
/// ping probes and (once they land) the `OpenAPI` doc, Swagger UI, and
/// Prometheus metrics.
#[must_use]
pub fn is_public_path(path: &str) -> bool {
    path == "/_health"
        || path == "/_ping"
        || path == "/api-docs/openapi.json"
        || path.starts_with("/swagger-ui")
        || path == "/metrics.prom"
}

/// The blanket read guard (spec §9.4 / T-19): when enforcement is on,
/// every non-public request needs a valid bearer token (`401`) whose
/// `attrs` the ABAC policy grants `read` on the aggregator (`403`). The
/// service is read-only, so the action is always [`Action::Read`];
/// governed-edge concealment (§10) stacks on top of this in the handlers.
/// Pure (flag + verifier + policy passed in), so it is unit-testable
/// without the process globals.
///
/// # Errors
///
/// `401` for a missing/invalid token; `403` when the policy denies read.
pub fn enforce(
    require_auth: bool,
    path: &str,
    headers: &HeaderMap,
    verifier: &Verifier,
    policy: &Policy,
) -> Result<(), (StatusCode, String)> {
    if !require_auth || is_public_path(path) {
        return Ok(());
    }
    let claims = bearer_claims(headers, verifier)?;
    let decision = policy.evaluate(&claims, Action::Read, GRAPH_ENTITY);
    if decision.allowed {
        Ok(())
    } else {
        Err((StatusCode::FORBIDDEN, decision.reason))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn claims_with(attrs: &[(&str, &[&str])]) -> Claims {
        let attrs: BTreeMap<String, Vec<String>> = attrs
            .iter()
            .map(|(k, vs)| {
                (
                    (*k).to_string(),
                    vs.iter().map(ToString::to_string).collect(),
                )
            })
            .collect();
        Claims {
            sub: "11111111-1111-1111-1111-111111111111".into(),
            email: "a@example.com".into(),
            name: "A".into(),
            iss: DEFAULT_ISSUER.into(),
            aud: DEFAULT_AUDIENCE.into(),
            exp: 9_999_999_999,
            iat: 1,
            nbf: None,
            sid: "s".into(),
            scope: Vec::new(),
            roles: Vec::new(),
            attrs,
        }
    }

    #[test]
    fn is_governed_flags_only_high_sensitivity_kinds() {
        assert!(is_governed(EdgeKind::SubjectOf));
        assert!(!is_governed(EdgeKind::SameIdentity));
        assert!(!is_governed(EdgeKind::WorksAt));
        assert!(!is_governed(EdgeKind::EmployedBy));
    }

    #[test]
    fn conceal_drops_governed_only_when_not_permitted() {
        let kinds = vec![
            EdgeKind::WorksAt,
            EdgeKind::SubjectOf,
            EdgeKind::SameIdentity,
        ];
        // Permitted: nothing removed.
        let keep = conceal_governed(kinds.clone(), true, |k| Some(*k));
        assert_eq!(keep.len(), 3);
        // Not permitted: the subject_of edge is concealed.
        let hidden = conceal_governed(kinds, false, |k| Some(*k));
        assert_eq!(hidden.len(), 2);
        assert!(!hidden.contains(&EdgeKind::SubjectOf));
    }

    #[test]
    fn may_see_governed_default_policy_gates_on_authentication() {
        // Under the built-in default policy (read is allowed for any
        // authenticated caller), an authenticated caller may see governed
        // edges; an unauthenticated one may not. (A deployment tightens
        // this by configuring a policy that denies read/case unless a
        // case-access attribute is present.)
        // NOTE: require_auth() reads a process global; this asserts the
        // decision logic given claims, independent of the flag, via the
        // policy directly.
        let authed = claims_with(&[]);
        assert!(
            Policy::default_policy()
                .evaluate(&authed, Action::Read, GOVERNED_ENTITY)
                .allowed
        );
    }

    /// An empty (no-keys) verifier — rejects every token; enough to pin the
    /// flag-off / public-path / missing-token branches of `enforce`.
    fn empty_verifier() -> Verifier {
        Verifier::from_paseto_keys_value(
            &serde_json::json!({ "keys": [] }),
            DEFAULT_ISSUER,
            DEFAULT_AUDIENCE,
        )
        .unwrap()
    }

    #[test]
    fn enforce_allows_when_off_or_on_a_public_path() {
        let (v, p, empty) = (empty_verifier(), Policy::default_policy(), HeaderMap::new());
        // Flag off ⇒ allow (dev default), even a protected path, no token.
        assert!(enforce(false, "/api/edges", &empty, &v, &p).is_ok());
        // Flag on but a public path ⇒ allow without a token.
        assert!(enforce(true, "/_health", &empty, &v, &p).is_ok());
    }

    #[test]
    fn enforce_on_rejects_a_protected_path_without_a_token() {
        let (v, p, empty) = (empty_verifier(), Policy::default_policy(), HeaderMap::new());
        let err = enforce(true, "/api/edges", &empty, &v, &p).unwrap_err();
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);
    }

    /// SEC-B11: `/api/health/freshness` is an operational liveness oracle,
    /// **not** a public health probe — it must stay behind the blanket guard
    /// so a future edit can't silently expose consumer-lag telemetry. It is
    /// not on the public allow-list, and with enforcement on it needs a token.
    #[test]
    fn freshness_is_guarded_not_public() {
        assert!(
            !is_public_path("/api/health/freshness"),
            "freshness must not be a public path"
        );
        let (v, p, empty) = (empty_verifier(), Policy::default_policy(), HeaderMap::new());
        let err = enforce(true, "/api/health/freshness", &empty, &v, &p).unwrap_err();
        assert_eq!(
            err.0,
            StatusCode::UNAUTHORIZED,
            "freshness requires a token when enforcement is on"
        );
    }
}
