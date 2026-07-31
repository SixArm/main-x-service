//! Webhook **delivery** (CMS-R23) — its own test binary.
//!
//! Two reasons this is not in the `requests` suite:
//!
//! 1. The event transport is resolved once per process (`OnceLock`), so
//!    a binary that has already run under the default in-memory
//!    transport cannot switch to the durable one. This binary sets
//!    `CMS_EVENT_TRANSPORT=outbox` before anything boots — the same
//!    reason the enforcement suite has its own binary.
//! 2. The tests run a **real receiver** on loopback rather than
//!    asserting about a mock, because the property worth pinning is
//!    that a third party can verify what we send. A mock that returns
//!    whatever we tell it proves nothing about the signature.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use content_management_system_service::app::App;
use content_management_system_service::rules::webhook;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

/// Select the durable transport before anything reads it.
///
/// Set per test rather than once, because the first test to boot the
/// app is the one that fixes the value and test order is not
/// guaranteed. Setting it repeatedly to the same value is harmless.
fn use_the_durable_transport() {
    // SAFETY: these tests are `#[serial]`, so no other thread is
    // reading the environment while this runs.
    unsafe { std::env::set_var("CMS_EVENT_TRANSPORT", "outbox") };
}

/// One request as the receiver saw it.
#[derive(Clone, Debug)]
struct Received {
    body: String,
    signature: String,
    timestamp: i64,
    event_id: String,
}

/// Start a receiver that records what it is sent and answers `status`.
async fn receiver(status: StatusCode) -> (SocketAddr, Arc<Mutex<Vec<Received>>>) {
    type Seen = Arc<Mutex<Vec<Received>>>;
    async fn handle(
        State((seen, status)): State<(Seen, StatusCode)>,
        headers: HeaderMap,
        body: String,
    ) -> StatusCode {
        let header = |name: &str| {
            headers
                .get(name)
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default()
                .to_string()
        };
        seen.lock().unwrap().push(Received {
            body,
            signature: header(webhook::SIGNATURE_HEADER),
            timestamp: header(webhook::TIMESTAMP_HEADER)
                .parse()
                .unwrap_or_default(),
            event_id: header(webhook::EVENT_ID_HEADER),
        });
        status
    }

    let seen: Seen = Arc::new(Mutex::new(Vec::new()));
    let app = Router::new()
        .route("/hook", axum::routing::post(handle))
        .with_state((seen.clone(), status));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (addr, seen)
}

/// A site with one entry, so the outbox holds real events.
async fn seed(request: &axum_test::TestServer, prefix: &str) -> String {
    let key = format!("{prefix}-{}", uuid::Uuid::new_v4().simple());
    let created: Value = request
        .post("/api/sites")
        .json(&json!({
            "key": key, "name": "Test site", "default_locale": "en", "locales": ["en"],
        }))
        .await
        .json();
    let site_pid = created["pid"].as_str().unwrap().to_string();
    request
        .post(&format!("/api/sites/{site_pid}/content-types"))
        .json(&json!({
            "key": "note", "name": "Note", "routable": false,
            "fields": [{ "key": "summary", "label": "Summary", "kind": "text" }],
        }))
        .await
        .assert_status_ok();
    request
        .post(&format!("/api/sites/{site_pid}/entries"))
        .json(&json!({
            "key": "hello", "content_type_key": "note", "title": "Hello",
            "blocks": [{ "kind": "paragraph", "text": "hello" }],
        }))
        .await
        .assert_status_ok();
    site_pid
}

/// The property that matters: a receiver can verify what we sent.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_delivery_is_signed_so_the_receiver_can_verify_it() {
    use_the_durable_transport();
    request::<App, _, _>(|request, _ctx| async move {
        let site_pid = seed(&request, "sign").await;
        let (addr, seen) = receiver(StatusCode::NO_CONTENT).await;

        let registered: Value = request
            .post(&format!("/api/sites/{site_pid}/webhooks"))
            .json(&json!({ "name": "Local receiver", "url": format!("http://{addr}/hook") }))
            .await
            .json();
        let secret = registered["secret"].as_str().unwrap().to_string();

        let dispatched = request.post("/api/webhooks/dispatch").await;
        dispatched.assert_status_ok();
        let report: Value = dispatched.json();
        assert!(
            report["delivered"].as_u64().unwrap() > 0,
            "the seeded entry's events should have been delivered: {report}"
        );

        let received = seen.lock().unwrap().clone();
        assert!(!received.is_empty(), "the receiver saw nothing");
        for delivery in &received {
            assert!(
                webhook::verify(
                    &secret,
                    delivery.timestamp,
                    &delivery.body,
                    &delivery.signature
                ),
                "the receiver could not verify the signature"
            );
            // The timestamp is inside the signature, so a captured
            // delivery cannot be replayed with a fresh one.
            assert!(!webhook::verify(
                &secret,
                delivery.timestamp + 1,
                &delivery.body,
                &delivery.signature
            ));
            // An event id is carried so a receiver can dedupe.
            assert!(!delivery.event_id.is_empty());
            let body: Value = serde_json::from_str(&delivery.body).unwrap();
            assert!(body.get("kind").is_some(), "the body is an event envelope");
        }

        // A rerun does not re-send what was delivered.
        let again: Value = request.post("/api/webhooks/dispatch").await.json();
        assert_eq!(again["delivered"], 0, "a rerun must not duplicate");
        assert_eq!(seen.lock().unwrap().len(), received.len());

        // The attempt log shows the deliveries.
        let hook_pid = registered["pid"].as_str().unwrap();
        let log: Value = request
            .get(&format!("/api/webhooks/{hook_pid}/deliveries"))
            .await
            .json();
        assert_eq!(log["webhook"]["consecutive_failures"], 0);
        assert!(
            log["deliveries"]
                .as_array()
                .unwrap()
                .iter()
                .all(|row| row["state"] == "delivered")
        );
    })
    .await;
}

/// A receiver that understood and refused is recorded, not retried.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_rejecting_receiver_is_recorded_and_not_retried() {
    use_the_durable_transport();
    request::<App, _, _>(|request, _ctx| async move {
        let site_pid = seed(&request, "reject").await;
        let (addr, seen) = receiver(StatusCode::BAD_REQUEST).await;
        let registered: Value = request
            .post(&format!("/api/sites/{site_pid}/webhooks"))
            .json(&json!({ "name": "Rejects", "url": format!("http://{addr}/hook") }))
            .await
            .json();
        let hook_pid = registered["pid"].as_str().unwrap().to_string();

        let report: Value = request.post("/api/webhooks/dispatch").await.json();
        assert_eq!(report["delivered"], 0);
        assert!(report["abandoned"].as_u64().unwrap() > 0);
        let attempts = seen.lock().unwrap().len();
        assert!(attempts > 0);

        // A 400 means the receiver understood and refused; repeating the
        // same request is noise, so a rerun sends nothing more.
        request
            .post("/api/webhooks/dispatch")
            .await
            .assert_status_ok();
        assert_eq!(seen.lock().unwrap().len(), attempts);

        let log: Value = request
            .get(&format!("/api/webhooks/{hook_pid}/deliveries"))
            .await
            .json();
        let rows = log["deliveries"].as_array().unwrap();
        assert!(rows.iter().all(|row| row["state"] == "abandoned"));
        assert!(rows.iter().all(|row| row["status_code"] == 400));
        assert!(log["webhook"]["consecutive_failures"].as_i64().unwrap() > 0);
    })
    .await;
}

/// A receiver that is down is retried later, not abandoned — and not
/// hammered in the meantime.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn an_unreachable_receiver_waits_out_its_backoff() {
    use_the_durable_transport();
    request::<App, _, _>(|request, _ctx| async move {
        let site_pid = seed(&request, "down").await;
        // A port nothing is listening on: the connection is refused.
        let dead = {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            drop(listener);
            addr
        };
        let registered: Value = request
            .post(&format!("/api/sites/{site_pid}/webhooks"))
            .json(&json!({ "name": "Down", "url": format!("http://{dead}/hook") }))
            .await
            .json();
        let hook_pid = registered["pid"].as_str().unwrap().to_string();

        let report: Value = request.post("/api/webhooks/dispatch").await.json();
        assert_eq!(report["delivered"], 0);
        assert!(
            report["failed"].as_u64().unwrap() > 0,
            "a connection failure is retryable, not abandoned: {report}"
        );

        // The second attempt is 30s away, so an immediate rerun does
        // nothing rather than retrying in a tight loop.
        let again: Value = request.post("/api/webhooks/dispatch").await.json();
        assert_eq!(again["outcomes"].as_array().unwrap().len(), 0);

        let log: Value = request
            .get(&format!("/api/webhooks/{hook_pid}/deliveries"))
            .await
            .json();
        let rows = log["deliveries"].as_array().unwrap();
        assert!(rows.iter().all(|row| row["state"] == "failed"));
        assert!(rows.iter().all(|row| row["attempt"] == 1));
        // The counter advances once per *failure*, not once per pass:
        // otherwise a subscription broken for every event would take
        // twenty dispatches to deactivate instead of twenty failures.
        assert_eq!(
            log["webhook"]["consecutive_failures"].as_u64().unwrap(),
            rows.len() as u64
        );
        // The error is recorded so an operator can see why.
        assert!(rows.iter().all(|row| row["error"].is_string()));
    })
    .await;
}
