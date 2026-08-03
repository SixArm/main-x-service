//! Durable event bus **Phase 3** — the outbox relay + retention.
//!
//! Phase 2 ([`crate::streaming`]) durably enqueues each event into the
//! `event_outbox` table inside the entity mutation's transaction. Phase 3
//! is the **relay**: a background loop that drains unpublished outbox rows
//! to an [`EventSink`], stamps `published_at`, and periodically purges old
//! published rows. Delivery is **at-least-once** (a crash between the sink
//! send and the `published_at` stamp re-sends), so sinks/consumers dedupe
//! on the envelope `event_id` (see `agents/share/event-bus.md` §6).
//!
//! The default [`LoggingSink`] ships every event to the tracing log: it is
//! the **no-broker** dev/CI sink and a useful observability aid — the drain
//! + retention machinery runs and is fully exercised without Fluvio.
//! `FluvioSink` (BUS-1; only compiled under this crate's `fluvio` cargo
//! feature, off by default, so a default build's dependency tree and
//! behaviour are unchanged) is the real-broker `impl EventSink`; the
//! [`EventSink`] trait is the seam, so the drain loop and retention never
//! change between the two (`agents/share/event-bus.md` §5).
//!
//! Activation: the relay only runs when the transport is `outbox`
//! (`CASE_EVENT_TRANSPORT=outbox`) **and** `CASE_EVENT_RELAY` is truthy —
//! so it is a no-op by default, matching the family's flag-gated posture.
//! With those on, [`spawn`] additionally selects `FluvioSink` over
//! [`LoggingSink`] when `CASE_FLUVIO_ENDPOINT` (§7) is configured — see
//! `spawn`'s and `build_sink`'s doc comments in this file for the "no
//! silent fallback" rule that governs an endpoint configured without the
//! `fluvio` feature.

use std::time::Duration;

use loco_rs::prelude::*;
use sea_orm::TransactionTrait;

use crate::models::_entities::event_outbox::Column as OutboxColumn;
use crate::models::event_outbox::{self, Model as OutboxRow};

/// A boxed, thread-safe error from a sink send (a bus/broker failure).
pub type SinkError = Box<dyn std::error::Error + Send + Sync>;

/// A durable-bus sink: ships one outbox event to the stream. The relay
/// calls [`EventSink::send`] once per unpublished row; a returned `Err`
/// leaves the row unpublished (it retries next tick). Delivery is
/// at-least-once, so real sinks/consumers dedupe on the envelope
/// `event_id` carried in `payload`.
#[async_trait::async_trait]
pub trait EventSink: Send + Sync {
    /// Ship one event: `entity` selects the topic (`mxi.<entity>.events`),
    /// `key` is the partition key (the record `pid`), `payload` is the full
    /// canonical envelope.
    ///
    /// # Errors
    ///
    /// When the underlying bus/broker send fails.
    async fn send(
        &self,
        entity: &str,
        key: &str,
        payload: &serde_json::Value,
    ) -> Result<(), SinkError>;
}

/// The default **no-broker** sink: log each event at `INFO`. Used in dev /
/// CI and as an observability tap; the relay drains + acks exactly as it
/// would against a real broker. Never fails.
pub struct LoggingSink;

#[async_trait::async_trait]
impl EventSink for LoggingSink {
    async fn send(
        &self,
        entity: &str,
        key: &str,
        payload: &serde_json::Value,
    ) -> Result<(), SinkError> {
        tracing::info!(
            topic = format!("mxi.{entity}.events"),
            key,
            payload = %payload,
            "relay: published outbox event"
        );
        Ok(())
    }
}

/// The real-broker sink (BUS-1; behind the `fluvio` cargo feature): ships
/// each event to a [Fluvio](https://www.fluvio.io/) topic, partitioned by
/// record `pid` per `agents/share/event-bus.md` §7. One producer per
/// topic, held for the sink's lifetime — `TopicProducerPool` is designed
/// to be built once and reused across sends rather than reconnected per
/// event.
#[cfg(feature = "fluvio")]
pub struct FluvioSink {
    /// The connected producer for this sink's topic.
    producer: fluvio::TopicProducerPool,
}

#[cfg(feature = "fluvio")]
impl FluvioSink {
    /// Connect to the Fluvio cluster at `endpoint` and open a producer for
    /// `topic`.
    ///
    /// # Errors
    ///
    /// When the cluster connection or the producer handshake fails.
    pub async fn connect(endpoint: &str, topic: &str) -> Result<Self, SinkError> {
        let config = fluvio::FluvioConfig::new(endpoint);
        let client = fluvio::Fluvio::connect_with_config(&config).await?;
        let producer = client.topic_producer(topic).await?;
        Ok(Self { producer })
    }
}

#[cfg(feature = "fluvio")]
#[async_trait::async_trait]
impl EventSink for FluvioSink {
    async fn send(
        &self,
        _entity: &str,
        key: &str,
        payload: &serde_json::Value,
    ) -> Result<(), SinkError> {
        // `_entity` selects the *topic*, which this sink is already
        // connected to (one FluvioSink per topic, per `spawn` below); the
        // partition key is the record `pid`, per §7.
        let bytes = serde_json::to_vec(payload)?;
        self.producer.send(key.to_string(), bytes).await?;
        Ok(())
    }
}

/// Drain up to `batch` unpublished outbox rows to `sink`, stamping
/// `published_at` on each successful send. Stops at the first send failure
/// (the row stays unpublished and is retried next tick, preserving per-pid
/// order). Returns the number of rows published this pass.
///
/// # Errors
///
/// When the outbox poll or the `mark_published` ack query fails.
pub async fn drain_once<S: EventSink + ?Sized>(
    db: &DatabaseConnection,
    sink: &S,
    batch: u64,
) -> ModelResult<usize> {
    // SEC-B6: run the whole drain in one transaction and claim the rows with
    // `FOR UPDATE SKIP LOCKED` (in `unpublished`), so two parallel relay
    // instances never grab the same rows and double-ship. The lock is held
    // across the send window; on commit the published rows are acked and any
    // row left unpublished (a send that failed, or was never reached) is
    // released and retried next tick. Delivery stays at-least-once (consumers
    // dedupe on `event_id`); the lock removes the duplicate-per-instance case.
    let txn = db.begin().await?;
    let rows = OutboxRow::unpublished(&txn, batch).await?;
    let mut published: Vec<i32> = Vec::with_capacity(rows.len());
    for row in &rows {
        match sink
            .send(&row.entity, &row.entity_pid.to_string(), &row.payload)
            .await
        {
            Ok(()) => published.push(row.id),
            Err(err) => {
                // Leave this row (and everything after it, to keep per-pid
                // order) unpublished; retry on the next tick.
                tracing::warn!(id = row.id, error = %err, "relay send failed; will retry");
                break;
            }
        }
    }
    if !published.is_empty() {
        OutboxRow::mark_published(&txn, &published).await?;
    }
    txn.commit().await?;
    Ok(published.len())
}

/// Delete published rows older than `retention_days` (the outbox is a
/// short-lived hand-off buffer; durable history is the bus's job — see
/// `agents/share/event-bus.md` §3). Returns the number of rows purged.
///
/// # Errors
///
/// When the delete query fails.
pub async fn purge_published(db: &DatabaseConnection, retention_days: i64) -> ModelResult<u64> {
    let cutoff =
        chrono::Utc::now() - chrono::Duration::try_days(retention_days.max(0)).unwrap_or_default();
    let res = event_outbox::Entity::delete_many()
        .filter(OutboxColumn::PublishedAt.is_not_null())
        .filter(OutboxColumn::PublishedAt.lt(cutoff.fixed_offset()))
        .exec(db)
        .await?;
    Ok(res.rows_affected)
}

/// Whether the relay background loop should run: `CASE_EVENT_RELAY`
/// truthy (`1`/`true`/`yes`/`on`). Off by default.
#[must_use]
pub fn relay_enabled() -> bool {
    matches!(
        std::env::var("CASE_EVENT_RELAY")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// Relay poll interval in seconds (`CASE_EVENT_RELAY_INTERVAL_SECS`,
/// default 5, floored at 1).
#[must_use]
pub fn interval_secs() -> u64 {
    std::env::var("CASE_EVENT_RELAY_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(5)
        .max(1)
}

/// Outbox retention in days (`CASE_EVENT_RETENTION_DAYS`, default 7).
#[must_use]
pub fn retention_days() -> i64 {
    std::env::var("CASE_EVENT_RETENTION_DAYS")
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(7)
}

/// Fluvio broker endpoint (`CASE_FLUVIO_ENDPOINT`, §7). Unset/blank ⇒ no
/// broker configured, so the relay uses [`LoggingSink`] — the documented
/// no-broker dev/CI sink — even under the `outbox` transport.
#[must_use]
pub fn fluvio_endpoint() -> Option<String> {
    std::env::var("CASE_FLUVIO_ENDPOINT")
        .ok()
        .filter(|s| !s.trim().is_empty())
}

/// The topic events publish to (`CASE_EVENT_TOPIC`, default
/// `mxi.case.events` per §7's `mxi.<entity>.events` convention).
#[must_use]
pub fn event_topic() -> String {
    std::env::var("CASE_EVENT_TOPIC")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "mxi.case.events".to_string())
}

/// How many drain ticks between retention purges (purge is cheaper and
/// less urgent than draining).
const PURGE_EVERY_TICKS: u64 = 60;

/// Seconds between retries while the initial Fluvio connection is down.
#[cfg(feature = "fluvio")]
const RECONNECT_BACKOFF_SECS: u64 = 5;

/// Build the relay's sink for this run.
///
/// [`FluvioSink`] when an endpoint is configured — [`spawn`] has already
/// refused to start if an endpoint was set without this feature compiled
/// in, so reaching a `None` endpoint here always means [`LoggingSink`] is
/// the intended sink. The initial connection is **retried indefinitely**
/// rather than falling back to [`LoggingSink`]: a fallback here would
/// silently start stamping outbox rows `published_at` without ever
/// reaching the broker the operator explicitly asked for — the same
/// silent-data-loss shape the family's artifact-store "no fallback on an
/// explicit backend choice" rule exists to prevent
/// (`agents/share/bulk-import-export.md` §12).
#[cfg(feature = "fluvio")]
async fn build_sink(endpoint: Option<&str>) -> Box<dyn EventSink> {
    let Some(endpoint) = endpoint else {
        return Box::new(LoggingSink);
    };
    let topic = event_topic();
    loop {
        match FluvioSink::connect(endpoint, &topic).await {
            Ok(sink) => {
                tracing::info!(endpoint, topic, "relay: connected FluvioSink");
                return Box::new(sink);
            }
            Err(err) => {
                tracing::warn!(endpoint, error = %err, "relay: FluvioSink connect failed; retrying");
                tokio::time::sleep(Duration::from_secs(RECONNECT_BACKOFF_SECS)).await;
            }
        }
    }
}

/// Without the `fluvio` feature, [`spawn`] has already refused to start
/// when an endpoint is configured, so the only reachable case is
/// [`LoggingSink`]. Stays `async` (with nothing to await) so both cfg
/// variants share one call site in [`spawn`].
#[cfg(not(feature = "fluvio"))]
#[allow(clippy::unused_async)]
async fn build_sink(_endpoint: Option<&str>) -> Box<dyn EventSink> {
    Box::new(LoggingSink)
}

/// The drain + retention loop, generic over the already-connected sink.
async fn run_drain_loop(
    db: DatabaseConnection,
    sink: Box<dyn EventSink>,
    interval: u64,
    retention: i64,
) {
    let mut ticks: u64 = 0;
    loop {
        if let Err(err) = drain_once(&db, sink.as_ref(), 100).await {
            tracing::warn!(error = %err, "relay drain pass failed");
        }
        ticks = ticks.wrapping_add(1);
        if ticks.is_multiple_of(PURGE_EVERY_TICKS) {
            match purge_published(&db, retention).await {
                Ok(n) if n > 0 => {
                    tracing::info!(purged = n, "relay purged old published outbox rows");
                }
                Ok(_) => {}
                Err(err) => tracing::warn!(error = %err, "relay retention purge failed"),
            }
        }
        tokio::time::sleep(Duration::from_secs(interval)).await;
    }
}

/// Spawn the background relay loop if (and only if) the transport is
/// `outbox` and [`relay_enabled`]. A no-op otherwise — so with the default
/// `memory` transport this never starts, keeping behaviour unchanged.
///
/// Sink selection: no `CASE_FLUVIO_ENDPOINT` ⇒ [`LoggingSink`]. An
/// endpoint configured **without** the `fluvio` cargo feature is a clean
/// refusal to start the relay at all (logged at `error`) rather than a
/// silent `LoggingSink` fallback — see `build_sink`'s doc comment in this
/// file for why a fallback here would be worse than not running.
pub fn spawn(db: DatabaseConnection) {
    if !crate::streaming::transport().is_outbox() || !relay_enabled() {
        return;
    }
    let endpoint = fluvio_endpoint();
    if endpoint.is_some() && !cfg!(feature = "fluvio") {
        tracing::error!(
            "CASE_FLUVIO_ENDPOINT is set but this binary was built without the `fluvio` \
             cargo feature; the relay will NOT start — rebuild with `--features fluvio` \
             (falling back to the logging sink would mark outbox rows published without \
             ever reaching a real broker)"
        );
        return;
    }
    let interval = interval_secs();
    let retention = retention_days();
    tracing::info!(
        interval_secs = interval,
        retention_days = retention,
        fluvio_endpoint = endpoint.as_deref().unwrap_or("(none — LoggingSink)"),
        "starting event-outbox relay"
    );
    tokio::spawn(async move {
        let sink = build_sink(endpoint.as_deref()).await;
        run_drain_loop(db, sink, interval, retention).await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// A capturing sink for tests: records every `(entity, key)` sent.
    struct CapturingSink(Mutex<Vec<(String, String)>>);

    #[async_trait::async_trait]
    impl EventSink for CapturingSink {
        async fn send(
            &self,
            entity: &str,
            key: &str,
            _payload: &serde_json::Value,
        ) -> Result<(), SinkError> {
            self.0
                .lock()
                .unwrap()
                .push((entity.to_string(), key.to_string()));
            Ok(())
        }
    }

    /// The logging sink never fails (async smoke over a tiny runtime).
    #[test]
    fn logging_sink_sends_ok() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let out = rt.block_on(async {
            LoggingSink
                .send("case", "pid-1", &serde_json::json!({"kind": "created"}))
                .await
        });
        assert!(out.is_ok());
    }

    /// A capturing sink records what it is handed (pins the send contract
    /// the DB-gated drain test relies on).
    #[test]
    fn capturing_sink_records_entity_and_key() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let sink = CapturingSink(Mutex::new(Vec::new()));
        rt.block_on(async {
            sink.send("case", "pid-9", &serde_json::json!({}))
                .await
                .unwrap();
        });
        assert_eq!(
            sink.0.lock().unwrap().as_slice(),
            &[("case".to_string(), "pid-9".to_string())]
        );
    }

    /// Config parsers: relay off by default; interval floors at 1; retention
    /// defaults to 7 (env-independent defaults, since env may be unset).
    #[test]
    fn config_defaults_are_safe() {
        // These read process env; with the vars unset (the CI default) the
        // documented defaults apply.
        if std::env::var("CASE_EVENT_RELAY").is_err() {
            assert!(!relay_enabled());
        }
        if std::env::var("CASE_EVENT_RELAY_INTERVAL_SECS").is_err() {
            assert_eq!(interval_secs(), 5);
        }
        if std::env::var("CASE_EVENT_RETENTION_DAYS").is_err() {
            assert_eq!(retention_days(), 7);
        }
    }
}
