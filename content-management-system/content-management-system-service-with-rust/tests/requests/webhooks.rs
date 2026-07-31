//! Outbound webhook registration (CMS-R23).
//!
//! Delivery itself lives in its own test binary
//! (`tests/webhook_delivery.rs`) because the event transport is
//! resolved once per process: a suite that runs under the default
//! in-memory transport cannot also exercise the durable one.
//! Registration and the honest refusal belong here, where the default
//! transport is what a caller actually meets.

use content_management_system_service::app::App;
use content_management_system_service::rules::webhook;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_key, a_site_payload};

/// The secret is shown once and never again, and the URL rule holds.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_subscription_shows_its_secret_once_and_never_again() {
    request::<App, _, _>(|request, _ctx| async move {
        let created: Value = request
            .post("/api/sites")
            .json(&a_site_payload(&a_key("hook")))
            .await
            .json();
        let site_pid = created["pid"].as_str().unwrap().to_string();

        let registered: Value = request
            .post(&format!("/api/sites/{site_pid}/webhooks"))
            .json(&json!({
                "name": "Search index", "url": "https://hooks.example.test/cms",
                "event_kinds": ["variant_published"],
            }))
            .await
            .json();
        let secret = registered["secret"].as_str().unwrap().to_string();
        assert_eq!(secret.len(), 64, "256 bits of secret");
        assert_eq!(registered["signature_header"], webhook::SIGNATURE_HEADER);

        // No read returns it.
        let listed = request
            .get(&format!("/api/sites/{site_pid}/webhooks"))
            .await;
        assert!(
            !listed.text().contains(&secret),
            "a read must never return the secret"
        );
        let rows: Value = listed.json();
        assert_eq!(rows["webhooks"][0]["url"], "https://hooks.example.test/cms");

        // Nor does the audit trail — the subscription is recorded, the
        // secret is not (security invariant 9).
        let trail = request.get("/api/audits/recent").await;
        assert!(!trail.text().contains(&secret));
        let audits: Value = trail.json();
        assert!(
            audits
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["entity"] == "webhook" && row["action"] == "created")
        );

        // Plain HTTP to a public host is refused, with a reason.
        let refused = request
            .post(&format!("/api/sites/{site_pid}/webhooks"))
            .json(&json!({ "name": "Insecure", "url": "http://hooks.example.test/cms" }))
            .await;
        assert_eq!(refused.status_code(), 422);
        assert!(refused.text().contains("https"));

        // As is a URL carrying credentials.
        assert_eq!(
            request
                .post(&format!("/api/sites/{site_pid}/webhooks"))
                .json(&json!({ "name": "Creds", "url": "https://u:p@hooks.example.test/cms" }))
                .await
                .status_code(),
            422
        );

        // Withdrawing it takes it out of the listing.
        let hook_pid = registered["pid"].as_str().unwrap();
        request
            .delete(&format!("/api/webhooks/{hook_pid}"))
            .await
            .assert_status_ok();
        let after: Value = request
            .get(&format!("/api/sites/{site_pid}/webhooks"))
            .await
            .json();
        assert!(after["webhooks"].as_array().unwrap().is_empty());
    })
    .await;
}

/// With the default transport there is nothing durable to deliver
/// from, and the endpoint says so instead of delivering a subset that
/// would disappear on restart.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn dispatch_refuses_rather_than_silently_delivering_a_subset() {
    request::<App, _, _>(|request, _ctx| async move {
        let refused = request.post("/api/webhooks/dispatch").await;
        assert_eq!(refused.status_code(), 422);
        assert!(
            refused.text().contains("CMS_EVENT_TRANSPORT=outbox"),
            "the refusal names the setting that fixes it: {}",
            refused.text()
        );
    })
    .await;
}
