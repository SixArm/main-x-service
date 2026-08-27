//! Event streaming (CRUD/merge/link notifications).
//!
//! Every state-changing operation publishes an
//! [`EventEvent`](crate::streaming::EventEvent) onto a stream so
//! downstream consumers (indexers, caches, analytics) can react.
//! Publishing goes through the
//! [`EventProducer`](crate::streaming::EventProducer) trait (default
//! impl: [`InMemoryEventPublisher`](crate::streaming::producer::InMemoryEventPublisher)).
//! [`EventConsumer`](crate::streaming::EventConsumer) is the read-side
//! counterpart; it currently has no implementation in this crate.
//! Durable production delivery to Fluvio is handled separately by the
//! outbox + relay (`src/relay.rs`), not by these two traits.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::Result;
use crate::models::Event;

/// Canonical durable-bus envelope + transport selector (event-bus.md
/// §4/§7). Sits alongside the legacy [`EventEvent`] ring buffer.
pub mod envelope;
/// Producer implementations (in-memory publisher).
pub mod producer;

pub use envelope::{
    ENTITY, Envelope, EventKind, EventTransport, EventView, SCHEMA_VERSION, transport,
};

/// A streamed domain event describing one change to an event record.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
pub enum EventEvent {
    /// An event record was created.
    Created {
        /// The newly created event.
        event: Event,
        /// When the change occurred.
        timestamp: DateTime<Utc>,
    },
    /// An event record was updated.
    Updated {
        /// The updated event.
        event: Event,
        /// When the change occurred.
        timestamp: DateTime<Utc>,
    },
    /// An event record was (soft) deleted.
    Deleted {
        /// Id of the deleted event.
        event_id: Uuid,
        /// When the change occurred.
        timestamp: DateTime<Utc>,
    },
    /// Two records were merged.
    Merged {
        /// Id of the source (absorbed) event.
        source_id: Uuid,
        /// Id of the surviving target event.
        target_id: Uuid,
        /// When the change occurred.
        timestamp: DateTime<Utc>,
    },
    /// A link was created between two events.
    Linked {
        /// Id of the event a link was added to.
        event_id: Uuid,
        /// Id of the newly linked event.
        linked_id: Uuid,
        /// When the change occurred.
        timestamp: DateTime<Utc>,
    },
    /// A link between two events was removed.
    Unlinked {
        /// Id of the event a link was removed from.
        event_id: Uuid,
        /// Id of the unlinked event.
        unlinked_id: Uuid,
        /// When the change occurred.
        timestamp: DateTime<Utc>,
    },
}

impl EventEvent {
    /// Return the timestamp carried by any variant.
    #[must_use]
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            EventEvent::Created { timestamp, .. }
            | EventEvent::Updated { timestamp, .. }
            | EventEvent::Deleted { timestamp, .. }
            | EventEvent::Merged { timestamp, .. }
            | EventEvent::Linked { timestamp, .. }
            | EventEvent::Unlinked { timestamp, .. } => *timestamp,
        }
    }

    /// Return the primary event id involved (for `Merged`, the source id).
    #[must_use]
    pub fn event_id(&self) -> Uuid {
        match self {
            EventEvent::Created { event, .. } | EventEvent::Updated { event, .. } => event.id,
            EventEvent::Merged { source_id, .. } => *source_id,
            EventEvent::Deleted { event_id, .. }
            | EventEvent::Linked { event_id, .. }
            | EventEvent::Unlinked { event_id, .. } => *event_id,
        }
    }
}

/// Strategy trait for publishing [`EventEvent`]s. Object-safe so it can
/// be held as `Arc<dyn EventProducer>` in shared application state.
pub trait EventProducer: Send + Sync {
    /// Publish one event onto the stream.
    ///
    /// # Errors
    ///
    /// Returns an error if the event cannot be published to the stream.
    fn publish(&self, event: EventEvent) -> Result<()>;
}

pub use producer::InMemoryEventPublisher;

/// Strategy trait for consuming [`EventEvent`]s from the stream.
pub trait EventConsumer {
    /// Begin a subscription to the stream.
    ///
    /// # Errors
    ///
    /// Returns an error if the subscription cannot be established.
    fn subscribe(&mut self) -> Result<()>;

    /// Pull the next event, or `None` when none is currently available.
    ///
    /// # Errors
    ///
    /// Returns an error if reading the next event from the stream fails.
    fn next_event(&mut self) -> Result<Option<EventEvent>>;
}

#[cfg(test)]
mod tests {
    use super::EventEvent;
    use crate::models::{Event, Location, Place};
    use chrono::{TimeZone, Utc};

    /// A `Created` envelope carrying geo coordinates round-trips.
    ///
    /// [`EventEvent`] is internally tagged (`#[serde(tag =
    /// "event_type")]`) and carries a whole [`Event`], so it buffers
    /// through serde's `Content` exactly as [`Location`] does. Under
    /// `serde_json`'s `arbitrary_precision` an `f64` coordinate read
    /// back from that buffer fails with `invalid type: map, expected
    /// f64` — which would have broken every bus consumer for any event
    /// with a venue position, silently, since nothing covered this path.
    /// Exact decimals have no such problem.
    #[test]
    fn created_envelope_with_coordinates_round_trips() {
        let mut event = Event::new(
            "Concert",
            Utc.with_ymd_and_hms(2026, 1, 15, 9, 0, 0).unwrap(),
        );
        event.location.push(Location::Place(Place {
            id: None,
            name: "Zellerbach Hall".into(),
            address: None,
            latitude_as_decimal_degrees: Some("37.87".parse().unwrap()),
            longitude_as_decimal_degrees: Some("-122.254".parse().unwrap()),
            url: None,
        }));

        let published = EventEvent::Created {
            event,
            timestamp: Utc.with_ymd_and_hms(2026, 1, 15, 9, 0, 0).unwrap(),
        };
        let json = serde_json::to_string(&published).unwrap();
        assert!(
            json.contains(r#""latitude_as_decimal_degrees":37.87"#),
            "{json}"
        );

        let consumed: EventEvent = serde_json::from_str(&json).unwrap();
        let EventEvent::Created { event, .. } = consumed else {
            panic!("expected Created");
        };
        let Some(Location::Place(place)) = event.location.first() else {
            panic!("expected a Place location");
        };
        assert_eq!(
            place.latitude_as_decimal_degrees,
            Some("37.87".parse().unwrap())
        );
        assert_eq!(
            place.longitude_as_decimal_degrees,
            Some("-122.254".parse().unwrap())
        );
    }
}
