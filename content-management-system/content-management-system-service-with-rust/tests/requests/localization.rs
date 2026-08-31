//! Localization (CMS-R13–R15): fallback resolution that reports what
//! it did, strict locales that refuse it, the translation workflow, and
//! staleness derived from source-revision drift.

use content_management_system_service::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_key, a_site_payload};

/// A three-locale site (`en` default, `fr`, `fr-CA` → `fr` → `en`) with
/// an article type.
async fn seed_multilingual_site(request: &loco_rs::TestServer, prefix: &str) -> (String, String) {
    let key = a_key(prefix);
    let created: Value = request
        .post("/api/sites")
        .json(&a_site_payload(&key))
        .await
        .json();
    let site_pid = created["pid"].as_str().unwrap().to_string();
    request
        .post(&format!("/api/sites/{site_pid}/content-types"))
        .json(&json!({
            "key": "article", "name": "Article",
            // Not routable: these tests are about locales, not addresses, and a
            // routable type cannot publish without one (CMS-R11).
            "routable": false,
            "fields": [{ "key": "standfirst", "label": "Standfirst", "kind": "text" }],
        }))
        .await
        .assert_status_ok();
    (site_pid, key)
}

/// Create an entry in the source locale.
async fn create_entry(request: &loco_rs::TestServer, site_pid: &str, key: &str) -> String {
    let created: Value = request
        .post(&format!("/api/sites/{site_pid}/entries"))
        .json(&json!({
            "key": key, "content_type_key": "article", "title": "Source title",
            "blocks": [{ "kind": "paragraph", "text": "source body" }],
        }))
        .await
        .json();
    created["pid"].as_str().unwrap().to_string()
}

/// Publish a variant.
async fn publish(request: &loco_rs::TestServer, entry_pid: &str, locale: &str) {
    request
        .post(&format!(
            "/api/entries/{entry_pid}/variants/{locale}/transition"
        ))
        .json(&json!({ "action": "publish" }))
        .await
        .assert_status_ok();
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn resolution_reports_the_locale_it_actually_served() {
    request::<App, _, _>(|request, _ctx| async move {
        let (site_pid, _) = seed_multilingual_site(&request, "resolve").await;
        let entry_pid = create_entry(&request, &site_pid, "about").await;
        publish(&request, &entry_pid, "en").await;

        // `fr-CA` has nothing, so the chain fr → en is walked and the
        // answer says so. This is the whole point: a reader asking for
        // French is told they got English.
        let resolved: Value = request
            .get(&format!("/api/entries/{entry_pid}/resolve/fr-CA"))
            .await
            .json();
        assert_eq!(resolved["resolution"]["locale_requested"], "fr-CA");
        assert_eq!(resolved["resolution"]["locale_served"], "en");
        assert_eq!(resolved["resolution"]["fallback_applied"], true);
        assert_eq!(
            resolved["resolution"]["chain_walked"],
            json!(["fr", "en"]),
            "the hops walked are visible, not inferred"
        );

        // Publish French, and the chain stops at the first hop.
        request
            .post(&format!("/api/entries/{entry_pid}/variants"))
            .json(&json!({ "locale": "fr", "body": { "title": "Titre" } }))
            .await
            .assert_status_ok();
        publish(&request, &entry_pid, "fr").await;
        let resolved: Value = request
            .get(&format!("/api/entries/{entry_pid}/resolve/fr-CA"))
            .await
            .json();
        assert_eq!(resolved["resolution"]["locale_served"], "fr");
        assert_eq!(resolved["resolution"]["chain_walked"], json!(["fr"]));

        // A locale that answers for itself reports no fallback at all.
        let resolved: Value = request
            .get(&format!("/api/entries/{entry_pid}/resolve/fr"))
            .await
            .json();
        assert_eq!(resolved["resolution"]["locale_served"], "fr");
        assert_eq!(resolved["resolution"]["fallback_applied"], false);

        // An undeclared locale is not guessed at.
        let resolved: Value = request
            .get(&format!("/api/entries/{entry_pid}/resolve/de"))
            .await
            .json();
        assert!(resolved["resolution"]["locale_served"].is_null());
        assert_eq!(resolved["resolution"]["refusal"], "undeclared");
    })
    .await;
}

/// A strict locale refuses fallback: for safety notices, showing
/// another language is worse than showing nothing.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_strict_locale_refuses_to_fall_back() {
    request::<App, _, _>(|request, _ctx| async move {
        let key = a_key("strict");
        let mut payload = a_site_payload(&key);
        payload["strict_locales"] = json!(["fr-CA"]);
        // A strict locale must not also carry a chain.
        payload["fallback_chains"] = json!({ "fr": ["en"] });
        let created: Value = request.post("/api/sites").json(&payload).await.json();
        let site_pid = created["pid"].as_str().unwrap().to_string();
        request
            .post(&format!("/api/sites/{site_pid}/content-types"))
            .json(&json!({
                "key": "article", "name": "Article", "routable": false,
                "fields": [{ "key": "x", "label": "X", "kind": "text" }],
            }))
            .await
            .assert_status_ok();
        let entry_pid = create_entry(&request, &site_pid, "safety-notice").await;
        publish(&request, &entry_pid, "en").await;

        let resolved: Value = request
            .get(&format!("/api/entries/{entry_pid}/resolve/fr-CA"))
            .await
            .json();
        assert!(resolved["resolution"]["locale_served"].is_null());
        assert_eq!(resolved["resolution"]["refusal"], "strict");

        // ...while a non-strict locale on the same site still falls back.
        let resolved: Value = request
            .get(&format!("/api/entries/{entry_pid}/resolve/fr"))
            .await
            .json();
        assert_eq!(resolved["resolution"]["locale_served"], "en");
    })
    .await;
}

/// The translation workflow, and the staleness it makes computable.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn translation_records_its_source_and_staleness_follows_the_source() {
    request::<App, _, _>(|request, _ctx| async move {
        let (site_pid, _) = seed_multilingual_site(&request, "translate").await;
        let entry_pid = create_entry(&request, &site_pid, "translated").await;
        publish(&request, &entry_pid, "en").await;
        request
            .post(&format!("/api/entries/{entry_pid}/variants"))
            .json(&json!({ "locale": "fr", "body": { "title": "Titre" } }))
            .await
            .assert_status_ok();

        let translate_path = format!("/api/entries/{entry_pid}/variants/fr/translation");
        // Out of order: completing something nobody asked for.
        let early = request
            .post(&translate_path)
            .json(&json!({ "action": "complete" }))
            .await;
        assert_eq!(early.status_code(), 422);
        assert!(early.text().contains("not requested"));

        // Request pins the source revision, so staleness is computable
        // from the moment the work starts.
        let requested: Value = request
            .post(&translate_path)
            .json(&json!({
                "action": "request",
                "translator_ref": format!("worker:{}", uuid::Uuid::new_v4()),
                "due_on": "2027-01-31",
            }))
            .await
            .json();
        assert_eq!(requested["translation_status"], "requested");
        assert!(!requested["translation_of_revision_pid"].is_null());

        request
            .post(&translate_path)
            .json(&json!({ "action": "claim" }))
            .await
            .assert_status_ok();
        let completed: Value = request
            .post(&translate_path)
            .json(&json!({ "action": "complete" }))
            .await
            .json();
        assert_eq!(completed["translation_status"], "translated");

        // Fresh: the source has not moved.
        let matrix: Value = request
            .get(&format!("/api/entries/{entry_pid}/translations"))
            .await
            .json();
        let fr = matrix["locales"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["locale"] == "fr")
            .unwrap();
        assert_eq!(fr["staleness"]["stale"], false);
        assert_eq!(fr["translation_status"], "translated");

        // The source publishes two more revisions.
        let variant: Value = request
            .get(&format!("/api/entries/{entry_pid}/variants/en"))
            .await
            .json();
        let mut base = variant["variant"]["current_revision_pid"]
            .as_str()
            .unwrap()
            .to_string();
        for title in ["Second", "Third"] {
            let saved: Value = request
                .post(&format!("/api/entries/{entry_pid}/variants/en/revisions"))
                .json(&json!({
                    "base_revision_pid": base, "title": title,
                    "blocks": [{ "kind": "paragraph", "text": title }],
                }))
                .await
                .json();
            base = saved["revision_pid"].as_str().unwrap().to_string();
        }
        publish(&request, &entry_pid, "en").await;

        // Now stale — and it says how far behind, and which revisions.
        let matrix: Value = request
            .get(&format!("/api/entries/{entry_pid}/translations"))
            .await
            .json();
        let fr = matrix["locales"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["locale"] == "fr")
            .unwrap();
        assert_eq!(fr["staleness"]["stale"], true);
        assert_eq!(fr["staleness"]["revisions_behind"], 2);
        assert_eq!(fr["staleness"]["newer_revision_numbers"], json!([2, 3]));

        // The source locale is never "stale against itself".
        let en = matrix["locales"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["locale"] == "en")
            .unwrap();
        assert_eq!(en["is_source"], true);
        assert_eq!(en["staleness"]["stale"], false);

        // The site view lists it, and says plainly that it unpublished
        // nothing.
        let site_view: Value = request
            .get(&format!("/api/sites/{site_pid}/translations"))
            .await
            .json();
        assert_eq!(site_view["auto_unpublished"], false);
        assert_eq!(site_view["stale"][0]["locale"], "fr");
        assert!(
            site_view["would_unpublish"].as_array().unwrap().is_empty(),
            "unpublish_on_stale is off by default"
        );

        // Refreshing is a re-request, which re-pins the source.
        let refreshed: Value = request
            .post(&translate_path)
            .json(&json!({ "action": "request" }))
            .await
            .json();
        assert_eq!(refreshed["translation_status"], "requested");
        assert_ne!(
            refreshed["translation_of_revision_pid"], requested["translation_of_revision_pid"],
            "the re-request points at the newer source revision"
        );
    })
    .await;
}

/// Translating the source locale into itself is refused, and a
/// translation cannot be requested with nothing to translate from.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn nonsensical_translation_requests_are_refused() {
    request::<App, _, _>(|request, _ctx| async move {
        let (site_pid, _) = seed_multilingual_site(&request, "nonsense").await;
        let entry_pid = create_entry(&request, &site_pid, "self").await;

        let source = request
            .post(&format!("/api/entries/{entry_pid}/variants/en/translation"))
            .json(&json!({ "action": "request" }))
            .await;
        assert_eq!(source.status_code(), 422);
        assert!(source.text().contains("source locale"));

        // A locale with no variant at all is a 404, not a silent
        // creation.
        let missing = request
            .post(&format!("/api/entries/{entry_pid}/variants/fr/translation"))
            .json(&json!({ "action": "request" }))
            .await;
        assert_eq!(missing.status_code(), 404);
    })
    .await;
}

/// Locale coverage lists the gap, not just the count: a percentage
/// says how bad it is, a list says what to do.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn locale_coverage_names_the_missing_entries() {
    request::<App, _, _>(|request, _ctx| async move {
        let (site_pid, _) = seed_multilingual_site(&request, "coverage").await;
        let first = create_entry(&request, &site_pid, "first").await;
        create_entry(&request, &site_pid, "second").await;
        request
            .post(&format!("/api/entries/{first}/variants"))
            .json(&json!({ "locale": "fr", "body": { "title": "Premier" } }))
            .await
            .assert_status_ok();
        publish(&request, &first, "fr").await;

        let coverage: Value = request
            .get(&format!("/api/sites/{site_pid}/locale-coverage"))
            .await
            .json();
        let fr = coverage["coverage"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["locale"] == "fr")
            .unwrap();
        assert_eq!(fr["entries_total"], 2);
        assert_eq!(fr["entries_started"], 1);
        assert_eq!(fr["entries_published"], 1);
        assert_eq!(fr["missing_entry_keys"], json!(["second"]));

        let en = coverage["coverage"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["locale"] == "en")
            .unwrap();
        assert_eq!(en["entries_started"], 2);
        assert_eq!(
            en["entries_published"], 0,
            "neither has been published in en"
        );

        // The per-entry matrix names the locales never started.
        let matrix: Value = request
            .get(&format!("/api/entries/{first}/translations"))
            .await
            .json();
        assert_eq!(matrix["missing_locales"], json!(["fr-CA"]));
    })
    .await;
}
