//! The synthetic corpus (CMS-T24).
//!
//! The seed exists to make the derived views demonstrable, so the test
//! that matters is not "did rows appear" but **"does every content
//! health rule actually fire"**. Both defects found while building this
//! fixture were invisible to a row count: a variant inserted under a
//! freshly-minted pid left every route and reference pointing at
//! nothing, and a field that became required after the content was
//! written went unreported because the health rule only inspected
//! fields that were present.

use content_management_system_service::app::App;
use content_management_system_service::tasks::seed::Seed;
use loco_rs::prelude::*;
use loco_rs::testing::prelude::request;
use serde_json::Value;
use serial_test::serial;

/// Every rule the health view knows about.
const RULES: [&str; 10] = [
    "image_alt_text_missing",
    "seo_metadata_missing",
    "broken_reference",
    "orphan_asset",
    "stale_content",
    "stale_translation",
    "stuck_in_review",
    "approved_not_published",
    "needs_migration",
    "route_hazard",
];

/// Run the seed task against the test app.
async fn run_seed(ctx: &AppContext) {
    Seed.run(ctx, &task::Vars::default())
        .await
        .expect("seed task failed");
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn the_seeded_site_demonstrates_every_health_rule() {
    request::<App, _, _>(|request, ctx| async move {
        run_seed(&ctx).await;
        let sites: Value = request.get("/api/sites").await.json();
        let site_pid = sites
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["key"] == "demo")
            .expect("the seed created a `demo` site")["pid"]
            .as_str()
            .unwrap()
            .to_string();

        let health: Value = request
            .get(&format!("/api/sites/{site_pid}/insights/health"))
            .await
            .json();
        let fired: Vec<&str> = health["by_rule"]
            .as_array()
            .unwrap()
            .iter()
            .map(|group| group["rule"].as_str().unwrap())
            .collect();
        for rule in RULES {
            assert!(
                fired.contains(&rule),
                "the corpus plants no instance of `{rule}`; fired: {fired:?}"
            );
        }
        // Each planted finding names the entry that demonstrates it, so
        // a reader can trace a finding back to the fixture.
        let planted: Vec<&str> = health["by_rule"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|group| group["findings"].as_array().unwrap())
            .filter_map(|finding| finding["subject"].as_str())
            .filter(|subject| subject.starts_with("plant-"))
            .collect();
        assert!(planted.len() >= 8, "planted subjects: {planted:?}");

        // A demo that is all findings teaches the wrong lesson: most of
        // the corpus is healthy.
        let findings = health["findings_total"].as_u64().unwrap();
        let published = health["published_variants"].as_u64().unwrap();
        assert!(
            findings < published,
            "{findings} findings against {published} published variants — the corpus should be \
             mostly healthy"
        );
    })
    .await;
}

/// The regression check for the fixture bug that row counts could not
/// see: routes and references must point at variants that exist.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn nothing_the_corpus_writes_points_at_a_missing_row() {
    request::<App, _, _>(|request, ctx| async move {
        run_seed(&ctx).await;
        let sites: Value = request.get("/api/sites").await.json();
        let site_pid = sites
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["key"] == "demo")
            .expect("the seed created a `demo` site")["pid"]
            .as_str()
            .unwrap()
            .to_string();

        // Locale coverage walks every entry's variants, so a dangling
        // variant would show up as content that exists in no locale.
        let coverage: Value = request
            .get(&format!("/api/sites/{site_pid}/locale-coverage"))
            .await
            .json();
        let rows = coverage["coverage"].as_array().unwrap();
        let english = rows
            .iter()
            .find(|row| row["locale"] == "en")
            .expect("the source locale is covered");
        assert_eq!(
            english["entries_started"], english["entries_total"],
            "every entry has an English variant"
        );
        let french = rows.iter().find(|row| row["locale"] == "fr").unwrap();
        assert!(
            french["entries_published"].as_u64().unwrap() >= 12,
            "the French translations should be published, not dangling: {french}"
        );

        // The seeded redirect chain collapses to its destination in one
        // hop rather than bouncing a reader through four.
        let followed = request.get("/delivery/demo/en/legacy-a").await;
        assert_eq!(followed.status_code(), 301);
        assert!(
            followed
                .headers()
                .get("location")
                .unwrap()
                .to_str()
                .unwrap()
                .ends_with("/guide-getting-started"),
            "the chain should collapse to its final target"
        );

        // The sitemap carries published pages only.
        let sitemap = request.get("/delivery/demo/sitemap.xml").await.text();
        assert!(sitemap.contains("/guide-permissions"));
        for unpublished in ["draft-pricing-refresh", "plant-stuck-in-review"] {
            assert!(
                !sitemap.contains(unpublished),
                "{unpublished} is not published and must not be in the sitemap"
            );
        }
    })
    .await;
}

/// A rerun must not double the corpus — an operator who runs the task
/// twice should get the same demo, not two of it.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn seeding_twice_leaves_one_corpus() {
    request::<App, _, _>(|request, ctx| async move {
        run_seed(&ctx).await;
        let sites: Value = request.get("/api/sites").await.json();
        let site_pid = sites
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["key"] == "demo")
            .expect("the seed created a `demo` site")["pid"]
            .as_str()
            .unwrap()
            .to_string();

        // Read a real count first, so this cannot pass by comparing two
        // identical errors.
        let path = format!("/api/sites/{site_pid}/entries");
        let listed: Value = request.get(&path).await.json();
        let first = listed
            .as_array()
            .expect("the entry listing is a list")
            .len();
        assert!(first > 20, "the corpus should be substantial, got {first}");

        run_seed(&ctx).await;
        let listed: Value = request.get(&path).await.json();
        let second = listed.as_array().unwrap().len();
        assert_eq!(first, second, "a second seed should be a no-op");

        // And the site itself was not duplicated.
        let sites: Value = request.get("/api/sites").await.json();
        assert_eq!(
            sites
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| row["key"] == "demo")
                .count(),
            1
        );
    })
    .await;
}
