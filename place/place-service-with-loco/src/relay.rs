//! Durable event bus **Phase 3** — the outbox relay + retention.
//!
//! Phase 2 ([`crate::streaming`] + [`crate::db::outbox`]) durably enqueues
//! each event into the `event_outbox` table inside the entity mutation's
//! transaction. Phase 3 is the **relay**: a background loop that drains
//! unpublished outbox rows to an [`EventSink`], stamps `published_at`, and
//! periodically purges old published rows. Delivery is **at-least-once** (a
//! crash between the sink send and the `published_at` stamp re-sends), so
//! sinks/consumers dedupe on the envelope `event_id` (see
//! `agents/share/event-bus.md` §6).
//!
//! The default [`LoggingSink`] ships every event to the tracing log: it is
//! the **no-broker** dev/CI sink and a useful observability aid — the drain
//! + retention machinery runs and is fully exercised without Fluvio. A real
//! **`FluvioSink`** is simply another `impl EventSink` (behind a future
//! `fluvio` cargo feature, since it needs the broker + the `fluvio` crate);
//! the [`EventSink`] trait is the seam, so the drain loop and retention
//! never change when it lands (`agents/share/event-bus.md` §5).
//!
//! Activation: the relay only runs when the transport is `outbox`
//! (`PLACE_EVENT_TRANSPORT=outbox`) **and** `PLACE_EVENT_RELAY` is truthy —
//! so it is a no-op by default, matching the family's flag-gated posture.

use std::time::Duration;

use loco_rs::prelude::*;
use time::OffsetDateTime;

use crate::Result;
use crate::db::models::event_outbox;
use crate::db::outbox::Model as OutboxRow;

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
    ) -> std::result::Result<(), SinkError>;
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
    ) -> std::result::Result<(), SinkError> {
        tracing::info!(
            topic = format!("mxi.{entity}.events"),
            key,
            payload = %payload,
            "relay: published outbox event"
        );
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
) -> Result<usize> {
    // SEC-B6: claim rows under a transaction with FOR UPDATE SKIP LOCKED
    // (in `unpublished`) so parallel relay instances can't double-ship.
    let txn = sea_orm::TransactionTrait::begin(db)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;
    let rows = OutboxRow::unpublished(&txn, batch).await?;
    let mut published: Vec<i64> = Vec::with_capacity(rows.len());
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
    txn.commit()
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;
    Ok(published.len())
}

/// Delete published rows older than `retention_days` (the outbox is a
/// short-lived hand-off buffer; durable history is the bus's job — see
/// `agents/share/event-bus.md` §3). Returns the number of rows purged.
///
/// # Errors
///
/// When the delete query fails.
pub async fn purge_published(db: &DatabaseConnection, retention_days: i64) -> Result<u64> {
    let cutoff = OffsetDateTime::now_utc() - time::Duration::days(retention_days.max(0));
    let res = event_outbox::Entity::delete_many()
        .filter(event_outbox::Column::PublishedAt.is_not_null())
        .filter(event_outbox::Column::PublishedAt.lt(cutoff))
        .exec(db)
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;
    Ok(res.rows_affected)
}

/// Whether the relay background loop should run: `PLACE_EVENT_RELAY`
/// truthy (`1`/`true`/`yes`/`on`). Off by default.
#[must_use]
pub fn relay_enabled() -> bool {
    matches!(
        std::env::var("PLACE_EVENT_RELAY")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// Relay poll interval in seconds (`PLACE_EVENT_RELAY_INTERVAL_SECS`,
/// default 5, floored at 1).
#[must_use]
pub fn interval_secs() -> u64 {
    std::env::var("PLACE_EVENT_RELAY_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(5)
        .max(1)
}

/// Outbox retention in days (`PLACE_EVENT_RETENTION_DAYS`, default 7).
#[must_use]
pub fn retention_days() -> i64 {
    std::env::var("PLACE_EVENT_RETENTION_DAYS")
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(7)
}

/// How many drain ticks between retention purges (purge is cheaper and
/// less urgent than draining).
const PURGE_EVERY_TICKS: u64 = 60;

/// Spawn the background relay loop if (and only if) the transport is
/// `outbox` and [`relay_enabled`]. A no-op otherwise — so with the default
/// `memory` transport this never starts, keeping behaviour unchanged.
///
/// Uses the [`LoggingSink`]; swap in a `FluvioSink` (feature-gated) here
/// when the broker lands — the loop is sink-agnostic.
pub fn spawn(db: DatabaseConnection) {
    if !crate::streaming::transport().is_outbox() || !relay_enabled() {
        return;
    }
    let interval = interval_secs();
    let retention = retention_days();
    tracing::info!(
        interval_secs = interval,
        retention_days = retention,
        "starting event-outbox relay"
    );
    tokio::spawn(async move {
        let sink = LoggingSink;
        let mut ticks: u64 = 0;
        loop {
            if let Err(err) = drain_once(&db, &sink, 100).await {
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
        ) -> std::result::Result<(), SinkError> {
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
                .send("place", "pid-1", &serde_json::json!({"kind": "created"}))
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
            sink.send("place", "pid-9", &serde_json::json!({}))
                .await
                .unwrap();
        });
        assert_eq!(
            sink.0.lock().unwrap().as_slice(),
            &[("place".to_string(), "pid-9".to_string())]
        );
    }

    /// Config parsers: relay off by default; interval floors at 1; retention
    /// defaults to 7 (env-independent defaults, since env may be unset).
    #[test]
    fn config_defaults_are_safe() {
        // These read process env; with the vars unset (the CI default) the
        // documented defaults apply.
        if std::env::var("PLACE_EVENT_RELAY").is_err() {
            assert!(!relay_enabled());
        }
        if std::env::var("PLACE_EVENT_RELAY_INTERVAL_SECS").is_err() {
            assert_eq!(interval_secs(), 5);
        }
        if std::env::var("PLACE_EVENT_RETENTION_DAYS").is_err() {
            assert_eq!(retention_days(), 7);
        }
    }
}
