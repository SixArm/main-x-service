//! Bearer-token authentication for the patient-flow API.
//!
//! [`AuthUser`] is an Axum extractor that pulls `Authorization: Bearer
//! <paseto>`, verifies the PASETO `v4.public` (Ed25519) signature, issuer,
//! audience and expiry against the
//! [authentication-service](../../../authentication/authentication-service-with-loco)
//! published key set, and yields the verified [`Claims`]. Verification is
//! stateless and offline — no database hit, no introspection call — so any
//! handler can require authentication by taking an `AuthUser` argument, and
//! a handler that wants the caller identity *when present* (e.g. to stamp
//! an audit `actor`) takes [`MaybeAuthUser`] instead.
//!
//! ## Key source
//!
//! The process-wide [`verifier`] is seeded once at boot ([`init`] is
//! called from `App::after_routes`, before the app serves traffic) and
//! built from the environment:
//!
//! - `PATIENT_FLOW_PASETO_KEYS_URL` — optional URL of the auth service's
//!   published key set (`/.well-known/paseto-keys`). Set (non-blank) ⇒
//!   the key set is fetched over HTTP **once at boot** via
//!   [`Verifier::from_paseto_keys_url`]; on success the fetched key set
//!   wins over `PATIENT_FLOW_PASETO_KEYS` (`tracing::info!`), on failure the
//!   service logs a `tracing::warn!` and falls back to the env path
//!   below — the service always boots. The key set is then **re-fetched
//!   periodically** ([`spawn_key_refresh`]) so a key rotation is picked
//!   up without a restart (interval `PATIENT_FLOW_PASETO_KEYS_REFRESH_SECS`,
//!   default 1 h; `0` disables; keeps the current keys on a failed
//!   fetch).
//! - `PATIENT_FLOW_PASETO_KEYS` — the Ed25519 key set (JSON, OKP/Ed25519
//!   JWK form) the auth service publishes at `/.well-known/paseto-keys`.
//!   Absent ⇒ an empty key set, so every token is rejected (the service
//!   still boots).
//! - `PATIENT_FLOW_TOKEN_ISSUER` — expected `iss` (default
//!   `authentication-service`).
//! - `PATIENT_FLOW_TOKEN_AUDIENCE` — expected `aud` (default
//!   `main-x-service`).
//!
//! ## Blanket enforcement
//!
//! When `PATIENT_FLOW_REQUIRE_AUTH` is truthy (`1`/`true`/`yes`/`on`,
//! case-insensitive), the [`enforce`] decision — wired as an Axum
//! middleware layer in `src/app.rs` — requires a valid bearer token on
//! every route except the public health/ping, OpenAPI/Swagger, and
//! Prometheus metrics paths (see [`is_public_path`]). It is **off by
//! default**: unset/blank/junk
//! ⇒ today's behaviour, where the extractor is opt-in per handler and
//! the extractor path proves end-to-end verification.
//! Activation is an operations decision once the SSO token flow is live;
//! see `agents/share/authentication-sessions.md` and
//! `agents/share/jwt-enforcement.md` for the family-wide contract.
//!
//! ## Authorization (ABAC)
//!
//! Inside the same guard — so it applies only when `PATIENT_FLOW_REQUIRE_AUTH`
//! is on — a verified token is further checked against an
//! **attribute-based access control** policy per
//! `agents/share/authorization-attributes.md`: the request's action is
//! derived from the HTTP method plus this crate's destructive named
//! POSTs ([`DESTRUCTIVE_POST_SUFFIXES`]), and the shared engine in the
//! `authentication-verifier` crate evaluates the policy over the
//! token's `attrs` claim. The policy is read once per process
//! ([`policy`], built by [`policy_from_env`]) from `PATIENT_FLOW_ABAC_POLICY`
//! (inline JSON) or `PATIENT_FLOW_ABAC_POLICY_FILE` (path); unset or unparsable
//! ⇒ the built-in default policy (`svc=true` ⇒ everything;
//! `access=admin` ⇒ destructive+write; `access=write` ⇒ write;
//! otherwise read-only) — the service always boots. **401** =
//! missing/bad credential; **403** = valid credential, policy denied
//! (body carries the deciding rule). Stay data is personal data:
//! deployments can express e.g. department or purpose-of-use scoping
//! as configured policy rules over the same `attrs` claim, no code
//! change required.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use crate::models::_entities::{stays, wards};
use authentication_verifier::{
    Action, Claims, Policy, ReloadablePolicy, ReloadableVerifier, Verifier,
};
use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum::http::{HeaderMap, Method, StatusCode};

/// The resource entity this crate guards, as seen by ABAC policies
/// (the `entity` pseudo-attribute in rule `when` clauses).
pub const ENTITY: &str = "patient_flow";

/// Path suffixes of this crate's **destructive named POSTs** (per
/// `authorization-attributes.md` §2, fixed family-wide): record merge
/// (`POST /api/wards/merge`, live today), batch deduplicate, and bulk
/// import — the latter two listed ahead of the corresponding features
/// (dedup scan and bulk import are §13 work), so the guard is already
/// correct when they land. A POST whose path ends with one of these
/// derives [`Action::Destructive`] instead of [`Action::Write`].
pub const DESTRUCTIVE_POST_SUFFIXES: [&str; 3] = ["/merge", "/deduplicate", "/import"];

/// Default issuer expected in tokens (`iss`).
const DEFAULT_ISSUER: &str = "authentication-service";
/// Default audience expected in tokens (`aud`).
const DEFAULT_AUDIENCE: &str = "main-x-service";

/// The process-wide **hot-reloadable** token verifier. Lazily built from
/// the environment on first use ([`build_from_env`]); [`init`] swaps in
/// the boot-time fetched key set, and [`spawn_key_refresh`] swaps in a
/// re-fetched key set periodically (**key rotation without a restart**).
static VERIFIER: OnceLock<ReloadableVerifier> = OnceLock::new();

/// The process-wide reloadable token verifier. Read the active snapshot
/// with `verifier().current()` per request; it is swapped by [`init`]
/// (boot fetch) and [`spawn_key_refresh`] (periodic re-fetch).
#[must_use]
pub fn verifier() -> &'static ReloadableVerifier {
    VERIFIER.get_or_init(|| ReloadableVerifier::new(build_from_env()))
}

/// Seed the process-wide [`verifier`] before the app serves traffic
/// (called from `App::after_routes`). When `PATIENT_FLOW_PASETO_KEYS_URL` is set
/// (non-blank) the published key set is fetched over HTTP **once at boot**
/// and, on success, swapped in over the env-built one; on fetch failure,
/// or with the URL unset/blank, the env-built verifier stands, so the
/// service always boots. Idempotent enough to call once at boot.
pub async fn init() {
    if let Some(url) = std::env::var("PATIENT_FLOW_PASETO_KEYS_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
    {
        let issuer = env_or("PATIENT_FLOW_TOKEN_ISSUER", DEFAULT_ISSUER);
        let audience = env_or("PATIENT_FLOW_TOKEN_AUDIENCE", DEFAULT_AUDIENCE);
        // `fetch_or` keeps the env-built verifier as the fallback on a
        // failed fetch, so the service always has a usable verifier.
        let fetched = fetch_or(url.trim(), &issuer, &audience, build_from_env()).await;
        verifier().store(fetched);
    }
}

/// Default key-set refresh interval (seconds) when
/// `PATIENT_FLOW_PASETO_KEYS_REFRESH_SECS` is unset. One hour — key rotation is
/// infrequent, so a slow poll suffices.
const KEY_REFRESH_DEFAULT_SECS: u64 = 3600;

/// Spawn a background task that periodically re-fetches the published
/// key set from `PATIENT_FLOW_PASETO_KEYS_URL` and swaps it into the live
/// [`verifier`], so a **key rotation** at the auth-service is picked up
/// **without restarting** this service. On a failed fetch it keeps the
/// current key set (a transient auth-service outage never locks callers
/// out). Interval from `PATIENT_FLOW_PASETO_KEYS_REFRESH_SECS` (default
/// [`KEY_REFRESH_DEFAULT_SECS`]); **`0` disables** the loop.
///
/// A no-op when `PATIENT_FLOW_PASETO_KEYS_URL` is unset (env-supplied keys have
/// nothing to re-fetch). Call once at boot (`app.rs::after_routes`).
pub fn spawn_key_refresh() {
    let Some(url) = std::env::var("PATIENT_FLOW_PASETO_KEYS_URL")
        .ok()
        .map(|u| u.trim().to_string())
        .filter(|u| !u.is_empty())
    else {
        return;
    };
    let secs = std::env::var("PATIENT_FLOW_PASETO_KEYS_REFRESH_SECS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(KEY_REFRESH_DEFAULT_SECS);
    if secs == 0 {
        return; // explicitly disabled
    }
    let issuer = env_or("PATIENT_FLOW_TOKEN_ISSUER", DEFAULT_ISSUER);
    let audience = env_or("PATIENT_FLOW_TOKEN_AUDIENCE", DEFAULT_AUDIENCE);
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(secs));
        ticker.tick().await; // the first tick is immediate — skip it
        loop {
            ticker.tick().await;
            match Verifier::from_paseto_keys_url(&url, &issuer, &audience).await {
                Ok(fetched) => {
                    tracing::info!(keys = fetched.key_count(), "refreshed PASETO key set");
                    verifier().store(fetched);
                }
                Err(error) => {
                    tracing::warn!(%error, "PASETO key-set refresh failed; keeping current keys");
                }
            }
        }
    });
    tracing::info!(
        secs,
        "polling PATIENT_FLOW_PASETO_KEYS_URL for key rotation"
    );
}

/// Build a verifier by fetching the published key set from `url`
/// ([`Verifier::from_paseto_keys_url`]); on success the fetched key set
/// wins (`tracing::info!`), on any fetch/parse failure the given
/// `fallback` verifier is returned after a `tracing::warn!` — never a
/// panic, so the caller always boots. Pure dependency injection (URL,
/// issuer, audience and fallback are all passed in), so it is testable
/// against a local HTTP listener without touching the process global.
pub async fn fetch_or(url: &str, issuer: &str, audience: &str, fallback: Verifier) -> Verifier {
    match Verifier::from_paseto_keys_url(url, issuer, audience).await {
        Ok(fetched) => {
            tracing::info!(
                url,
                keys = fetched.key_count(),
                "PASETO key set fetched over HTTP; fetched key set wins over the env key set"
            );
            fetched
        }
        Err(error) => {
            tracing::warn!(
                url,
                %error,
                "PASETO key set fetch failed; falling back to the env-configured key set"
            );
            fallback
        }
    }
}

/// Whether blanket `/api/*` enforcement is on, read once from
/// `PATIENT_FLOW_REQUIRE_AUTH` and cached. Off by default — see the
/// module docs and `agents/share/jwt-enforcement.md`. Mirrors
/// [`verifier`]: a process-wide `OnceLock` built from the environment.
#[must_use]
pub fn require_auth() -> bool {
    static REQUIRE_AUTH: OnceLock<bool> = OnceLock::new();
    *REQUIRE_AUTH
        .get_or_init(|| parse_bool(&std::env::var("PATIENT_FLOW_REQUIRE_AUTH").unwrap_or_default()))
}

/// Lenient boolean parse: `1`/`true`/`yes`/`on` (case-insensitive,
/// surrounding whitespace ignored) ⇒ `true`; everything else
/// (incl. empty) ⇒ `false`.
#[must_use]
pub fn parse_bool(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// Paths that stay public even when enforcement is on: health/ping, the
/// `OpenAPI` doc + Swagger UI, and the Prometheus metrics endpoint (so a
/// scraper needs no bearer token). Everything else requires a valid bearer
/// token.
fn is_public_path(path: &str) -> bool {
    path == "/_health"
        || path == "/_ping"
        || path == "/api-docs/openapi.json"
        || path.starts_with("/swagger-ui")
        || path == "/metrics.prom"
}

/// Derive the request's ABAC action from its HTTP method and path (per
/// `authorization-attributes.md` §2): `GET`/`HEAD`/`OPTIONS` ⇒ `Read`;
/// `DELETE` ⇒ `Delete`; a `POST` whose path ends with a
/// [`DESTRUCTIVE_POST_SUFFIXES`] entry ⇒ `Destructive`; every other
/// `POST`/`PUT`/`PATCH` (and any unrecognised method) ⇒ `Write`.
#[must_use]
pub fn derive_action(method: &Method, path: &str) -> Action {
    // SEC-G6: normalise a trailing slash before the destructive-suffix
    // check, so `POST /api/wards/merge/` (and `//`) is still classified as
    // `Destructive` rather than silently downgraded to `Write` — which would
    // let an `access=write` (non-admin) caller reach a destructive op.
    let path = path.trim_end_matches('/');
    match *method {
        Method::GET | Method::HEAD | Method::OPTIONS => Action::Read,
        Method::DELETE => Action::Delete,
        Method::POST
            if DESTRUCTIVE_POST_SUFFIXES
                .iter()
                .any(|suffix| path.ends_with(suffix)) =>
        {
            Action::Destructive
        }
        _ => Action::Write,
    }
}

/// Load the ABAC policy: `PATIENT_FLOW_ABAC_POLICY` (inline JSON) wins, then
/// `PATIENT_FLOW_ABAC_POLICY_FILE` (path to a JSON file), else the built-in
/// default policy. A present-but-unparsable policy (bad JSON, unknown
/// effect/action names, unreadable file) `tracing::warn!`s and falls
/// back to the default — the service always boots, matching the
/// key-fetch posture. Read once per process via [`policy`]; restart to
/// change.
#[must_use]
pub fn policy_from_env() -> Policy {
    let source = std::env::var("PATIENT_FLOW_ABAC_POLICY")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| {
            let path = std::env::var("PATIENT_FLOW_ABAC_POLICY_FILE")
                .ok()
                .filter(|v| !v.trim().is_empty())?;
            match std::fs::read_to_string(path.trim()) {
                Ok(contents) => Some(contents),
                Err(error) => {
                    tracing::warn!(%error, %path, "ABAC policy file unreadable; using the built-in default policy");
                    None
                }
            }
        });
    match source {
        Some(json) => Policy::from_json(&json).unwrap_or_else(|error| {
            tracing::warn!(%error, "ABAC policy JSON invalid; using the built-in default policy");
            Policy::default_policy()
        }),
        None => Policy::default_policy(),
    }
}

/// The process-wide **hot-reloadable** ABAC policy, initialised from
/// `PATIENT_FLOW_ABAC_POLICY` / `PATIENT_FLOW_ABAC_POLICY_FILE` (else the built-in
/// default). Read the active snapshot with `policy().current()` per
/// request; swap it at runtime with [`reload_policy`] (e.g. from the
/// policy-file watcher in `app.rs`) — **no restart needed**.
#[must_use]
pub fn policy() -> &'static ReloadablePolicy {
    static POLICY: OnceLock<ReloadablePolicy> = OnceLock::new();
    POLICY.get_or_init(|| ReloadablePolicy::new(policy_from_env()))
}

/// Re-read the ABAC policy from the environment (`PATIENT_FLOW_ABAC_POLICY` /
/// `PATIENT_FLOW_ABAC_POLICY_FILE`, same rules and default-fallback as
/// [`policy_from_env`]) and swap it into the live [`policy`] holder.
/// Called by the policy-file watcher (`app.rs`) when the file changes;
/// a malformed policy falls back to the built-in default (never leaves
/// the service unprotected) and warn-logs. New requests see the new
/// policy immediately; in-flight ones finish against their snapshot.
pub fn reload_policy() {
    policy().store(policy_from_env());
    tracing::info!("ABAC policy reloaded");
}

/// How often the policy-file watcher polls for a change (seconds).
const POLICY_WATCH_SECS: u64 = 15;

/// Spawn a background task that hot-reloads the ABAC policy when the
/// `PATIENT_FLOW_ABAC_POLICY_FILE` changes on disk — so operators can edit the
/// policy file and have it take effect without a restart. It polls the
/// file's mtime every [`POLICY_WATCH_SECS`] and calls [`reload_policy`]
/// on a change (mtime-poll, not an OS notify, to stay dependency-light).
///
/// A no-op when `PATIENT_FLOW_ABAC_POLICY_FILE` is unset — an inline
/// `PATIENT_FLOW_ABAC_POLICY` (or the built-in default) has nothing to watch,
/// and env vars do not change at runtime. Call once at boot
/// (`app.rs::after_routes`).
pub fn spawn_policy_watcher() {
    let Some(path) = std::env::var("PATIENT_FLOW_ABAC_POLICY_FILE")
        .ok()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
    else {
        return;
    };
    tokio::spawn(async move {
        let mut last = file_mtime(&path);
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(POLICY_WATCH_SECS));
        // The first tick fires immediately; skip it so we react only to
        // subsequent changes, not the initial state we already loaded.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            let now = file_mtime(&path);
            if now != last {
                last = now;
                reload_policy();
            }
        }
    });
    tracing::info!(
        secs = POLICY_WATCH_SECS,
        "watching PATIENT_FLOW_ABAC_POLICY_FILE for changes"
    );
}

/// The last-modified time of `path`, or `None` if it cannot be read
/// (missing file, permission error, or a platform without mtime).
fn file_mtime(path: &str) -> Option<std::time::SystemTime> {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
}

/// The blanket-enforcement decision: authentication, then ABAC
/// authorization. `Ok(())` ⇒ let the request through; `Err((401|403,
/// msg))` ⇒ reject. Pure: the caller passes the flag, method, path,
/// headers, verifier and policy, so it is fully unit-testable without
/// booting the app or a database.
///
/// # Errors
///
/// `401` when enforcement is on, the path is not public, and the request
/// carries no valid bearer token (missing/malformed/expired/tampered).
/// `403` when the token is valid but the ABAC policy denies the derived
/// action (the message names the deciding rule, per
/// `authorization-attributes.md` §5).
pub fn enforce(
    require_auth: bool,
    method: &Method,
    path: &str,
    headers: &HeaderMap,
    verifier: &Verifier,
    policy: &Policy,
) -> Result<(), (StatusCode, String)> {
    if !require_auth || is_public_path(path) {
        return Ok(());
    }
    let claims = bearer_claims(headers, verifier)?;
    let decision = policy.evaluate(&claims, derive_action(method, path), ENTITY);
    if decision.allowed {
        Ok(())
    } else {
        Err((StatusCode::FORBIDDEN, decision.reason))
    }
}

/// Derive the **record-level resource attributes** of a stored stay
/// for the ABAC decision (`authorization-attributes.md` §9). Keys a
/// policy matches with `resource.<key>`:
///
/// | Resource key | From | Example tokens |
/// |---|---|---|
/// | `resource.ward` | `Stay::ward_pid` | the ward's pid string |
/// | `resource.status` | `Stay::status` | `admitted`, `discharge_ready`, `discharged` |
/// | `resource.source` | `Stay::source` | `ed`, `elective`, `virtual_admission` |
///
/// A deployment can then write e.g. "allow read only when
/// `resource.ward` is one of the caller's `ward` attributes", or
/// "allow a **masked** read otherwise" (an `allow` rule with the
/// `mask` obligation) — entirely as policy, no code change.
#[must_use]
pub fn stay_resource_attrs(stay: &stays::Model) -> BTreeMap<String, Vec<String>> {
    let mut attrs = BTreeMap::new();
    if let Some(ward_pid) = stay.ward_pid {
        attrs.insert("ward".to_string(), vec![ward_pid.to_string()]);
    }
    attrs.insert("status".to_string(), vec![stay.status.clone()]);
    attrs.insert("source".to_string(), vec![stay.source.clone()]);
    attrs
}

/// Record-level resource attributes of a **ward** (the whiteboard
/// read): `resource.ward` (pid), `resource.ward_code` (lowercased
/// display code), `resource.kind`.
#[must_use]
pub fn ward_resource_attrs(ward: &wards::Model) -> BTreeMap<String, Vec<String>> {
    let mut attrs = BTreeMap::new();
    attrs.insert("ward".to_string(), vec![ward.pid.to_string()]);
    attrs.insert("ward_code".to_string(), vec![ward.code.to_lowercase()]);
    attrs.insert("kind".to_string(), vec![ward.kind.clone()]);
    attrs
}

/// The placeholder shown for a redacted patient name under the `mask`
/// obligation (corridor screens; spec `auth.md`).
pub const MASKED: &str = "\u{2022}\u{2022}\u{2022}";

/// Apply the `mask` obligation to a stay: redact the display name,
/// the alert chips, and the home-location note. Bed/ward state and
/// the pseudonymous URN remain visible (a domestic team member can
/// see a bed needs cleaning without seeing who was in it).
#[must_use]
pub fn mask_stay(mut stay: stays::Model) -> stays::Model {
    stay.display_name = MASKED.to_string();
    stay.alerts = serde_json::json!([]);
    stay.home_location_note = None;
    stay
}

/// Environment attributes for the current request, for the `env.*`
/// policy namespace (`authorization-attributes.md` §10). Derived here —
/// not in the pure engine — so the clock stays at the service edge and
/// the engine stays deterministic:
///
/// | Env key | Meaning |
/// |---|---|
/// | `env.hour` | Current UTC hour, `0`–`23`. |
/// | `env.after_hours` | `true` outside 08:00–17:59 UTC, else `false`. |
///
/// A deployment can then write e.g. "deny write when
/// `env.after_hours=true` unless `access=admin`". With no `env.*` rule
/// these attributes are inert.
#[must_use]
pub fn request_env_attrs() -> BTreeMap<String, Vec<String>> {
    use chrono::Timelike;
    env_attrs_at(chrono::Utc::now().hour())
}

/// Pure derivation of the environment attributes for a given UTC `hour`
/// (`0`–`23`), split out of [`request_env_attrs`] so it is unit-testable
/// without a clock.
#[must_use]
fn env_attrs_at(hour: u32) -> BTreeMap<String, Vec<String>> {
    let after_hours = !(8..18).contains(&hour);
    let mut env = BTreeMap::new();
    env.insert("hour".to_string(), vec![hour.to_string()]);
    env.insert("after_hours".to_string(), vec![after_hours.to_string()]);
    env
}

/// **Record-level** authorization for a handler that has loaded the
/// target record: evaluate the policy with the record's resource
/// attributes ([`stay_resource_attrs`] / [`ward_resource_attrs`]) and
/// the request's environment attributes
/// ([`request_env_attrs`]). A finer, second pass on top of the coarse
/// blanket guard — the guard already required a valid token and a
/// coarse (entity-level) allow before the handler ran; this refines the
/// decision with attributes of the *specific* record and the request
/// context.
///
/// Gated on the same `PATIENT_FLOW_REQUIRE_AUTH` flag as the blanket guard:
/// when enforcement is **off** this is a no-op (behaviour-neutral, no
/// authn/authz); when **on**, the blanket guard guarantees a token, so
/// absent claims here are a `401` fail-safe.
///
/// # Errors
///
/// `401` if enforcement is on but no verified claims are present (should
/// not happen behind the blanket guard — fail safe). `403` if the policy
/// denies the action given the record's / request's attributes (the
/// message names the deciding rule).
/// On an **allow**, returns the decision's **obligations** — advisory
/// instructions the handler must honour, e.g. `["mask"]` (return the
/// masked view). When enforcement is off the vec is empty (no authz).
pub fn authorize_record(
    caller: &MaybeAuthUser,
    action: Action,
    resource: &BTreeMap<String, Vec<String>>,
) -> Result<Vec<String>, (StatusCode, String)> {
    if !require_auth() {
        return Ok(Vec::new());
    }
    let claims = caller
        .claims()
        .ok_or((StatusCode::UNAUTHORIZED, "missing bearer token".to_string()))?;
    let decision = policy().current().evaluate_with_context(
        claims,
        action,
        ENTITY,
        resource,
        &request_env_attrs(),
    );
    if decision.allowed {
        Ok(decision.obligations)
    } else {
        Err((StatusCode::FORBIDDEN, decision.reason))
    }
}

/// Read env var `name`, treating unset/blank as absent and falling back
/// to `default`. Used for the issuer/audience so a blank value doesn't
/// override the sensible default.
fn env_or(name: &str, default: &str) -> String {
    std::env::var(name)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

/// Build the process-wide [`Verifier`] from the environment: issuer,
/// audience, and the published key set. A missing/blank/unparseable key
/// set yields an empty key set (every token rejected) so the service still
/// boots without credentials configured.
fn build_from_env() -> Verifier {
    let issuer = env_or("PATIENT_FLOW_TOKEN_ISSUER", DEFAULT_ISSUER);
    let audience = env_or("PATIENT_FLOW_TOKEN_AUDIENCE", DEFAULT_AUDIENCE);
    let keys = std::env::var("PATIENT_FLOW_PASETO_KEYS")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .unwrap_or_else(|| serde_json::json!({ "keys": [] }));
    Verifier::from_paseto_keys_value(&keys, &issuer, &audience)
        .unwrap_or_else(|_| empty_verifier(&issuer, &audience))
}

/// A verifier with no keys: rejects every token until a real key set is
/// configured. Infallible — an empty `keys` array always parses.
fn empty_verifier(issuer: &str, audience: &str) -> Verifier {
    let empty = serde_json::json!({ "keys": [] });
    Verifier::from_paseto_keys_value(&empty, issuer, audience).expect("empty key set always builds")
}

/// Extract and verify the bearer token from request headers. Pure (the
/// verifier is passed in), so it is unit-testable without the global.
///
/// # Errors
///
/// `401` when the `Authorization` header is missing, is not a bearer
/// token, or the token fails PASETO signature / issuer / audience / expiry
/// verification.
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

/// A request whose bearer token passed PASETO signature / issuer /
/// audience / expiry verification. The wrapped [`Claims`] identify the
/// caller (`sub` is the user `pid`). Taking this argument makes a handler
/// require authentication.
pub struct AuthUser(pub Claims);

/// Extracting an [`AuthUser`] verifies the request's bearer token against
/// the process-wide [`verifier`]; a missing/invalid token rejects with
/// `401` before the handler runs, so the type is the "require auth" gate.
impl<S: Send + Sync> FromRequestParts<S> for AuthUser {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Snapshot the current (hot-reloadable) verifier for this request.
        let verifier = verifier().current();
        bearer_claims(&parts.headers, &verifier).map(AuthUser)
    }
}

/// Like [`AuthUser`], but never rejects: yields `Some(claims)` when a
/// valid bearer token is present and `None` otherwise. Handlers use it to
/// stamp the caller identity (e.g. the audit `actor`) without requiring
/// authentication on the route.
pub struct MaybeAuthUser(pub Option<Claims>);

impl MaybeAuthUser {
    /// The caller's `sub` (user `pid`) if a valid token was presented.
    #[must_use]
    pub fn actor(&self) -> Option<&str> {
        self.0.as_ref().map(|c| c.sub.as_str())
    }

    /// The verified [`Claims`] if a valid token was presented, for the
    /// record-level authorization pass ([`authorize_record`]).
    #[must_use]
    pub fn claims(&self) -> Option<&Claims> {
        self.0.as_ref()
    }
}

/// Extracting a [`MaybeAuthUser`] never rejects: a valid token yields
/// `Some(claims)`, anything else `None`. Handlers use it to opportunistically
/// stamp the caller identity without requiring authentication.
impl<S: Send + Sync> FromRequestParts<S> for MaybeAuthUser {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let verifier = verifier().current();
        Ok(Self(bearer_claims(&parts.headers, &verifier).ok()))
    }
}

/// DB-free, fully in-process pins for token verification and the blanket
/// `enforce` decision. A throwaway Ed25519 key mints PASETO tokens and a
/// matching key set, so the whole verification path (valid / missing /
/// non-bearer / expired / tampered / empty-keys) and the on/off/public-path
/// enforcement matrix are exercised without the auth service or a database.
#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use ed25519_dalek::SigningKey;
    use rusty_paseto::core::{
        Footer, Key, Paseto, PasetoAsymmetricPrivateKey, Payload, Public, V4,
    };
    use sha2::{Digest, Sha256};

    /// Issuer the test tokens and verifier agree on.
    const ISSUER: &str = "authentication-service";
    /// Audience the test tokens and verifier agree on.
    const AUDIENCE: &str = "main-x-service";
    /// A throwaway Ed25519 seed, used only to mint test tokens and a
    /// matching key set in-process. Not a secret — never used in production.
    const SEED: [u8; 32] = [
        3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
        3, 3,
    ];

    /// Build a key set + matching `kid` from the test public key, the same
    /// way the auth service publishes it.
    fn test_keys_and_kid() -> (serde_json::Value, String) {
        let public = SigningKey::from_bytes(&SEED).verifying_key().to_bytes();
        let kid = URL_SAFE_NO_PAD.encode(Sha256::digest(public));
        let keys = serde_json::json!({
            "keys": [{
                "kty": "OKP", "crv": "Ed25519", "use": "sig",
                "kid": kid, "x": URL_SAFE_NO_PAD.encode(public),
            }]
        });
        (keys, kid)
    }

    /// Mint a signed PASETO `v4.public` token for the test identity with
    /// `kid` in the footer and `exp` set `exp_offset_secs` from a fixed
    /// `iat` (negative offsets produce an already-expired token).
    fn sign(kid: &str, exp_offset_secs: i64) -> String {
        sign_with_attrs(kid, exp_offset_secs, &[])
    }

    /// Like [`sign`], with the given ABAC subject attributes minted into
    /// the token's `attrs` claim (e.g. `&[("access", &["write"])]`).
    fn sign_with_attrs(kid: &str, exp_offset_secs: i64, attrs: &[(&str, &[&str])]) -> String {
        let iat: i64 = 1_700_000_000;
        let claims = Claims {
            sub: "11111111-1111-1111-1111-111111111111".into(),
            email: "alice@example.com".into(),
            name: "Alice".into(),
            iss: ISSUER.into(),
            aud: AUDIENCE.into(),
            exp: iat + exp_offset_secs,
            iat,
            nbf: None,
            sid: "test-sid".into(),
            scope: Vec::new(),
            roles: Vec::new(),
            attrs: attrs
                .iter()
                .map(|(key, values)| {
                    (
                        (*key).to_string(),
                        values.iter().map(ToString::to_string).collect(),
                    )
                })
                .collect(),
        };
        let keypair = SigningKey::from_bytes(&SEED).to_keypair_bytes();
        let key = Key::<64>::from(keypair);
        let private = PasetoAsymmetricPrivateKey::<V4, Public>::from(&key);
        let payload = serde_json::to_string(&claims).expect("serialize claims");
        let footer = format!(r#"{{"kid":"{kid}"}}"#);
        let mut builder = Paseto::<V4, Public>::builder();
        builder.set_payload(Payload::from(payload.as_str()));
        builder.set_footer(Footer::from(footer.as_str()));
        builder.try_sign(&private).expect("sign")
    }

    /// Wrap a token in a `HeaderMap` with `Authorization: Bearer <token>`.
    fn bearer(token: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(AUTHORIZATION, format!("Bearer {token}").parse().unwrap());
        h
    }

    /// A well-formed, in-date, correctly-signed token verifies and yields
    /// the expected claims.
    #[test]
    fn valid_token_yields_claims() {
        let (keys, kid) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let token = sign(&kid, 10_000_000_000);
        let claims = bearer_claims(&bearer(&token), &verifier).expect("valid token verifies");
        assert_eq!(claims.sub, "11111111-1111-1111-1111-111111111111");
        assert_eq!(claims.email, "alice@example.com");
    }

    /// No `Authorization` header ⇒ `401`.
    #[test]
    fn missing_header_is_401() {
        let (keys, _) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let err = bearer_claims(&HeaderMap::new(), &verifier).unwrap_err();
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);
    }

    /// A non-bearer scheme (e.g. `Basic`) ⇒ `401`.
    #[test]
    fn non_bearer_header_is_401() {
        let (keys, _) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let mut h = HeaderMap::new();
        h.insert(AUTHORIZATION, "Basic abc123".parse().unwrap());
        assert_eq!(
            bearer_claims(&h, &verifier).unwrap_err().0,
            StatusCode::UNAUTHORIZED
        );
    }

    /// A token whose `exp` is in the past ⇒ `401`.
    #[test]
    fn expired_token_is_401() {
        let (keys, kid) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let token = sign(&kid, -60);
        assert_eq!(
            bearer_claims(&bearer(&token), &verifier).unwrap_err().0,
            StatusCode::UNAUTHORIZED
        );
    }

    /// Flipping a token character breaks PASETO verification ⇒ `401`.
    #[test]
    fn tampered_token_is_401() {
        let (keys, kid) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let mut token = sign(&kid, 10_000_000_000);
        let last = token.pop().unwrap();
        token.push(if last == 'a' { 'b' } else { 'a' });
        assert_eq!(
            bearer_claims(&bearer(&token), &verifier).unwrap_err().0,
            StatusCode::UNAUTHORIZED
        );
    }

    /// A no-key verifier (the boot fallback) rejects even a valid token.
    #[test]
    fn empty_verifier_rejects_everything() {
        let verifier = empty_verifier(ISSUER, AUDIENCE);
        let (_, kid) = test_keys_and_kid();
        let token = sign(&kid, 10_000_000_000);
        assert_eq!(
            bearer_claims(&bearer(&token), &verifier).unwrap_err().0,
            StatusCode::UNAUTHORIZED
        );
    }

    /// `parse_bool` accepts the documented truthy set and rejects the
    /// rest (including empty, `0`, and junk).
    #[test]
    fn parse_bool_truthy_and_falsy() {
        for t in ["1", "true", "TRUE", "Yes", "on", " on ", "ON"] {
            assert!(parse_bool(t), "{t:?} should parse true");
        }
        for f in ["", " ", "0", "false", "no", "off", "junk", "2"] {
            assert!(!parse_bool(f), "{f:?} should parse false");
        }
    }

    /// The default policy the enforcement tests share (what an
    /// unconfigured service uses).
    fn policy() -> Policy {
        Policy::default_policy()
    }

    /// Enforcement off ⇒ a protected path passes with no token — for
    /// reads and mutations alike (no authn and no authz when the flag
    /// is off; today's default behaviour stays intact).
    #[test]
    fn enforce_off_allows_without_token() {
        let (keys, _) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let policy = policy();
        for method in [Method::GET, Method::POST, Method::DELETE] {
            assert!(
                enforce(
                    false,
                    &method,
                    "/api/wards",
                    &HeaderMap::new(),
                    &verifier,
                    &policy
                )
                .is_ok(),
                "{method} should pass with enforcement off"
            );
        }
    }

    /// SEC-G8 — the default-off **exposure pin**. With `PATIENT_FLOW_REQUIRE_AUTH`
    /// off (the shipped default), the most sensitive reads — the audit
    /// trail, patient locate, and a single stay's PII — are **open without
    /// a token**. This is by design
    /// (see `agents/share/security.md` §4), but it means **activation is a
    /// tracked release gate**: a deployment exposed to untrusted callers
    /// MUST set the flag before it is reachable. This test documents that
    /// exposure explicitly so flipping the default cannot happen silently.
    #[test]
    fn default_off_exposes_sensitive_reads_activation_is_a_release_gate() {
        let (keys, _) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let policy = policy();
        let no_token = HeaderMap::new();
        for path in [
            "/api/stays/0c4f1e2a-0000-4000-8000-000000000001", // a stay's PII
            "/api/locate/person:0c4f1e2a-0000-4000-8000-000000000001", // patient location
            "/api/audits/recent",                              // system-wide audit
            "/api/whiteboard/0c4f1e2a-0000-4000-8000-000000000002", // ward board
        ] {
            assert!(
                enforce(false, &Method::GET, path, &no_token, &verifier, &policy).is_ok(),
                "SEC-G8: with the flag OFF, {path} is open without a token (activation is the gate)"
            );
        }
    }

    /// Enforcement on ⇒ the public paths (health/ping, `OpenAPI`,
    /// Swagger UI, Prometheus metrics) still pass without a token.
    #[test]
    fn enforce_on_allows_public_paths() {
        let (keys, _) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let policy = policy();
        for path in [
            "/_health",
            "/_ping",
            "/api-docs/openapi.json",
            "/swagger-ui",
            "/swagger-ui/index.html",
            "/metrics.prom",
        ] {
            assert!(
                enforce(
                    true,
                    &Method::GET,
                    path,
                    &HeaderMap::new(),
                    &verifier,
                    &policy
                )
                .is_ok(),
                "{path} should be public"
            );
        }
    }

    /// Enforcement on, protected path, no token ⇒ `401`.
    #[test]
    fn enforce_on_protected_without_token_is_401() {
        let (keys, _) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let err = enforce(
            true,
            &Method::GET,
            "/api/wards",
            &HeaderMap::new(),
            &verifier,
            &policy(),
        )
        .unwrap_err();
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);
    }

    /// Enforcement on, protected path, valid token ⇒ a read passes.
    #[test]
    fn enforce_on_protected_with_valid_token_is_ok() {
        let (keys, kid) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let token = sign(&kid, 10_000_000_000);
        assert!(
            enforce(
                true,
                &Method::GET,
                "/api/wards",
                &bearer(&token),
                &verifier,
                &policy()
            )
            .is_ok()
        );
    }

    /// Enforcement on, protected path, expired token ⇒ `401`.
    #[test]
    fn enforce_on_protected_with_expired_token_is_401() {
        let (keys, kid) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let token = sign(&kid, -60);
        let err = enforce(
            true,
            &Method::GET,
            "/api/wards",
            &bearer(&token),
            &verifier,
            &policy(),
        )
        .unwrap_err();
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);
    }

    /// Enforcement on, protected path, tampered token ⇒ `401`.
    #[test]
    fn enforce_on_protected_with_tampered_token_is_401() {
        let (keys, kid) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let mut token = sign(&kid, 10_000_000_000);
        let last = token.pop().unwrap();
        token.push(if last == 'a' { 'b' } else { 'a' });
        let err = enforce(
            true,
            &Method::GET,
            "/api/wards",
            &bearer(&token),
            &verifier,
            &policy(),
        )
        .unwrap_err();
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);
    }

    /// Action derivation (`authorization-attributes.md` §2): safe
    /// methods read; DELETE deletes; the crate's destructive named
    /// POSTs (merge / deduplicate / import) are destructive, not
    /// write; every other POST/PUT/PATCH writes.
    #[test]
    fn derive_action_matrix() {
        for method in [Method::GET, Method::HEAD, Method::OPTIONS] {
            assert_eq!(derive_action(&method, "/api/wards"), Action::Read);
        }
        assert_eq!(
            derive_action(&Method::DELETE, "/api/wards/1"),
            Action::Delete
        );
        for path in [
            "/api/wards/merge",
            "/api/wards/deduplicate",
            "/api/wards/import",
        ] {
            assert_eq!(derive_action(&Method::POST, path), Action::Destructive);
        }
        assert_eq!(derive_action(&Method::POST, "/api/wards"), Action::Write);
        assert_eq!(
            derive_action(&Method::POST, "/api/wards/check-duplicates"),
            Action::Write
        );
        assert_eq!(derive_action(&Method::PUT, "/api/wards/1"), Action::Write);
        assert_eq!(derive_action(&Method::PATCH, "/api/wards/1"), Action::Write);
        // GET on a destructive-suffixed path is still a read — only
        // POST consults the suffix list.
        assert_eq!(
            derive_action(&Method::GET, "/api/wards/merge"),
            Action::Read
        );
        // SEC-G6: a trailing slash must NOT downgrade a destructive POST to
        // Write (a non-admin `access=write` caller must not reach merge).
        for path in [
            "/api/wards/merge/",
            "/api/wards/merge//",
            "/api/wards/deduplicate/",
            "/api/wards/import/",
        ] {
            assert_eq!(
                derive_action(&Method::POST, path),
                Action::Destructive,
                "{path} must stay Destructive"
            );
        }
    }

    /// ABAC default policy, empty `attrs` ⇒ GET allowed, POST `403`
    /// (default allow-read / deny-mutation).
    #[test]
    fn abac_empty_attrs_reads_but_cannot_write() {
        let (keys, kid) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let policy = policy();
        let token = sign_with_attrs(&kid, 10_000_000_000, &[]);
        assert!(
            enforce(
                true,
                &Method::GET,
                "/api/wards",
                &bearer(&token),
                &verifier,
                &policy
            )
            .is_ok()
        );
        let err = enforce(
            true,
            &Method::POST,
            "/api/wards",
            &bearer(&token),
            &verifier,
            &policy,
        )
        .unwrap_err();
        assert_eq!(err.0, StatusCode::FORBIDDEN);
    }

    /// ABAC `access=write` ⇒ POST/PUT allowed; DELETE and the merge
    /// POST still `403` (write is not destructive).
    #[test]
    fn abac_access_write_writes_but_not_destructive() {
        let (keys, kid) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let policy = policy();
        let token = sign_with_attrs(&kid, 10_000_000_000, &[("access", &["write"])]);
        for method in [Method::POST, Method::PUT] {
            assert!(
                enforce(
                    true,
                    &method,
                    "/api/wards",
                    &bearer(&token),
                    &verifier,
                    &policy
                )
                .is_ok(),
                "{method} should be allowed for access=write"
            );
        }
        let delete = enforce(
            true,
            &Method::DELETE,
            "/api/wards/1",
            &bearer(&token),
            &verifier,
            &policy,
        )
        .unwrap_err();
        assert_eq!(delete.0, StatusCode::FORBIDDEN);
        let merge = enforce(
            true,
            &Method::POST,
            "/api/wards/merge",
            &bearer(&token),
            &verifier,
            &policy,
        )
        .unwrap_err();
        assert_eq!(merge.0, StatusCode::FORBIDDEN);
    }

    /// ABAC `access=admin` ⇒ DELETE and the destructive named POSTs
    /// are allowed (destructive covers delete).
    #[test]
    fn abac_access_admin_allows_destructive() {
        let (keys, kid) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let policy = policy();
        let token = sign_with_attrs(&kid, 10_000_000_000, &[("access", &["admin"])]);
        assert!(
            enforce(
                true,
                &Method::DELETE,
                "/api/wards/1",
                &bearer(&token),
                &verifier,
                &policy
            )
            .is_ok()
        );
        for path in ["/api/wards/merge", "/api/wards/deduplicate"] {
            assert!(
                enforce(
                    true,
                    &Method::POST,
                    path,
                    &bearer(&token),
                    &verifier,
                    &policy
                )
                .is_ok(),
                "{path} should be allowed for access=admin"
            );
        }
    }

    /// ABAC `svc=true` (machine peer) ⇒ everything is allowed.
    #[test]
    fn abac_svc_true_allows_everything() {
        let (keys, kid) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let policy = policy();
        let token = sign_with_attrs(&kid, 10_000_000_000, &[("svc", &["true"])]);
        for (method, path) in [
            (Method::GET, "/api/wards"),
            (Method::POST, "/api/wards"),
            (Method::PUT, "/api/wards/1"),
            (Method::DELETE, "/api/wards/1"),
            (Method::POST, "/api/wards/merge"),
            (Method::POST, "/api/wards/deduplicate"),
        ] {
            assert!(
                enforce(true, &method, path, &bearer(&token), &verifier, &policy).is_ok(),
                "{method} {path} should be allowed for svc=true"
            );
        }
    }

    /// A configured deny rule ahead of an allow rule wins
    /// (first-match-wins pin, through the guard).
    #[test]
    fn abac_configured_deny_beats_later_allow() {
        let (keys, kid) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let policy = Policy::from_json(
            r#"{ "rules": [
                { "effect": "deny",  "actions": ["write"], "when": { "purpose": ["research"] } },
                { "effect": "allow", "actions": ["write"], "when": { "access": ["write"] } }
            ] }"#,
        )
        .expect("policy parses");
        let denied = sign_with_attrs(
            &kid,
            10_000_000_000,
            &[("access", &["write"]), ("purpose", &["research"])],
        );
        let err = enforce(
            true,
            &Method::POST,
            "/api/wards",
            &bearer(&denied),
            &verifier,
            &policy,
        )
        .unwrap_err();
        assert_eq!(err.0, StatusCode::FORBIDDEN);
        let allowed = sign_with_attrs(&kid, 10_000_000_000, &[("access", &["write"])]);
        assert!(
            enforce(
                true,
                &Method::POST,
                "/api/wards",
                &bearer(&allowed),
                &verifier,
                &policy
            )
            .is_ok()
        );
    }

    /// 401 vs 403: missing/bad credential is `401`; a valid credential
    /// the policy denies is `403` with the deciding-rule reason.
    #[test]
    fn abac_401_versus_403_distinction() {
        let (keys, kid) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let policy = policy();
        let no_token = enforce(
            true,
            &Method::POST,
            "/api/wards",
            &HeaderMap::new(),
            &verifier,
            &policy,
        )
        .unwrap_err();
        assert_eq!(no_token.0, StatusCode::UNAUTHORIZED);
        let token = sign_with_attrs(&kid, 10_000_000_000, &[]);
        let denied = enforce(
            true,
            &Method::POST,
            "/api/wards",
            &bearer(&token),
            &verifier,
            &policy,
        )
        .unwrap_err();
        assert_eq!(denied.0, StatusCode::FORBIDDEN);
        assert_eq!(denied.1, "default deny");
    }

    /// `policy_from_env` never breaks boot: bad policy JSON falls back
    /// to the built-in default policy (the pure fallback path
    /// `policy_from_env` takes on parse failure).
    #[test]
    fn policy_bad_json_falls_back_to_default() {
        assert!(Policy::from_json("{ not json").is_err());
        assert_eq!(
            Policy::from_json("{ not json").unwrap_or_else(|_| Policy::default_policy()),
            Policy::default_policy()
        );
    }

    /// Serve `keys` as the key-set JSON from a local ephemeral-port HTTP
    /// listener (the auth service's `/.well-known/paseto-keys` stand-in)
    /// and return the URL to fetch it from.
    async fn serve_keys(keys: serde_json::Value) -> String {
        let app = axum::Router::new().route(
            "/.well-known/paseto-keys",
            axum::routing::get(move || {
                let keys = keys.clone();
                async move { axum::Json(keys) }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local addr");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve key set");
        });
        format!("http://{addr}/.well-known/paseto-keys")
    }

    /// Boot-time fetch happy path: `fetch_or` against a local listener
    /// serving a valid key set builds the verifier from the **fetched**
    /// keys — a token signed by the served key verifies even though the
    /// fallback verifier has no keys (the fetched key set wins).
    #[tokio::test]
    async fn fetch_or_fetched_key_set_wins() {
        let (keys, kid) = test_keys_and_kid();
        let url = serve_keys(keys).await;
        let verifier = fetch_or(&url, ISSUER, AUDIENCE, empty_verifier(ISSUER, AUDIENCE)).await;
        assert_eq!(verifier.key_count(), 1);
        let token = sign(&kid, 10_000_000_000);
        let claims =
            bearer_claims(&bearer(&token), &verifier).expect("token signed by fetched key");
        assert_eq!(claims.sub, "11111111-1111-1111-1111-111111111111");
    }

    /// Boot-time fetch fallback: a fast-failing URL (nothing listens on
    /// port 1) makes `fetch_or` return the env-style fallback verifier —
    /// no panic, and tokens signed by the fallback key set still verify,
    /// so the service always boots.
    #[tokio::test]
    async fn fetch_or_unreachable_url_falls_back() {
        let (keys, kid) = test_keys_and_kid();
        let fallback = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let verifier = fetch_or("http://127.0.0.1:1/", ISSUER, AUDIENCE, fallback).await;
        assert_eq!(verifier.key_count(), 1);
        let token = sign(&kid, 10_000_000_000);
        assert!(bearer_claims(&bearer(&token), &verifier).is_ok());
    }

    /// Build `Claims` with the given subject attributes for the
    /// record-level decision tests (no signing needed).
    fn claims_with_attrs(attrs: &[(&str, &[&str])]) -> Claims {
        Claims {
            sub: "11111111-1111-1111-1111-111111111111".into(),
            email: "alice@example.com".into(),
            name: "Alice".into(),
            iss: ISSUER.into(),
            aud: AUDIENCE.into(),
            exp: 2_000_000_000,
            iat: 1_900_000_000,
            nbf: None,
            sid: "test-sid".into(),
            scope: Vec::new(),
            roles: Vec::new(),
            attrs: attrs
                .iter()
                .map(|(key, values)| {
                    (
                        (*key).to_string(),
                        values.iter().map(ToString::to_string).collect(),
                    )
                })
                .collect(),
        }
    }

    /// The environment-attribute derivation flags working vs after
    /// hours from a UTC hour (`authorization-attributes.md` §10).
    #[test]
    fn env_attrs_at_flags_after_hours() {
        for hour in 8..18 {
            let env = env_attrs_at(hour);
            assert_eq!(
                env["after_hours"],
                vec!["false".to_string()],
                "{hour}:00 is working hours"
            );
            assert_eq!(env["hour"], vec![hour.to_string()]);
        }
        for hour in [0, 7, 18, 22, 23] {
            assert_eq!(
                env_attrs_at(hour)["after_hours"],
                vec!["true".to_string()],
                "{hour}:00 is after hours"
            );
        }
    }

    /// A stay model for the resource-attribute tests.
    fn a_stay(ward: Option<uuid::Uuid>, status: &str) -> crate::models::_entities::stays::Model {
        crate::models::_entities::stays::Model {
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
            id: 1,
            pid: uuid::Uuid::new_v4(),
            person_ref: format!("person:{}", uuid::Uuid::new_v4()),
            display_name: "Ada Lovelace".to_string(),
            status: status.to_string(),
            admitted_at: chrono::Utc::now().into(),
            source: "ed".to_string(),
            ward_pid: ward,
            bed_pid: None,
            home_location_note: Some("home".to_string()),
            named_nurse_ref: None,
            consultant_ref: None,
            senior_review_at: None,
            edd: None,
            ccd: None,
            ccd_met: false,
            discharge_pathway: None,
            discharge_ready_at: None,
            discharged_at: None,
            discharge_destination: None,
            alerts: serde_json::json!(["falls risk"]),
            deleted_at: None,
        }
    }

    /// The record-level resource-attribute derivation maps a stay's
    /// ward / status / source to the lowercase `resource.*` tokens a
    /// policy matches (`authorization-attributes.md` §9).
    #[test]
    fn stay_resource_attrs_maps_ward_status_source() {
        let ward = uuid::Uuid::new_v4();
        let attrs = stay_resource_attrs(&a_stay(Some(ward), "admitted"));
        assert_eq!(attrs["ward"], vec![ward.to_string()]);
        assert_eq!(attrs["status"], vec!["admitted".to_string()]);
        assert_eq!(attrs["source"], vec!["ed".to_string()]);
        // No ward (virtual pre-placement) ⇒ no ward key.
        assert!(!stay_resource_attrs(&a_stay(None, "admitted")).contains_key("ward"));
    }

    /// These attributes drive a ward-scoped decision through the shared
    /// engine: a caller scoped to ward A reads a ward-A stay, is denied
    /// on ward B, and a mask-obligation rule yields the obligation.
    #[test]
    fn stay_resource_attrs_drive_ward_scoping_and_masking() {
        let ward_a = uuid::Uuid::new_v4();
        let ward_b = uuid::Uuid::new_v4();
        let policy = Policy::from_json(&format!(
            r#"{{ "rules": [
                {{ "effect": "allow", "actions": ["read"], "when": {{ "ward": ["{ward_a}"], "resource.ward": ["{ward_a}"] }} }},
                {{ "effect": "allow", "actions": ["read"], "when": {{}}, "obligations": ["mask"] }}
            ] }}"#
        ))
        .expect("policy parses");
        let nurse_a = claims_with_attrs(&[("ward", &[&ward_a.to_string()])]);
        let own_ward = policy.evaluate_with_resource(
            &nurse_a,
            Action::Read,
            ENTITY,
            &stay_resource_attrs(&a_stay(Some(ward_a), "admitted")),
        );
        assert!(own_ward.allowed);
        assert!(own_ward.obligations.is_empty());
        let other_ward = policy.evaluate_with_resource(
            &nurse_a,
            Action::Read,
            ENTITY,
            &stay_resource_attrs(&a_stay(Some(ward_b), "admitted")),
        );
        assert!(other_ward.allowed, "falls through to the masked-read rule");
        assert!(other_ward.requires("mask"));
    }

    /// `mask_stay` redacts the display name, alerts, and home note,
    /// and leaves bed/ward state + the pseudonymous URN visible.
    #[test]
    fn mask_stay_redacts_identifying_fields() {
        let ward = uuid::Uuid::new_v4();
        let masked = mask_stay(a_stay(Some(ward), "admitted"));
        assert_eq!(masked.display_name, MASKED);
        assert_eq!(masked.alerts, serde_json::json!([]));
        assert!(masked.home_location_note.is_none());
        assert_eq!(masked.ward_pid, Some(ward));
        assert!(masked.person_ref.starts_with("person:"));
    }
}
