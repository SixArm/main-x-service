//! Upstream service clients (WPM-D11): **stub-first** display-name
//! resolution for `person:` / `worker:` `EntityRefs`.
//!
//! WPM references identities by URN and needs only display labels.
//! Lookups are read-only, cached, best-effort, and **never block
//! writes**: an unreachable upstream degrades a view to showing the
//! URN.
//!
//! Selection is per-process via `CRM_UPSTREAM_MODE`:
//! - `stub` (default) — no network; returns `None` so callers fall
//!   back to the caller-supplied or URN-derived label.
//! - `http` — `GET {base}/api/<plural>/{pid}` against
//!   `CRM_PERSON_SERVICE_URL` / `CRM_WORKER_SERVICE_URL` /
//!   `CRM_ORGANIZATION_SERVICE_URL` / `CRM_COURSE_SERVICE_URL`,
//!   reading a best-effort `name`-ish field. (Thin by design; the
//!   family front-ends do the rich rendering.)

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use entity_ref::EntityRef;

/// Client mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// No network; every lookup misses (default).
    Stub,
    /// HTTP lookups against the sibling services.
    Http,
}

/// The process-wide mode, from `CRM_UPSTREAM_MODE`.
#[must_use]
pub fn mode() -> Mode {
    static MODE: OnceLock<Mode> = OnceLock::new();
    *MODE.get_or_init(|| {
        match std::env::var("CRM_UPSTREAM_MODE")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "http" => Mode::Http,
            _ => Mode::Stub,
        }
    })
}

/// Process-wide display-name cache (URN → name).
fn cache() -> &'static Mutex<HashMap<String, String>> {
    static CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Resolve a display name for `entity_ref`, best-effort: the cache,
/// then (in `http` mode) the owning service, else `None`. Failures
/// are logged at debug and swallowed — upstream reads never fail an
/// WPM request.
pub async fn display_name(entity_ref: &EntityRef) -> Option<String> {
    let urn = entity_ref.to_string();
    if let Some(hit) = cache()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(&urn)
    {
        return Some(hit.clone());
    }
    if mode() != Mode::Http {
        return None;
    }
    let fetched = fetch_display_name(entity_ref).await;
    if let Some(name) = &fetched {
        cache()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(urn, name.clone());
    }
    fetched
}

/// Seed the cache (used by tests and the seed task).
pub fn prime(urn: &str, name: &str) {
    cache()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(urn.to_string(), name.to_string());
}

/// The env var naming the base URL for an entity type's service.
fn base_url_var(entity_ref: &EntityRef) -> String {
    format!(
        "CRM_{}_SERVICE_URL",
        entity_ref.entity_type.as_str().to_ascii_uppercase()
    )
}

/// One HTTP lookup against the owning service (`http` mode only).
async fn fetch_display_name(entity_ref: &EntityRef) -> Option<String> {
    let base = std::env::var(base_url_var(entity_ref)).ok()?;
    let url = format!(
        "{}/api/{}s/{}",
        base.trim_end_matches('/'),
        entity_ref.entity_type.as_str(),
        entity_ref.id
    );
    let body: serde_json::Value = reqwest_get_json(&url).await?;
    // Best-effort name extraction across the family's DTO shapes.
    ["display_name", "name", "title"]
        .iter()
        .find_map(|k| body.get(k).and_then(serde_json::Value::as_str))
        .map(ToString::to_string)
}

/// Minimal GET-JSON helper: short timeout, no redirects (SSRF posture,
/// security invariant 7). Errors ⇒ `None` (best-effort).
async fn reqwest_get_json(url: &str) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .ok()?;
    match client.get(url).send().await {
        Ok(response) => response.json().await.ok(),
        Err(error) => {
            tracing::debug!(%url, %error, "upstream lookup failed; degrading to URN");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    /// Stub mode misses; a primed cache hits regardless of mode.
    #[tokio::test]
    async fn stub_mode_misses_and_cache_hits() {
        let urn = format!("person:{}", uuid::Uuid::new_v4());
        let entity_ref = EntityRef::from_str(&urn).unwrap();
        assert_eq!(display_name(&entity_ref).await, None);
        prime(&urn, "Ada Lovelace");
        assert_eq!(
            display_name(&entity_ref).await.as_deref(),
            Some("Ada Lovelace")
        );
    }
}
