//! Minimal in-memory event stream.
//!
//! Every CRUD action publishes a typed [`CaseEvent`] to a process-wide
//! ring buffer. This is the MVP of the family's event-streaming layer
//! (siblings swap in Kafka/NATS/Fluvio behind the same publish call).
//! In loco there is no per-request shared state for this, so the buffer
//! is a `OnceLock`-initialised global.

use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

/// The kind of change that occurred.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EventKind {
    /// A record was created.
    Created,
    /// A record was updated.
    Updated,
    /// A record was soft-deleted.
    Deleted,
    /// A duplicate was merged into this (surviving) record.
    Merged,
}

/// A published case event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseEvent {
    /// The kind of change.
    pub kind: EventKind,
    /// The case's public id.
    pub pid: String,
    /// The case's title at the time of the event.
    pub title: String,
    /// Monotonic sequence number (per process).
    pub seq: u64,
}

const CAPACITY: usize = 1000;

fn buffer() -> &'static Mutex<VecDeque<CaseEvent>> {
    static BUF: OnceLock<Mutex<VecDeque<CaseEvent>>> = OnceLock::new();
    BUF.get_or_init(|| Mutex::new(VecDeque::with_capacity(CAPACITY)))
}

fn next_seq() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(1);
    SEQ.fetch_add(1, Ordering::Relaxed)
}

/// Publish an event to the in-memory stream. Never fails; if the lock is
/// poisoned the event is dropped (the audit log is the durable record).
pub fn publish(kind: EventKind, pid: &str, title: &str) {
    let event = CaseEvent {
        kind,
        pid: pid.to_string(),
        title: title.to_string(),
        seq: next_seq(),
    };
    if let Ok(mut buf) = buffer().lock() {
        if buf.len() == CAPACITY {
            buf.pop_front();
        }
        buf.push_back(event);
    }
}

/// The most recent events (newest last), capped at `limit`.
#[must_use]
pub fn recent(limit: usize) -> Vec<CaseEvent> {
    buffer().lock().map_or_else(
        |_| Vec::new(),
        |buf| buf.iter().rev().take(limit).rev().cloned().collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_and_read_back() {
        publish(EventKind::Created, "pid-1", "Housing benefit appeal");
        publish(
            EventKind::Updated,
            "pid-1",
            "Housing benefit appeal (rev 2)",
        );
        let events = recent(10);
        assert!(events.len() >= 2);
        let last = events.last().unwrap();
        assert_eq!(last.kind, EventKind::Updated);
        assert_eq!(last.title, "Housing benefit appeal (rev 2)");
        // Sequence numbers are monotonic.
        assert!(events.windows(2).all(|w| w[0].seq < w[1].seq));
    }
}
