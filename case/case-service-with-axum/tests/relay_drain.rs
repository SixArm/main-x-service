//! The relay's drain and purge against a real outbox, with no broker:
//! a sink that fails leaves every row unpublished (and per-pid order
//! intact), a sink that succeeds stamps `published_at` on what it sent,
//! and the retention purge removes published rows. `fluvio_relay.rs`
//! covers a live broker behind the `fluvio` feature; this needs none.
//!
//! Its own binary: the transport is read once, and is pinned to `outbox`
//! here. `#[ignore]`d: boots the app against Postgres.

use axa_test::prelude::*;
use case_matcher::Case;
use case_service::app::App;
use case_service::models::event_outbox;
use case_service::relay::{EventSink, SinkError, drain_once, purge_published};
use case_service::streaming;
use serial_test::serial;
use std::sync::Mutex;

struct Capturing(Mutex<Vec<String>>);

#[async_trait::async_trait]
impl EventSink for Capturing {
    async fn send(
        &self,
        _entity: &str,
        key: &str,
        _payload: &serde_json::Value,
    ) -> Result<(), SinkError> {
        self.0.lock().unwrap().push(key.to_string());
        Ok(())
    }
}

struct Failing;

#[async_trait::async_trait]
impl EventSink for Failing {
    async fn send(
        &self,
        _entity: &str,
        _key: &str,
        _payload: &serde_json::Value,
    ) -> Result<(), SinkError> {
        Err("broker unavailable".into())
    }
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL; run with: cargo test --test relay_drain -- --ignored"]
async fn drain_publishes_on_success_holds_on_failure_and_purge_removes() {
    // SAFETY: single-threaded setup, before the app reads them.
    unsafe {
        std::env::set_var("CASE_EVENT_TRANSPORT", "outbox");
        std::env::set_var(
            "CASE_SEARCH_INDEX_PATH",
            std::env::temp_dir().join(format!("case-relay-test-index-{}", std::process::id())),
        );
    }
    request::<App, _, _>(|_request, ctx| async move {
        let model = streaming::create_and_emit(&ctx.db, &Case::new("Relay drain case"), None)
            .await
            .expect("create under the outbox transport");
        let pid = model.pid;
        let ours = || async { event_outbox::Model::for_entity(&ctx.db, pid).await.unwrap() };
        assert_eq!(ours().await.len(), 1, "one outbox row for the create");
        assert!(ours().await[0].published_at.is_none());

        let sent = drain_once(&ctx.db, &Failing, 1000)
            .await
            .expect("drain with a failing sink");
        assert_eq!(sent, 0, "nothing is published when the first send fails");
        assert!(
            ours().await[0].published_at.is_none(),
            "the row waits for the next tick"
        );

        let sink = Capturing(Mutex::new(Vec::new()));
        let sent = drain_once(&ctx.db, &sink, 1000).await.expect("drain");
        assert!(sent >= 1);
        assert!(
            sink.0.lock().unwrap().contains(&pid.to_string()),
            "keyed by pid"
        );
        assert!(
            ours().await[0].published_at.is_some(),
            "published_at is stamped"
        );
        assert_eq!(
            drain_once(&ctx.db, &sink, 1000).await.unwrap(),
            0,
            "a published row is not resent"
        );

        let purged = purge_published(&ctx.db, 0).await.expect("purge");
        assert!(purged >= 1);
        assert!(ours().await.is_empty(), "retention removes published rows");
    })
    .await;
}
