//! Request-level test suites: the site + template journey in [`sites`],
//! the content-type declaration + compatibility gate in [`types`], and
//! the authoring journey (revisions, conflicts, sanitization,
//! references) in [`entries`], and the asset library (upload typing,
//! dedupe, safe delivery, renditions, replace, orphans, the alt-text
//! gate) in [`assets`], and the editorial workflow (transitions,
//! publishing, scheduling, locks) in [`workflow`], and localization
//! (fallback resolution, translation, staleness) in [`localization`],
//! routing + delivery + SEO + personalization in [`delivery`], and
//! content health + editorial throughput in [`insights`], and preview
//! tokens in [`preview`], and webhook registration in [`webhooks`]
//! (delivery has its own binary, `tests/webhook_delivery.rs`, because
//! the event transport is resolved once per process), and the
//! synthetic corpus in [`seed`] — which is held to the standard that
//! matters for a fixture: every health rule must actually fire.

mod assets;
mod delivery;
mod entries;
mod insights;
mod localization;
mod preview;
mod seed;
mod sites;
mod types;
mod webhooks;
mod workflow;

use serde_json::{Value, json};

/// A fresh synthetic `organization:` URN.
pub fn an_organization() -> String {
    format!("organization:{}", uuid::Uuid::new_v4())
}

/// A minimal valid site payload with a unique key.
pub fn a_site_payload(key: &str) -> Value {
    json!({
        "key": key,
        "name": "Test site",
        "default_locale": "en",
        "locales": ["en", "fr", "fr-CA"],
        "fallback_chains": { "fr-CA": ["fr", "en"], "fr": ["en"] },
    })
}

/// Create a site and return its pid.
pub async fn seed_site(request: &loco_rs::TestServer, key: &str) -> String {
    let created: Value = request
        .post("/api/sites")
        .json(&a_site_payload(key))
        .await
        .json();
    created["pid"]
        .as_str()
        .unwrap_or_else(|| panic!("site create returned no pid: {created}"))
        .to_string()
}

/// A unique site key per test (keys are unique among live sites).
pub fn a_key(prefix: &str) -> String {
    format!("{prefix}-{}", uuid::Uuid::new_v4().simple())
}
