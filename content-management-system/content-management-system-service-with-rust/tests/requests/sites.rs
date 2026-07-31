//! The site + template journey (CMS-R1): declare a namespace, declare
//! its region contracts, and prove the refusals that keep the
//! namespace coherent — key clashes, unwalkable fallback chains,
//! and deletes that would orphan something.

use content_management_system_service::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_key, a_site_payload, an_organization, seed_site};

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn site_journey_end_to_end() {
    request::<App, _, _>(|request, _ctx| async move {
        let key = a_key("journey");
        let mut payload = a_site_payload(&key);
        payload["owner_ref"] = json!(an_organization());
        let created = request.post("/api/sites").json(&payload).await;
        created.assert_status_ok();
        let site_pid = created.json::<Value>()["pid"].as_str().unwrap().to_string();

        // A new site is restricted until someone says otherwise.
        let detail: Value = request.get(&format!("/api/sites/{site_pid}")).await.json();
        assert_eq!(detail["site"]["visibility"], "restricted");
        assert_eq!(detail["site"]["default_locale"], "en");
        assert!(detail["templates"].as_array().unwrap().is_empty());
        assert!(detail["content_types"].as_array().unwrap().is_empty());

        // The key is the delivery namespace's handle: taking it twice
        // is a conflict, not a second site.
        let clash = request.post("/api/sites").json(&a_site_payload(&key)).await;
        assert_eq!(clash.status_code(), 409);

        // Declare a region contract, then a second with the same key.
        let template = request
            .post(&format!("/api/sites/{site_pid}/templates"))
            .json(&json!({
                "key": "article",
                "name": "Article layout",
                "regions": [
                    { "key": "body", "label": "Body",
                      "allowed_block_kinds": ["heading", "paragraph"], "min": 1 }
                ],
                "applies_to_type_keys": ["article"],
            }))
            .await;
        template.assert_status_ok();
        let template_pid = template.json::<Value>()["pid"].as_str().unwrap().to_string();
        let clash = request
            .post(&format!("/api/sites/{site_pid}/templates"))
            .json(&json!({
                "key": "article", "name": "Another", "regions": [{ "key": "body", "label": "Body" }],
            }))
            .await;
        assert_eq!(clash.status_code(), 409);

        // A content type may name the template...
        let content_type = request
            .post(&format!("/api/sites/{site_pid}/content-types"))
            .json(&json!({
                "key": "article",
                "name": "Article",
                "template_key": "article",
                "fields": [{ "key": "summary", "label": "Summary", "kind": "text" }],
            }))
            .await;
        content_type.assert_status_ok();
        let type_pid = content_type.json::<Value>()["pid"].as_str().unwrap().to_string();

        // ...and while it does, the template cannot be deleted, and the
        // site cannot be deleted out from under either of them.
        assert_eq!(
            request.delete(&format!("/api/templates/{template_pid}")).await.status_code(),
            409
        );
        assert_eq!(
            request.delete(&format!("/api/sites/{site_pid}")).await.status_code(),
            409
        );

        // Tear down in dependency order and the deletes succeed.
        request
            .delete(&format!("/api/content-types/{type_pid}"))
            .await
            .assert_status_ok();
        request
            .delete(&format!("/api/templates/{template_pid}"))
            .await
            .assert_status_ok();
        request
            .delete(&format!("/api/sites/{site_pid}"))
            .await
            .assert_status_ok();
        // ...and the site is then gone from reads.
        assert_eq!(
            request.get(&format!("/api/sites/{site_pid}")).await.status_code(),
            404
        );
    })
    .await;
}

/// A fallback chain that cannot be walked is refused at write time,
/// because the alternative is a delivery request that falls off the end
/// of the list with nothing to serve (CMS-R14).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn unwalkable_locale_configurations_are_refused() {
    request::<App, _, _>(|request, _ctx| async move {
        // Chain never reaches the default locale.
        let mut payload = a_site_payload(&a_key("bad-chain"));
        payload["fallback_chains"] = json!({ "fr": ["fr-CA"] });
        let response = request.post("/api/sites").json(&payload).await;
        assert_eq!(response.status_code(), 422);
        assert!(
            response.text().contains("must end at the default locale"),
            "the refusal names the problem: {}",
            response.text()
        );

        // Undeclared default locale.
        let mut payload = a_site_payload(&a_key("bad-default"));
        payload["default_locale"] = json!("de");
        assert_eq!(
            request
                .post("/api/sites")
                .json(&payload)
                .await
                .status_code(),
            422
        );

        // A cycle would never terminate at resolution time.
        let mut payload = a_site_payload(&a_key("cycle"));
        payload["fallback_chains"] = json!({ "fr-CA": ["fr", "fr-CA", "en"] });
        assert_eq!(
            request
                .post("/api/sites")
                .json(&payload)
                .await
                .status_code(),
            422
        );

        // A malformed locale code.
        let mut payload = a_site_payload(&a_key("bad-code"));
        payload["locales"] = json!(["en", "FR"]);
        assert_eq!(
            request
                .post("/api/sites")
                .json(&payload)
                .await
                .status_code(),
            422
        );
    })
    .await;
}

/// Flipping `restricted → public` is the one edit that changes who may
/// read this site without a credential, so it is audited as its own
/// action rather than an anonymous "updated" (CMS-D7).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_visibility_change_is_audited_as_such() {
    request::<App, _, _>(|request, _ctx| async move {
        let key = a_key("visibility");
        let site_pid = seed_site(&request, &key).await;

        let mut payload = a_site_payload(&key);
        payload["visibility"] = json!("public");
        let updated = request
            .put(&format!("/api/sites/{site_pid}"))
            .json(&payload)
            .await;
        updated.assert_status_ok();
        assert_eq!(updated.json::<Value>()["visibility"], "public");

        let trail: Value = request.get(&format!("/api/audits/{site_pid}")).await.json();
        let actions: Vec<&str> = trail
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["action"].as_str().unwrap())
            .collect();
        assert!(
            actions.contains(&"visibility_changed"),
            "expected a visibility_changed row, got {actions:?}"
        );
        let change = trail
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["action"] == "visibility_changed")
            .unwrap();
        assert_eq!(change["snapshot"]["visibility_from"], "restricted");
        assert_eq!(change["snapshot"]["visibility_to"], "public");
    })
    .await;
}

/// The family 404 pin: an unknown or malformed pid is a 404, never a
/// 500 from an unmapped model error.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn unknown_pids_are_404() {
    request::<App, _, _>(|request, _ctx| async move {
        let unknown = uuid::Uuid::new_v4();
        for path in [
            format!("/api/sites/{unknown}"),
            format!("/api/templates/{unknown}"),
            format!("/api/content-types/{unknown}"),
            "/api/sites/not-a-uuid".to_string(),
        ] {
            assert_eq!(
                request.get(&path).await.status_code(),
                404,
                "{path} should be 404"
            );
        }
    })
    .await;
}
