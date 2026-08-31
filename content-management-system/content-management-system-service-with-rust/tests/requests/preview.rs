//! Preview tokens (CMS-R22): the one credential that shows
//! unpublished content, and every property that keeps it from becoming
//! a leak.

use content_management_system_service::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_key, a_site_payload};

/// A site with a non-routable article type and one drafted entry.
/// Returns `(site_key, entry_pid, first_revision_pid)`.
async fn seed_draft(request: &loco_rs::TestServer, prefix: &str) -> (String, String, String) {
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
            "key": "article", "name": "Article", "routable": false,
            "fields": [{ "key": "standfirst", "label": "Standfirst", "kind": "text" }],
        }))
        .await
        .assert_status_ok();
    let entry: Value = request
        .post(&format!("/api/sites/{site_pid}/entries"))
        .json(&json!({
            "key": "embargoed", "content_type_key": "article", "title": "Embargoed story",
            "blocks": [{ "kind": "paragraph", "text": "not for publication yet" }],
        }))
        .await
        .json();
    (
        key,
        entry["pid"].as_str().unwrap().to_string(),
        entry["revision_pid"].as_str().unwrap().to_string(),
    )
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_preview_share_renders_one_revision_and_says_it_is_a_preview() {
    request::<App, _, _>(|request, _ctx| async move {
        let (site_key, entry_pid, revision_pid) = seed_draft(&request, "preview").await;

        let issued: Value = request
            .post(&format!("/api/entries/{entry_pid}/variants/en/preview"))
            .json(&json!({}))
            .await
            .json();
        let token = issued["token"].as_str().unwrap().to_string();
        assert_eq!(token.len(), 64, "256 bits of token");
        assert_eq!(issued["revision_pid"], revision_pid);
        assert!(issued["url"].as_str().unwrap().contains(&site_key));

        let response = request
            .get(&format!("/delivery/{site_key}/preview/{token}"))
            .await;
        response.assert_status_ok();
        // Unpublished content must never be cached or indexed.
        assert_eq!(
            response.headers().get("cache-control").unwrap(),
            "private, no-store"
        );
        assert!(
            response
                .headers()
                .get("x-robots-tag")
                .unwrap()
                .to_str()
                .unwrap()
                .contains("noindex")
        );
        let payload: Value = response.json();
        assert_eq!(payload["preview"], true);
        assert_eq!(payload["revision"]["title"], "Embargoed story");
        assert_eq!(payload["status"], "draft");
        assert_eq!(
            payload["is_published_revision"], false,
            "the payload is explicit that this is not what delivery serves"
        );

        // The share and its use are both audited — and the token is in
        // neither audit row.
        let trail: Value = request.get("/api/audits/recent").await.json();
        let rows = trail.as_array().unwrap();
        assert!(rows.iter().any(|row| row["action"] == "preview_issued"));
        assert!(rows.iter().any(|row| row["action"] == "preview_used"));
        assert!(
            !trail.to_string().contains(&token),
            "the token never appears in an audit row"
        );

        // The listing shows the outstanding share, without the token.
        let shares: Value = request
            .get(&format!("/api/entries/{entry_pid}/variants/en/preview"))
            .await
            .json();
        assert_eq!(shares["shares"][0]["used_count"], 1);
        assert_eq!(shares["shares"][0]["live"], true);
        assert!(!shares.to_string().contains(&token));
    })
    .await;
}

/// The property that matters most: a share does not follow the content
/// forward.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_token_is_scoped_to_the_revision_it_was_minted_for() {
    request::<App, _, _>(|request, _ctx| async move {
        let (site_key, entry_pid, first) = seed_draft(&request, "scope").await;
        let issued: Value = request
            .post(&format!("/api/entries/{entry_pid}/variants/en/preview"))
            .json(&json!({}))
            .await
            .json();
        let token = issued["token"].as_str().unwrap().to_string();

        // The story is rewritten — perhaps with something nobody meant
        // to share.
        request
            .post(&format!("/api/entries/{entry_pid}/variants/en/revisions"))
            .json(&json!({
                "base_revision_pid": first,
                "title": "Embargoed story",
                "blocks": [{ "kind": "paragraph", "text": "the merger closes on Friday" }],
            }))
            .await
            .assert_status_ok();

        // The old link still renders the old revision, not the new one.
        let payload: Value = request
            .get(&format!("/delivery/{site_key}/preview/{token}"))
            .await
            .json();
        assert_eq!(payload["revision"]["pid"], first);
        assert_eq!(
            payload["revision"]["blocks"][0]["text"], "not for publication yet",
            "the share is pinned to what was shared"
        );
    })
    .await;
}

/// Revocation is immediate, and every refusal looks the same from
/// outside.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn revocation_is_immediate_and_refusals_are_uniform() {
    request::<App, _, _>(|request, _ctx| async move {
        let (site_key, entry_pid, _) = seed_draft(&request, "revoke").await;
        let issued: Value = request
            .post(&format!("/api/entries/{entry_pid}/variants/en/preview"))
            .json(&json!({ "ttl_secs": 3600 }))
            .await
            .json();
        let token = issued["token"].as_str().unwrap().to_string();
        let token_pid = issued["pid"].as_str().unwrap().to_string();

        request
            .get(&format!("/delivery/{site_key}/preview/{token}"))
            .await
            .assert_status_ok();
        request
            .delete(&format!("/api/preview-tokens/{token_pid}"))
            .await
            .assert_status_ok();

        let revoked = request
            .get(&format!("/delivery/{site_key}/preview/{token}"))
            .await;
        assert_eq!(revoked.status_code(), 404);
        let revoked_body = revoked.text();

        // A token that never existed gets the same answer, so the
        // endpoint cannot be used to probe for real ones.
        let unknown = request
            .get(&format!("/delivery/{site_key}/preview/{}", "0".repeat(64)))
            .await;
        assert_eq!(unknown.status_code(), 404);
        assert_eq!(unknown.text(), revoked_body);

        // The refusal is recorded even though the caller learns nothing.
        let trail: Value = request.get("/api/audits/recent").await.json();
        assert!(trail.as_array().unwrap().iter().any(
            |row| row["action"] == "preview_refused" && row["snapshot"]["refusal"] == "revoked"
        ));
    })
    .await;
}

/// Lifetimes are clamped, and a share belongs to its own site.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn lifetimes_are_clamped_and_tokens_do_not_cross_sites() {
    request::<App, _, _>(|request, _ctx| async move {
        let (site_key, entry_pid, _) = seed_draft(&request, "clamp").await;
        let (other_key, _, _) = seed_draft(&request, "other").await;

        // A request for a year gets a day.
        let issued: Value = request
            .post(&format!("/api/entries/{entry_pid}/variants/en/preview"))
            .json(&json!({ "ttl_secs": 31_536_000 }))
            .await
            .json();
        let expires: chrono::DateTime<chrono::Utc> =
            issued["expires_at"].as_str().unwrap().parse().unwrap();
        let ttl = (expires - chrono::Utc::now()).num_seconds();
        assert!(
            (86_000..=86_400).contains(&ttl),
            "a year was clamped to a day, got {ttl}s"
        );

        // The same token presented against another site is refused.
        let token = issued["token"].as_str().unwrap();
        assert_eq!(
            request
                .get(&format!("/delivery/{other_key}/preview/{token}"))
                .await
                .status_code(),
            404
        );
        // ...and works against its own.
        request
            .get(&format!("/delivery/{site_key}/preview/{token}"))
            .await
            .assert_status_ok();
    })
    .await;
}
