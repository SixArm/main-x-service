//! The authoring journey (CMS-R3–R5): entries, the append-only
//! revision chain, conflict refusal, sanitization on write, reference
//! extraction, and the delete-refusal that depends on it.

use content_management_system_service::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_key, seed_site};

/// Declare a content type with a representative field set, returning
/// the site pid.
async fn seed_site_and_type(request: &axum_test::TestServer, prefix: &str) -> String {
    let site_pid = seed_site(request, &a_key(prefix)).await;
    request
        .post(&format!("/api/sites/{site_pid}/content-types"))
        .json(&json!({
            "key": "article",
            "name": "Article",
            "fields": [
                { "key": "standfirst", "label": "Standfirst", "kind": "text",
                  "validation": { "max_len": 300 } },
                { "key": "section", "label": "Section", "kind": "choice",
                  "validation": { "options": ["news", "guide"] } },
                { "key": "related", "label": "Related", "kind": "reference", "repeatable": true },
                { "key": "about", "label": "About", "kind": "entity_ref",
                  "validation": { "entity_types": ["course"] } }
            ],
        }))
        .await
        .assert_status_ok();
    site_pid
}

/// Create an entry, returning `(entry_pid, revision_pid)`.
async fn create_entry(
    request: &axum_test::TestServer,
    site_pid: &str,
    key: &str,
) -> (String, String) {
    let created: Value = request
        .post(&format!("/api/sites/{site_pid}/entries"))
        .json(&json!({
            "key": key,
            "content_type_key": "article",
            "title": "First title",
            "blocks": [{ "kind": "paragraph", "text": "First body" }],
        }))
        .await
        .json();
    (
        created["pid"].as_str().unwrap().to_string(),
        created["revision_pid"].as_str().unwrap().to_string(),
    )
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn authoring_journey_saves_diffs_and_restores() {
    request::<App, _, _>(|request, _ctx| async move {
        let site_pid = seed_site_and_type(&request, "authoring").await;
        let (entry_pid, first) = create_entry(&request, &site_pid, "hello").await;

        // Save a second revision, stating what it edited.
        let saved: Value = request
            .post(&format!("/api/entries/{entry_pid}/variants/en/revisions"))
            .json(&json!({
                "base_revision_pid": first,
                "title": "Second title",
                "blocks": [
                    { "kind": "paragraph", "text": "Rewritten body" },
                    { "kind": "divider" },
                ],
                "fields": { "section": "news" },
                "note": "tightened the intro",
            }))
            .await
            .json();
        assert_eq!(saved["number"], 2);
        let second = saved["revision_pid"].as_str().unwrap().to_string();

        // The diff reports what actually changed, and says how it
        // compared blocks.
        let diff: Value = request
            .get(&format!("/api/revisions/{first}/diff/{second}"))
            .await
            .json();
        assert_eq!(diff["diff"]["identical"], false);
        assert_eq!(diff["diff"]["title_changed"], true);
        assert_eq!(diff["diff"]["blocks"][0]["change"], "changed");
        assert_eq!(diff["diff"]["blocks"][1]["change"], "added");
        assert_eq!(diff["diff"]["fields"][0]["key"], "section");
        assert!(
            diff["diff"]["block_comparison"]
                .as_str()
                .unwrap()
                .contains("positional")
        );

        // Restore the first revision: history extends, never rewinds.
        let restored: Value = request
            .post(&format!("/api/entries/{entry_pid}/variants/en/restore"))
            .json(&json!({ "revision_pid": first, "note": "reverted the rewrite" }))
            .await
            .json();
        assert_eq!(restored["number"], 3, "restore writes a NEW revision");

        let history: Value = request
            .get(&format!("/api/entries/{entry_pid}/variants/en/revisions"))
            .await
            .json();
        let rows = history.as_array().unwrap();
        assert_eq!(rows.len(), 3, "nothing was deleted or overwritten");
        assert_eq!(rows[0]["number"], 3);
        assert_eq!(rows[0]["is_current"], true);
        assert_eq!(rows[0]["restored_from_pid"], first);
        assert_eq!(rows[0]["note"], "reverted the rewrite");
        // ...and the restored body really is the old one.
        let current: Value = request
            .get(&format!("/api/entries/{entry_pid}/variants/en"))
            .await
            .json();
        assert_eq!(current["current_revision"]["title"], "First title");
        assert_eq!(
            current["current_revision"]["blocks"][0]["text"],
            "First body"
        );
    })
    .await;
}

/// An edit made from a superseded revision is refused, and the refusal
/// names the revision that won — the client can then show a real
/// conflict instead of silently destroying someone's paragraph.
///
/// This exercises the *check*, sequentially. True parallel racing is
/// held by the variant row lock plus `UNIQUE (variant_pid, number)`,
/// which a sequential test cannot demonstrate; what it can show is that
/// the second writer is refused rather than merged.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_stale_edit_is_refused_and_the_chain_stays_gapless() {
    request::<App, _, _>(|request, _ctx| async move {
        let site_pid = seed_site_and_type(&request, "conflict").await;
        let (entry_pid, first) = create_entry(&request, &site_pid, "conflicted").await;
        let path = format!("/api/entries/{entry_pid}/variants/en/revisions");

        let winner = request
            .post(&path)
            .json(&json!({ "base_revision_pid": first, "title": "Winner",
                           "blocks": [{ "kind": "paragraph", "text": "mine" }] }))
            .await;
        winner.assert_status_ok();

        // The loser edited from the same base and is refused.
        let loser = request
            .post(&path)
            .json(&json!({ "base_revision_pid": first, "title": "Loser",
                           "blocks": [{ "kind": "paragraph", "text": "also mine" }] }))
            .await;
        assert_eq!(loser.status_code(), 409);
        assert!(
            loser.text().contains("the variant is now at"),
            "the refusal names the competing revision: {}",
            loser.text()
        );

        // Two saves attempted, one accepted: numbers 1 and 2, no gap
        // and no duplicate.
        let history: Value = request.get(&path).await.json();
        let numbers: Vec<i64> = history
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["number"].as_i64().unwrap())
            .collect();
        assert_eq!(numbers, vec![2, 1]);

        // Retrying from the current revision succeeds.
        let current = history.as_array().unwrap()[0]["pid"].as_str().unwrap();
        request
            .post(&path)
            .json(&json!({ "base_revision_pid": current, "title": "Retry",
                           "blocks": [{ "kind": "paragraph", "text": "rebased" }] }))
            .await
            .assert_status_ok();
    })
    .await;
}

/// Malformed bodies are refused by path, and nothing an author sent is
/// silently dropped.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn invalid_bodies_are_refused_by_path() {
    request::<App, _, _>(|request, _ctx| async move {
        let site_pid = seed_site_and_type(&request, "invalid-body").await;
        let path = format!("/api/sites/{site_pid}/entries");
        let post = |body: Value| request.post(&path).json(&body);

        // Unknown block kind, unknown block key, and a mark that runs
        // off the end of its text.
        let response = post(json!({
            "key": "bad", "content_type_key": "article", "title": "Bad",
            "blocks": [
                { "kind": "markdown", "text": "x" },
                { "kind": "paragraph", "text": "y", "html": "<b>z</b>" },
                { "kind": "paragraph", "text": "four",
                  "marks": [{ "kind": "em", "start": 0, "end": 99 }] },
            ],
        }))
        .await;
        assert_eq!(response.status_code(), 422);
        let text = response.text();
        assert!(text.contains("blocks[0].kind must be one of"), "{text}");
        assert!(
            text.contains("blocks[1].html is not a key of a paragraph block"),
            "{text}"
        );
        assert!(text.contains("blocks[2].marks[0] range 0..99"), "{text}");

        // Unknown field key, wrong value kind, and an out-of-scope
        // entity reference.
        let response = post(json!({
            "key": "bad2", "content_type_key": "article", "title": "Bad",
            "fields": {
                "nonexistent": "x",
                "section": "opinion",
                "about": format!("person:{}", uuid::Uuid::new_v4()),
            },
        }))
        .await;
        assert_eq!(response.status_code(), 422);
        let text = response.text();
        assert!(text.contains("fields.nonexistent is not a field"), "{text}");
        assert!(text.contains("fields.section must be one of"), "{text}");
        assert!(
            text.contains("fields.about must reference one of"),
            "{text}"
        );

        // A title is required; a draft's *fields* may be incomplete,
        // but the thing has to be called something.
        let response =
            post(json!({ "key": "bad3", "content_type_key": "article", "title": " " })).await;
        assert_eq!(response.status_code(), 422);
        assert!(response.text().contains("title is required"));

        // An unknown content type has no fields to validate against.
        let response =
            post(json!({ "key": "bad4", "content_type_key": "ghost", "title": "x" })).await;
        assert_eq!(response.status_code(), 422);
        assert!(response.text().contains("is not a type on this site"));
    })
    .await;
}

/// HTML reaching the one place it may appear is sanitized **on write**,
/// and the response says it was altered rather than implying the markup
/// was stored verbatim.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn embedded_html_is_sanitized_before_storage() {
    request::<App, _, _>(|request, _ctx| async move {
        let site_pid = seed_site_and_type(&request, "sanitize").await;
        let created: Value = request
            .post(&format!("/api/sites/{site_pid}/entries"))
            .json(&json!({
                "key": "embedded", "content_type_key": "article", "title": "Embedded",
                "blocks": [{
                    "kind": "embed",
                    "url": "https://player.example.test/v/1",
                    "html": "<p>caption</p><script>alert(1)</script><a href=\"javascript:alert(2)\">x</a>",
                }],
            }))
            .await
            .json();
        assert_eq!(created["blocks_sanitized"], 1);

        let revision: Value = request
            .get(&format!("/api/revisions/{}", created["revision_pid"].as_str().unwrap()))
            .await
            .json();
        let html = revision["blocks"][0]["html"].as_str().unwrap();
        assert!(html.contains("<p>caption</p>"), "safe markup survives: {html}");
        assert!(!html.contains("<script"), "script never reaches storage: {html}");
        assert!(!html.contains("javascript:"), "js: URL never reaches storage: {html}");
    })
    .await;
}

/// References are extracted on save, so "where used" is answerable and
/// a delete that would break a live reference is refused.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn references_drive_usage_and_delete_refusal() {
    request::<App, _, _>(|request, _ctx| async move {
        let site_pid = seed_site_and_type(&request, "references").await;
        let (target_pid, _) = create_entry(&request, &site_pid, "target").await;
        let asset = uuid::Uuid::new_v4().to_string();

        // A referring entry: one entry reference in a field, one asset
        // reference in a block.
        let referrer: Value = request
            .post(&format!("/api/sites/{site_pid}/entries"))
            .json(&json!({
                "key": "referrer", "content_type_key": "article", "title": "Referrer",
                "blocks": [{ "kind": "image", "asset": asset }],
                "fields": { "related": [target_pid] },
            }))
            .await
            .json();
        assert_eq!(referrer["references"], 2);
        let referrer_pid = referrer["pid"].as_str().unwrap().to_string();

        // "Where used" answers for both the entry and the asset.
        let usage: Value = request
            .get(&format!("/api/entries/{target_pid}/usage"))
            .await
            .json();
        assert_eq!(usage["referrers"].as_array().unwrap().len(), 1);
        assert_eq!(usage["referrers"][0]["entry_key"], "referrer");
        assert_eq!(usage["referrers"][0]["field_key"], "related");

        let asset_usage: Value = request
            .get(&format!("/api/assets/{asset}/usage"))
            .await
            .json();
        assert_eq!(asset_usage["referrers"][0]["field_key"], "blocks[0].asset");

        // Deleting the referenced entry is refused, naming the referrer
        // and the escape hatch.
        let refused = request.delete(&format!("/api/entries/{target_pid}")).await;
        assert_eq!(refused.status_code(), 409);
        assert!(refused.text().contains("referrer"), "{}", refused.text());
        assert!(refused.text().contains("force=true"), "{}", refused.text());

        // Forcing without a reason is still refused: the audit row is
        // the point of the override.
        let unexplained = request
            .delete(&format!("/api/entries/{target_pid}?force=true"))
            .await;
        assert_eq!(unexplained.status_code(), 422);
        assert!(unexplained.text().contains("requires a reason"));

        // Drop the reference in a new revision, and the delete succeeds:
        // usage follows the *current* revision, not history.
        let current: Value = request
            .get(&format!("/api/entries/{referrer_pid}/variants/en"))
            .await
            .json();
        request
            .post(&format!(
                "/api/entries/{referrer_pid}/variants/en/revisions"
            ))
            .json(&json!({
                "base_revision_pid": current["variant"]["current_revision_pid"],
                "title": "Referrer",
                "blocks": [{ "kind": "paragraph", "text": "no references now" }],
            }))
            .await
            .assert_status_ok();

        let usage: Value = request
            .get(&format!("/api/entries/{target_pid}/usage"))
            .await
            .json();
        assert!(usage["referrers"].as_array().unwrap().is_empty());
        request
            .delete(&format!("/api/entries/{target_pid}"))
            .await
            .assert_status_ok();
        assert_eq!(
            request
                .get(&format!("/api/entries/{target_pid}"))
                .await
                .status_code(),
            404
        );
    })
    .await;
}

/// A locale variant is its own unit of work, and only the site's
/// declared locales exist.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn variants_are_per_declared_locale() {
    request::<App, _, _>(|request, _ctx| async move {
        let site_pid = seed_site_and_type(&request, "locales").await;
        let (entry_pid, _) = create_entry(&request, &site_pid, "multilingual").await;

        // The site declares en / fr / fr-CA (see the shared fixture).
        request
            .post(&format!("/api/entries/{entry_pid}/variants"))
            .json(&json!({ "locale": "fr", "body": { "title": "Titre" } }))
            .await
            .assert_status_ok();

        // A second fr variant is a conflict, not a second copy.
        let clash = request
            .post(&format!("/api/entries/{entry_pid}/variants"))
            .json(&json!({ "locale": "fr" }))
            .await;
        assert_eq!(clash.status_code(), 409);

        // An undeclared locale is refused, naming what the site does
        // declare.
        let undeclared = request
            .post(&format!("/api/entries/{entry_pid}/variants"))
            .json(&json!({ "locale": "de" }))
            .await;
        assert_eq!(undeclared.status_code(), 422);
        assert!(undeclared.text().contains("is not declared by site"));

        // Each variant carries its own revision chain.
        let detail: Value = request
            .get(&format!("/api/entries/{entry_pid}"))
            .await
            .json();
        let locales: Vec<&str> = detail["variants"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["locale"].as_str().unwrap())
            .collect();
        assert_eq!(locales, vec!["en", "fr"]);
        let fr: Value = request
            .get(&format!("/api/entries/{entry_pid}/variants/fr"))
            .await
            .json();
        assert_eq!(fr["current_revision"]["title"], "Titre");
        assert_eq!(fr["current_revision"]["number"], 1);
        assert!(fr["variant"]["published_revision_pid"].is_null());
    })
    .await;
}

/// The reasoned override: a referenced entry *can* be deleted, but the
/// deletion records every reference it broke, so the cleanup is a
/// work-list rather than a mystery (CMS-R5).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_forced_delete_records_what_it_broke() {
    request::<App, _, _>(|request, _ctx| async move {
        let site_pid = seed_site_and_type(&request, "force-delete").await;
        let (target_pid, _) = create_entry(&request, &site_pid, "doomed").await;
        request
            .post(&format!("/api/sites/{site_pid}/entries"))
            .json(&json!({
                "key": "keeper", "content_type_key": "article", "title": "Keeper",
                "fields": { "related": [target_pid] },
            }))
            .await
            .assert_status_ok();

        request
            .delete(&format!(
                "/api/entries/{target_pid}?force=true&reason=legal+takedown"
            ))
            .await
            .assert_status_ok();

        let trail: Value = request
            .get(&format!("/api/audits/{target_pid}"))
            .await
            .json();
        let forced = trail
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["action"] == "force_deleted")
            .expect("a forced delete is audited as such, not as an ordinary delete");
        assert_eq!(forced["snapshot"]["reason"], "legal takedown");
        assert_eq!(
            forced["snapshot"]["broken_references"][0]["entry_key"], "keeper",
            "the audit row names what is now broken"
        );
    })
    .await;
}
