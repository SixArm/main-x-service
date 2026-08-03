//! Durable event bus — the real Fluvio consumer (BUS-2; spec §13 T-6,
//! `agents/share/event-bus.md` §5/§8/§9).
//!
//! One consumer task per entity topic (`mxi.<entity>.events`,
//! [`entity_ref::EntityType::ALL`]), each folding every consumed
//! envelope into the read-model via
//! [`events::apply_event_idempotent`](crate::events::apply_event_idempotent).
//! Behind this crate's own `fluvio` Cargo feature (off by default) — a
//! default build has no consumer at all, and lazy verify-on-read
//! (`crate::probe`, spec T-10) plus periodic reconciliation
//! (`crate::reconcile`, design §8) remain the read-model's integrity
//! path with no broker configured, exactly as before this task.
//!
//! ## Resume position: delegated to Fluvio, not `consumer_offsets`
//!
//! Each consumer connects with a stable, per-topic **named** consumer
//! (`offset_consumer`) and `OffsetManagementStrategy::Auto`, so Fluvio's
//! own SC persists and resumes that consumer's position server-side
//! across restarts — the idiomatic mechanism for exactly what spec T-6
//! asks ("per-topic offset resume"), rather than this crate
//! reimplementing it against `consumer_offsets.offset_val`.
//! `offset_start(Offset::beginning())` is only the fallback for a
//! brand-new named consumer that has never committed a position; a
//! consumer with prior history resumes from there regardless.
//!
//! This leaves `consumer_offsets.offset_val` exactly what
//! [`events::apply_event`](crate::events::apply_event) has always
//! written to it: the envelope's own per-`entity_pid` `seq` — a
//! diagnostic/freshness value (spec §10.3's "last committed bus offset"
//! wording predates a real consumer and is read here as "last committed
//! **event**", not a literal Fluvio partition byte offset). Changing
//! `apply_event`'s signature to thread a real Fluvio-record offset
//! through it would touch roughly two dozen existing test call sites
//! for no behavioural gain, since resume does not depend on it.
//!
//! ## Idempotency is independent of the resume mechanism
//!
//! Delivery is at-least-once regardless of how offsets are tracked
//! (event-bus.md §6), so `processed_events` dedup
//! ([`events::apply_event_idempotent`](crate::events::apply_event_idempotent))
//! is required either way — it is not a stand-in for offset tracking,
//! and offset tracking is not a stand-in for it.

#[cfg(feature = "fluvio")]
use std::time::Duration;

#[cfg(feature = "fluvio")]
use entity_ref::EntityType;
use sea_orm::DatabaseConnection;

#[cfg(feature = "fluvio")]
use crate::events;

/// Fluvio broker endpoint (`LINK_GRAPH_FLUVIO_ENDPOINT`, event-bus.md
/// §7). Unset/blank ⇒ no bus consumption at all — lazy verify-on-read
/// and reconciliation are the read-model's only integrity path, exactly
/// as before this task.
#[must_use]
pub fn fluvio_endpoint() -> Option<String> {
    std::env::var("LINK_GRAPH_FLUVIO_ENDPOINT")
        .ok()
        .filter(|s| !s.trim().is_empty())
}

/// `processed_events` retention in days
/// (`LINK_GRAPH_PROCESSED_EVENTS_RETENTION_DAYS`, default 7) — mirrors
/// the family's outbox-row retention convention (event-bus.md §3).
#[must_use]
pub fn retention_days() -> i64 {
    std::env::var("LINK_GRAPH_PROCESSED_EVENTS_RETENTION_DAYS")
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(7)
}

/// How often the retention purge runs, in seconds
/// (`LINK_GRAPH_PROCESSED_EVENTS_PURGE_INTERVAL_SECS`, default 3600).
#[must_use]
pub fn purge_interval_secs() -> u64 {
    std::env::var("LINK_GRAPH_PROCESSED_EVENTS_PURGE_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(3600)
        .max(1)
}

/// Spawn one consumer task per entity topic, plus the retention purge
/// loop, if (and only if) `LINK_GRAPH_FLUVIO_ENDPOINT` is configured. A
/// no-op otherwise — so with no endpoint set, behaviour is unchanged.
///
/// An endpoint configured **without** the `fluvio` cargo feature is a
/// clean refusal (logged at `error`) rather than silently starting no
/// consumer at all with no warning — the read-model would then fall
/// behind with nothing telling the operator why, the same
/// no-silent-fallback shape as BUS-1's relay sink selection
/// (`case/case-service-with-loco/src/relay.rs`).
///
/// `db` is only actually consumed (moved into the per-topic tasks) under
/// the `fluvio` feature; the lints below are scoped to a build without
/// it, where the function's tail is the early-return branches above.
#[cfg_attr(
    not(feature = "fluvio"),
    allow(
        unused_variables,
        clippy::needless_pass_by_value,
        clippy::needless_return
    )
)]
pub fn spawn(db: DatabaseConnection) {
    let Some(endpoint) = fluvio_endpoint() else {
        return;
    };
    if !cfg!(feature = "fluvio") {
        tracing::error!(
            "LINK_GRAPH_FLUVIO_ENDPOINT is set but this binary was built without the \
             `fluvio` cargo feature; no bus consumer will start — rebuild with \
             `--features fluvio` (the read-model would otherwise silently fall behind \
             with no live consumption and no warning)"
        );
        return;
    }
    #[cfg(feature = "fluvio")]
    {
        tracing::info!(endpoint, "starting per-topic bus consumers");
        for entity in EntityType::ALL {
            let db = db.clone();
            let endpoint = endpoint.clone();
            tokio::spawn(async move {
                run_topic_consumer(db, endpoint, entity).await;
            });
        }
        tokio::spawn(purge_loop(db));
    }
}

/// Periodically purge `processed_events` rows older than
/// [`retention_days`] — a short-lived dedup window, not durable history
/// (event-bus.md §3).
#[cfg(feature = "fluvio")]
async fn purge_loop(db: DatabaseConnection) {
    let interval = purge_interval_secs();
    let retention = retention_days();
    loop {
        match crate::models::processed_events::Model::purge_older_than(&db, retention).await {
            Ok(n) if n > 0 => tracing::info!(purged = n, "purged old processed_events rows"),
            Ok(_) => {}
            Err(err) => tracing::warn!(error = %err, "processed_events retention purge failed"),
        }
        tokio::time::sleep(Duration::from_secs(interval)).await;
    }
}

/// Seconds between reconnect attempts after the consumer stream for a
/// topic ends or errors.
#[cfg(feature = "fluvio")]
const RECONNECT_BACKOFF_SECS: u64 = 5;

/// Consume `mxi.<entity>.events` forever, folding every record into the
/// read-model via [`events::apply_event_idempotent`]. Never returns
/// under normal operation; a connection or stream error is logged and
/// retried after a backoff rather than propagated — this runs detached
/// via [`tokio::spawn`], so there is no caller to propagate an error to.
#[cfg(feature = "fluvio")]
async fn run_topic_consumer(db: DatabaseConnection, endpoint: String, entity: EntityType) {
    let topic = events::topic_for(entity.as_str());
    loop {
        if let Err(err) = consume_once(&db, &endpoint, &topic).await {
            tracing::warn!(topic, error = %err, "bus consumer stream ended; reconnecting");
        }
        tokio::time::sleep(Duration::from_secs(RECONNECT_BACKOFF_SECS)).await;
    }
}

/// Connect, open a consumer stream for `topic`, and consume records
/// until the stream ends or errors. Returns on any stream-level error so
/// the caller reconnects; a per-record decode/apply error is logged and
/// skipped instead — a single malformed envelope must not wedge the
/// whole topic.
#[cfg(feature = "fluvio")]
async fn consume_once(
    db: &DatabaseConnection,
    endpoint: &str,
    topic: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use fluvio::consumer::{ConsumerConfigExtBuilder, OffsetManagementStrategy};
    use fluvio::{Fluvio, FluvioConfig, Offset};
    use futures_util::StreamExt;

    let config = FluvioConfig::new(endpoint);
    let client = Fluvio::connect_with_config(&config).await?;
    let mut stream = client
        .consumer_with_config(
            ConsumerConfigExtBuilder::default()
                .topic(topic.to_string())
                .offset_consumer(format!("link-graph-{topic}"))
                .offset_start(Offset::beginning())
                .offset_strategy(OffsetManagementStrategy::Auto)
                .build()?,
        )
        .await?;

    tracing::info!(topic, "bus consumer connected");
    while let Some(record) = stream.next().await {
        let record = record?;
        match serde_json::from_slice::<events::Envelope>(record.as_ref()) {
            Ok(envelope) => {
                if let Err(err) = events::apply_event_idempotent(db, envelope).await {
                    tracing::warn!(topic, error = %err, "apply_event_idempotent failed; event skipped");
                }
            }
            Err(err) => {
                tracing::warn!(topic, error = %err, "malformed envelope on topic; skipped");
            }
        }
    }
    Ok(())
}
