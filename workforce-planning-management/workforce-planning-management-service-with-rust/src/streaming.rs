//! Event streaming — the canonical versioned [`Envelope`] behind the
//! transport seam (`agents/share/event-bus.md`).
//!
//! Every mutation emits one envelope. The transport is selected by
//! `HCM_EVENT_TRANSPORT` (default `memory`):
//!
//! - `memory` — an in-process ring buffer ([`InMemoryPublisher`]),
//!   served by `GET /api/events/recent`. Lost on restart; Phase 1.
//! - `outbox` — a row in `event_outbox` written **on the caller's
//!   transaction** ([`crate::models::event_outbox::OutboxInsert`]), so
//!   no committed change lacks its event (Phase 2). A Phase-3 relay to
//!   Fluvio is roadmap, family-wide.
//!
//! Unlike the single-entity registries, HCM emits for several record
//! kinds, so `entity` and `kind` are `String`s: `employee` /
//! `requisition` / `application` / `leave_request` / `payroll_run` /
//! … × `created` / `updated` / `deleted` / `employee_hired` /
//! `employee_activated` / `employee_terminated` / `leave_approved` /
//! `payroll_run_calculated` / `review_shared` / … (the full kind list
//! is spec `audit.md`).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use sea_orm::ConnectionTrait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::event_outbox::OutboxInsert;

/// Version stamp carried by every envelope.
pub const SCHEMA_VERSION: u32 = 1;

/// The canonical event envelope (`event-bus.md` §4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    /// Consumer dedup key.
    pub event_id: Uuid,
    /// Envelope schema version ([`SCHEMA_VERSION`]).
    pub schema_version: u32,
    /// The record kind: `employee`, `requisition`, `payroll_run`, ….
    pub entity: String,
    /// The change kind: `created`, `bed_state_changed`, ….
    pub kind: String,
    /// The record's public id.
    pub pid: String,
    /// Process-local monotonic sequence.
    pub seq: u64,
    /// The acting user's pid, when a verified token was presented.
    pub actor: Option<String>,
    /// Human-oriented label (employee number, requisition title — already
    /// subject to masking rules at the read surface).
    pub name: String,
    /// Kind-specific detail (old/new state, edge fields, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// The flat operator projection served by `/api/events/recent`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventView {
    /// The change kind token.
    pub kind: String,
    /// The record pid.
    pub pid: String,
    /// The envelope's display label.
    pub name: String,
    /// The envelope sequence.
    pub seq: u64,
}

impl From<&Envelope> for EventView {
    fn from(env: &Envelope) -> Self {
        Self {
            kind: env.kind.clone(),
            pid: env.pid.clone(),
            name: env.name.clone(),
            seq: env.seq,
        }
    }
}

/// Ring-buffer capacity of the in-memory publisher.
const MEMORY_CAPACITY: usize = 1024;

/// The in-process ring buffer (Phase 1 transport).
struct InMemoryPublisher {
    events: Mutex<Vec<Envelope>>,
}

/// Process-wide publisher + sequence counter.
static PUBLISHER: OnceLock<InMemoryPublisher> = OnceLock::new();
static SEQ: AtomicU64 = AtomicU64::new(0);

fn publisher() -> &'static InMemoryPublisher {
    PUBLISHER.get_or_init(|| InMemoryPublisher {
        events: Mutex::new(Vec::new()),
    })
}

/// Next process-local sequence number.
fn next_seq() -> u64 {
    SEQ.fetch_add(1, Ordering::Relaxed)
}

/// Build an envelope (fresh `event_id` + next `seq`).
#[must_use]
pub fn envelope(
    entity: &str,
    kind: &str,
    pid: &str,
    name: &str,
    actor: Option<&str>,
    data: Option<serde_json::Value>,
) -> Envelope {
    Envelope {
        event_id: Uuid::new_v4(),
        schema_version: SCHEMA_VERSION,
        entity: entity.to_string(),
        kind: kind.to_string(),
        pid: pid.to_string(),
        seq: next_seq(),
        actor: actor.map(ToString::to_string),
        name: name.to_string(),
        data,
    }
}

/// The selected event transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventTransport {
    /// In-process ring buffer (default).
    Memory,
    /// Transactional `event_outbox` rows.
    Outbox,
}

impl EventTransport {
    /// Parse a transport token (case-insensitive; unknown ⇒ `Memory`).
    #[must_use]
    pub fn parse(s: &str) -> Self {
        if s.trim().eq_ignore_ascii_case("outbox") {
            Self::Outbox
        } else {
            Self::Memory
        }
    }

    /// Whether this transport writes outbox rows.
    #[must_use]
    pub const fn is_outbox(self) -> bool {
        matches!(self, Self::Outbox)
    }
}

/// The process-wide transport, read once from
/// `HCM_EVENT_TRANSPORT` (default `memory`).
#[must_use]
pub fn transport() -> EventTransport {
    static TRANSPORT: OnceLock<EventTransport> = OnceLock::new();
    *TRANSPORT.get_or_init(|| {
        EventTransport::parse(&std::env::var("HCM_EVENT_TRANSPORT").unwrap_or_default())
    })
}

/// Emit one event **on the given connection** under the active
/// transport: `outbox` inserts the `event_outbox` row on `conn` (pass
/// the mutation's transaction — one commit boundary); `memory` pushes
/// to the in-process ring buffer.
///
/// # Errors
///
/// When the outbox insert fails (the `memory` path is infallible).
pub async fn emit_on<C: ConnectionTrait>(
    conn: &C,
    entity: &str,
    kind: &str,
    pid: &str,
    name: &str,
    actor: Option<&str>,
    data: Option<serde_json::Value>,
) -> loco_rs::Result<()> {
    let env = envelope(entity, kind, pid, name, actor, data);
    if transport().is_outbox() {
        OutboxInsert::from_envelope(&env, chrono::Utc::now().into())
            .map_err(loco_rs::Error::Model)?
            .insert_on(conn)
            .await
            .map_err(loco_rs::Error::Model)?;
    } else {
        let mut events = publisher().events.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if events.len() >= MEMORY_CAPACITY {
            events.remove(0);
        }
        events.push(env);
    }
    Ok(())
}

/// The most recent in-memory events, newest first, capped at `limit`.
/// (Under the `outbox` transport, `/events/recent` reads the outbox
/// table instead — see the controller.)
#[must_use]
pub fn recent(limit: usize) -> Vec<EventView> {
    let events = publisher().events.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    events.iter().rev().take(limit).map(EventView::from).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unknown/blank transport tokens default to `memory`; `outbox`
    /// (any case) selects the outbox.
    #[test]
    fn transport_parse_defaults_to_memory() {
        assert_eq!(EventTransport::parse(""), EventTransport::Memory);
        assert_eq!(EventTransport::parse("junk"), EventTransport::Memory);
        assert_eq!(EventTransport::parse("memory"), EventTransport::Memory);
        assert_eq!(EventTransport::parse("outbox"), EventTransport::Outbox);
        assert_eq!(EventTransport::parse(" OUTBOX "), EventTransport::Outbox);
        assert!(EventTransport::Outbox.is_outbox());
        assert!(!EventTransport::Memory.is_outbox());
    }

    /// Envelopes carry the schema version, a fresh event id, and a
    /// monotonic sequence; the view projection is faithful.
    #[test]
    fn envelope_and_view_shape() {
        let a = envelope("employee", "employee_hired", "pid-1", "E-1001", Some("u1"), None);
        let b = envelope("employee", "employee_hired", "pid-1", "E-1001", None, None);
        assert_eq!(a.schema_version, SCHEMA_VERSION);
        assert_ne!(a.event_id, b.event_id);
        assert!(b.seq > a.seq);
        let view = EventView::from(&a);
        assert_eq!(view.kind, "employee_hired");
        assert_eq!(view.pid, "pid-1");
        assert_eq!(view.seq, a.seq);
    }
}
