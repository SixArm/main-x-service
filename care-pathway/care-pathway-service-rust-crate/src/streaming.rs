//! Minimal in-memory event stream.
//!
//! Every CRUD action publishes a typed [`PathwayEvent`] to a process-wide
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
}

/// A published care-pathway event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathwayEvent {
    /// The kind of change.
    pub kind: EventKind,
    /// The care pathway's public id.
    pub pid: String,
    /// The care pathway's name at the time of the event.
    pub name: String,
    /// Monotonic sequence number (per process).
    pub seq: u64,
}

const CAPACITY: usize = 1000;

fn buffer() -> &'static Mutex<VecDeque<PathwayEvent>> {
    static BUF: OnceLock<Mutex<VecDeque<PathwayEvent>>> = OnceLock::new();
    BUF.get_or_init(|| Mutex::new(VecDeque::with_capacity(CAPACITY)))
}

fn next_seq() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(1);
    SEQ.fetch_add(1, Ordering::Relaxed)
}

/// Publish an event to the in-memory stream. Never fails; if the lock is
/// poisoned the event is dropped (the audit log is the durable record).
pub fn publish(kind: EventKind, pid: &str, name: &str) {
    let event = PathwayEvent {
        kind,
        pid: pid.to_string(),
        name: name.to_string(),
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
pub fn recent(limit: usize) -> Vec<PathwayEvent> {
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
        publish(EventKind::Created, "pid-1", "Acute Stroke Care Pathway");
        publish(EventKind::Updated, "pid-1", "Acute Stroke Pathway");
        let events = recent(10);
        assert!(events.len() >= 2);
        let last = events.last().unwrap();
        assert_eq!(last.kind, EventKind::Updated);
        assert_eq!(last.name, "Acute Stroke Pathway");
        // Sequence numbers are monotonic.
        assert!(events.windows(2).all(|w| w[0].seq < w[1].seq));
    }
}
