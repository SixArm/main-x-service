//! Content types and the compatibility gate (CMS-R2).
//!
//! The gate is the thing worth pinning: an operator editing a live
//! content type must be told, before the write lands, whether stored
//! content survives it — and a breaking edit must be impossible to
//! make by accident.

use content_management_system_service::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_key, seed_site};

/// The starting declaration: one optional text field.
fn initial_fields() -> Value {
    json!([{ "key": "summary", "label": "Summary", "kind": "text" }])
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn the_compatibility_gate_classifies_confirms_and_versions() {
    request::<App, _, _>(|request, _ctx| async move {
        let site_pid = seed_site(&request, &a_key("types")).await;
        let created = request
            .post(&format!("/api/sites/{site_pid}/content-types"))
            .json(&json!({ "key": "article", "name": "Article", "fields": initial_fields() }))
            .await;
        created.assert_status_ok();
        let pid = created.json::<Value>()["pid"].as_str().unwrap().to_string();
        assert_eq!(
            request
                .get(&format!("/api/content-types/{pid}"))
                .await
                .json::<Value>()["schema_version"],
            1
        );

        // Dry run: adding an optional field cannot invalidate anything.
        let additive = json!([
            { "key": "summary", "label": "Summary", "kind": "text" },
            { "key": "standfirst", "label": "Standfirst", "kind": "text" },
        ]);
        let preview: Value = request
            .post(&format!("/api/content-types/{pid}/compatibility"))
            .json(&json!({ "fields": additive }))
            .await
            .json();
        assert_eq!(preview["level"], "additive");

        // Applying it bumps the schema version.
        let applied: Value = request
            .put(&format!("/api/content-types/{pid}"))
            .json(&json!({ "key": "article", "name": "Article", "fields": additive }))
            .await
            .json();
        assert_eq!(applied["compatibility"]["level"], "additive");
        assert_eq!(applied["schema_version"], 2);

        // A no-op edit does not bump the version — a version that moves
        // for nothing tells an operator nothing.
        let noop: Value = request
            .put(&format!("/api/content-types/{pid}"))
            .json(&json!({ "key": "article", "name": "Article", "fields": additive }))
            .await
            .json();
        assert_eq!(noop["schema_version"], 2);

        // Making a field required is a tightening: applied, and
        // reported so the operator expects needs_migration findings.
        let tightened = json!([
            { "key": "summary", "label": "Summary", "kind": "text" },
            { "key": "standfirst", "label": "Standfirst", "kind": "text", "required": true },
        ]);
        let applied: Value = request
            .put(&format!("/api/content-types/{pid}"))
            .json(&json!({ "key": "article", "name": "Article", "fields": tightened }))
            .await
            .json();
        assert_eq!(applied["compatibility"]["level"], "tightening");
        assert_eq!(applied["schema_version"], 3);

        // A breaking edit: dropping a field. Refused without an
        // explicit confirmation, and the refusal names what breaks.
        let breaking = json!([{ "key": "summary", "label": "Summary", "kind": "text" }]);
        let refused = request
            .put(&format!("/api/content-types/{pid}"))
            .json(&json!({ "key": "article", "name": "Article", "fields": breaking }))
            .await;
        assert_eq!(refused.status_code(), 422);
        assert!(
            refused.text().contains("standfirst") && refused.text().contains("field removed"),
            "the refusal names the breaking change: {}",
            refused.text()
        );

        // Confirmed but unexplained is still refused: the audit row is
        // the point of the confirmation.
        let unexplained = request
            .put(&format!("/api/content-types/{pid}"))
            .json(&json!({
                "key": "article", "name": "Article", "fields": breaking,
                "confirm_breaking": true,
            }))
            .await;
        assert_eq!(unexplained.status_code(), 422);
        assert!(unexplained.text().contains("reason"));

        // Confirmed with a reason: applied, versioned, and audited with
        // the classification and the reason.
        let applied: Value = request
            .put(&format!("/api/content-types/{pid}"))
            .json(&json!({
                "key": "article", "name": "Article", "fields": breaking,
                "confirm_breaking": true, "reason": "standfirst was never used",
            }))
            .await
            .json();
        assert_eq!(applied["compatibility"]["level"], "breaking");
        assert_eq!(applied["schema_version"], 4);

        let trail: Value = request.get(&format!("/api/audits/{pid}")).await.json();
        let schema_change = trail
            .as_array()
            .unwrap()
            .iter()
            .find(|row| {
                row["action"] == "schema_changed" && row["snapshot"]["compatibility"] == "breaking"
            })
            .expect("the breaking edit is audited as such");
        assert_eq!(schema_change["snapshot"]["confirmed_breaking"], true);
        assert_eq!(
            schema_change["snapshot"]["reason"],
            "standfirst was never used"
        );
    })
    .await;
}

/// A declaration that could never be satisfied is refused up front,
/// with every problem in one response rather than one per round trip.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn invalid_declarations_are_refused() {
    request::<App, _, _>(|request, _ctx| async move {
        let site_pid = seed_site(&request, &a_key("invalid")).await;
        let path = format!("/api/sites/{site_pid}/content-types");

        // Unknown field kind, reserved key, and a choice with no options.
        let response = request
            .post(&path)
            .json(&json!({
                "key": "bad", "name": "Bad",
                "fields": [
                    { "key": "body", "label": "Body", "kind": "markdown" },
                    { "key": "pid", "label": "Pid", "kind": "text" },
                    { "key": "section", "label": "Section", "kind": "choice" },
                ],
            }))
            .await;
        assert_eq!(response.status_code(), 422);
        let text = response.text();
        assert!(text.contains("kind must be one of"), "{text}");
        assert!(text.contains("is reserved"), "{text}");
        assert!(text.contains("options is required"), "{text}");

        // An entity_ref pointing at a type the family does not register
        // could never be satisfied.
        let response = request
            .post(&path)
            .json(&json!({
                "key": "listing", "name": "Listing",
                "fields": [{ "key": "about", "label": "About", "kind": "entity_ref",
                             "validation": { "entity_types": ["kourse"] } }],
            }))
            .await;
        assert_eq!(response.status_code(), 422);
        assert!(response.text().contains("is not a known entity type"));

        // A template_key that names no template on this site.
        let response = request
            .post(&path)
            .json(&json!({
                "key": "page", "name": "Page", "template_key": "nonexistent",
                "fields": [{ "key": "summary", "label": "Summary", "kind": "text" }],
            }))
            .await;
        assert_eq!(response.status_code(), 422);
        assert!(response.text().contains("does not name a template"));

        // An empty field set declares nothing.
        let response = request
            .post(&path)
            .json(&json!({ "key": "empty", "name": "Empty", "fields": [] }))
            .await;
        assert_eq!(response.status_code(), 422);
    })
    .await;
}
