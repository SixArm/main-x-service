//! Content insights (CMS-R21): findings that name their rule, ratios
//! that show their working, percentiles that refuse to summarise a
//! sample too small to have one — and no reader analytics anywhere.

use content_management_system_service::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_key, a_site_payload};

/// A site with a routable article type.
async fn seed_site(request: &axum_test::TestServer, prefix: &str) -> String {
    let mut payload = a_site_payload(&a_key(prefix));
    payload["base_url"] = json!("https://example.test");
    let created: Value = request.post("/api/sites").json(&payload).await.json();
    let site_pid = created["pid"].as_str().unwrap().to_string();
    request
        .post(&format!("/api/sites/{site_pid}/content-types"))
        .json(&json!({
            "key": "article", "name": "Article", "routable": false,
            "fields": [{ "key": "hero", "label": "Hero", "kind": "media" }],
        }))
        .await
        .assert_status_ok();
    site_pid
}

/// A minimal PNG header of the given size.
///
/// The size varies per call site on purpose: identical bytes
/// deduplicate to one asset (CMS-R6), so two "different" uploads of the
/// same content would be one row — which is correct behaviour and would
/// quietly make an orphan-asset fixture impossible.
fn png(size: u32) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    bytes.extend_from_slice(&[0, 0, 0, 13]);
    bytes.extend_from_slice(b"IHDR");
    bytes.extend_from_slice(&size.to_be_bytes());
    bytes.extend_from_slice(&size.to_be_bytes());
    bytes
}

/// Upload an asset, optionally with alt text.
async fn upload(
    request: &axum_test::TestServer,
    site_pid: &str,
    size: u32,
    alt: Option<&str>,
) -> String {
    use axum_test::multipart::{MultipartForm, Part};
    let mut form = MultipartForm::new().add_part(
        "file",
        Part::bytes(png(size))
            .file_name("i.png")
            .mime_type("image/png"),
    );
    if let Some(alt) = alt {
        form = form.add_text("alt_text", alt.to_string());
    }
    let uploaded: Value = request
        .post(&format!("/api/sites/{site_pid}/assets"))
        .multipart(form)
        .await
        .json();
    uploaded["pid"].as_str().unwrap().to_string()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn health_findings_name_their_rule_and_explain_it() {
    request::<App, _, _>(|request, _ctx| async move {
        let site_pid = seed_site(&request, "health").await;
        let asset = upload(&request, &site_pid, 64, None).await;
        // An orphan: uploaded, referenced by nothing. Different bytes,
        // or it would deduplicate onto the one above.
        let orphan = upload(
            &request,
            &site_pid,
            128,
            Some("An unused but described image"),
        )
        .await;
        assert_ne!(orphan, asset, "the fixture needs two distinct assets");

        // A published page using an image with no alt text, and no SEO
        // metadata.
        let created: Value = request
            .post(&format!("/api/sites/{site_pid}/entries"))
            .json(&json!({
                "key": "illustrated", "content_type_key": "article", "title": "Illustrated",
                "blocks": [{ "kind": "image", "asset": asset }],
            }))
            .await
            .json();
        let entry_pid = created["pid"].as_str().unwrap().to_string();
        // Publishing is blocked by the alt-text gate, so give the asset
        // alt text, publish, then take it away again — which is exactly
        // how a real site ends up in this state.
        request
            .put(&format!("/api/assets/{asset}"))
            .json(&json!({ "alt_text": "temporary" }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/entries/{entry_pid}/variants/en/transition"))
            .json(&json!({ "action": "publish" }))
            .await
            .assert_status_ok();
        request
            .put(&format!("/api/assets/{asset}"))
            .json(&json!({ "alt_text": "" }))
            .await
            .assert_status_ok();

        let response = request
            .get(&format!("/api/sites/{site_pid}/insights/health"))
            .await;
        response.assert_status_ok();
        let payload: Value = response.json();
        assert!(payload["as_of"].is_string());
        assert_eq!(payload["published_variants"], 1);

        let rules: Vec<&str> = payload["by_rule"]
            .as_array()
            .unwrap()
            .iter()
            .map(|group| group["rule"].as_str().unwrap())
            .collect();
        assert!(rules.contains(&"image_alt_text_missing"), "{rules:?}");
        assert!(rules.contains(&"seo_metadata_missing"), "{rules:?}");
        assert!(rules.contains(&"orphan_asset"), "{rules:?}");

        // Every group ships the sentence the code applied, so a
        // dashboard never invents its own wording.
        for group in payload["by_rule"].as_array().unwrap() {
            assert!(
                group["explanation"]
                    .as_str()
                    .is_some_and(|text| text.len() > 20),
                "{} has no explanation",
                group["rule"]
            );
            assert_eq!(
                group["count"].as_u64().unwrap() as usize,
                group["findings"].as_array().unwrap().len()
            );
        }

        // Nothing is acted on automatically, and the response says so.
        assert!(
            payload["note"].as_str().unwrap().contains("automatically"),
            "the view states that it changes nothing"
        );
        assert!(payload["orphan_bytes"].as_i64().unwrap() > 0);

        // Conditional: an unchanged view keeps its tag.
        let tag = response
            .headers()
            .get("etag")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let conditional = request
            .get(&format!("/api/sites/{site_pid}/insights/health"))
            .add_header("if-none-match", tag)
            .await;
        assert_eq!(conditional.status_code(), 304);
    })
    .await;
}

/// A clean site produces no findings — and says so with a count, not an
/// absence.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_clean_site_reports_zero_findings() {
    request::<App, _, _>(|request, _ctx| async move {
        let site_pid = seed_site(&request, "clean").await;
        let payload: Value = request
            .get(&format!("/api/sites/{site_pid}/insights/health"))
            .await
            .json();
        assert_eq!(payload["findings_total"], 0);
        assert!(payload["by_rule"].as_array().unwrap().is_empty());
        assert_eq!(payload["entries"], 0);
    })
    .await;
}

/// Throughput ratios show their working, and a small sample refuses to
/// become a percentile.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn throughput_is_honest_about_small_samples_and_zero_denominators() {
    request::<App, _, _>(|request, _ctx| async move {
        let site_pid = seed_site(&request, "throughput").await;

        // Nothing has happened yet: every rate is null, not 0% or 100%.
        let payload: Value = request
            .get(&format!("/api/sites/{site_pid}/insights/throughput"))
            .await
            .json();
        assert!(payload["rates"]["approval_rate"]["value"].is_null());
        assert_eq!(payload["rates"]["approval_rate"]["numerator"], 0);
        assert_eq!(payload["rates"]["approval_rate"]["denominator"], 0);
        assert_eq!(payload["activity"]["published"], 0);

        // Walk two entries through the lifecycle.
        for key in ["one", "two"] {
            let created: Value = request
                .post(&format!("/api/sites/{site_pid}/entries"))
                .json(&json!({ "key": key, "content_type_key": "article", "title": key }))
                .await
                .json();
            let entry_pid = created["pid"].as_str().unwrap().to_string();
            for action in ["submit", "approve", "publish"] {
                request
                    .post(&format!("/api/entries/{entry_pid}/variants/en/transition"))
                    .json(&json!({ "action": action }))
                    .await
                    .assert_status_ok();
            }
        }

        let payload: Value = request
            .get(&format!("/api/sites/{site_pid}/insights/throughput"))
            .await
            .json();
        assert_eq!(payload["activity"]["submitted"], 2);
        assert_eq!(payload["activity"]["approved"], 2);
        assert_eq!(payload["activity"]["published"], 2);
        // The ratio shows its working.
        let approval = &payload["rates"]["approval_rate"];
        assert_eq!(approval["numerator"], 2);
        assert_eq!(approval["denominator"], 2);
        assert!((approval["value"].as_f64().unwrap() - 1.0).abs() < f64::EPSILON);

        // Two observations is not a p90: the raw durations are returned
        // instead, with a note saying why.
        let review = &payload["time_in_state"]["review_to_approved"];
        assert_eq!(review["sample_size"], 2);
        assert!(review["p90_seconds"].is_null());
        assert_eq!(review["observations_seconds"].as_array().unwrap().len(), 2);
        assert!(review["note"].as_str().unwrap().contains("raw durations"));

        // Time in state is measured from transitions, and the payload
        // says so rather than leaving a reader to assume.
        assert!(
            payload["time_in_state"]["measured_from"]
                .as_str()
                .unwrap()
                .contains("audit"),
        );

        // These are editorial insights, not reader analytics: nothing
        // here counts visits, because nothing records them.
        let body = payload.to_string();
        for word in ["visit", "visitor", "pageview", "session"] {
            assert!(
                !body.contains(word),
                "{word} has no business in this payload"
            );
        }
    })
    .await;
}

/// The backlog buckets what is waiting by age.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn the_backlog_shows_what_is_waiting_and_for_how_long() {
    request::<App, _, _>(|request, _ctx| async move {
        let site_pid = seed_site(&request, "backlog").await;
        let created: Value = request
            .post(&format!("/api/sites/{site_pid}/entries"))
            .json(&json!({ "key": "waiting", "content_type_key": "article", "title": "Waiting" }))
            .await
            .json();
        let entry_pid = created["pid"].as_str().unwrap().to_string();
        request
            .post(&format!("/api/entries/{entry_pid}/variants/en/transition"))
            .json(&json!({ "action": "submit" }))
            .await
            .assert_status_ok();

        let payload: Value = request
            .get(&format!("/api/sites/{site_pid}/insights/backlog"))
            .await
            .json();
        let pending = payload["pending_review"].as_array().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0]["entry_key"], "waiting");
        assert_eq!(pending[0]["bucket"], "today");
        assert!(payload["pending_schedule"].as_array().unwrap().is_empty());
        assert!(payload["open_translations"].as_array().unwrap().is_empty());
    })
    .await;
}
