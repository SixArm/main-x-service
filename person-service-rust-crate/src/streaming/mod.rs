//! Event streaming: the [`PersonEvent`](crate::streaming::PersonEvent) envelope and producer/consumer
//! traits.
//!
//! Every CRUD/merge/link operation emits a [`PersonEvent`](crate::streaming::PersonEvent) so downstream
//! systems (analytics, cache invalidation, replication) can react. The
//! transport is pluggable behind two traits: [`EventProducer`](crate::streaming::EventProducer) publishes
//! events and [`EventConsumer`](crate::streaming::EventConsumer) reads them. The default in-process
//! implementation is [`InMemoryEventPublisher`](crate::streaming::producer::InMemoryEventPublisher) (see [`producer`](crate::streaming::producer)); a
//! Fluvio-backed transport is the intended production target. Event
//! payloads serialize via Serde with an internally-tagged `event_type`
//! discriminator, so the wire form is self-describing JSON.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::models::Person;
use crate::Result;

/// In-process and (future) Fluvio event publishers.
pub mod producer;
/// Event consumer interface (stub) for reading the stream.
pub mod consumer;

/// A domain event describing one mutation to a person record.
///
/// Serialized with an internally-tagged `event_type` field, so each
/// variant round-trips as a discriminated JSON object. Every variant
/// carries the wall-clock `timestamp` of the operation; data-bearing
/// variants embed the full [`Person`] so consumers need no follow-up
/// fetch.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
pub enum PersonEvent {
    /// A new person record was created.
    Created {
        /// The newly created person record.
        person: Person,
        /// Wall-clock time of the operation.
        timestamp: DateTime<Utc>,
    },
    /// An existing person record was updated.
    Updated {
        /// The updated person record (post-update state).
        person: Person,
        /// Wall-clock time of the operation.
        timestamp: DateTime<Utc>,
    },
    /// A person record was (soft-) deleted; only its id is carried.
    Deleted {
        /// Id of the deleted person.
        person_id: Uuid,
        /// Wall-clock time of the operation.
        timestamp: DateTime<Utc>,
    },
    /// Two records were merged: `source_id` folded into `target_id`.
    Merged {
        /// Id of the record absorbed into the target.
        source_id: Uuid,
        /// Id of the surviving record.
        target_id: Uuid,
        /// Wall-clock time of the operation.
        timestamp: DateTime<Utc>,
    },
    /// A link was created from `person_id` to `linked_id`.
    Linked {
        /// Id of the person the link originates from.
        person_id: Uuid,
        /// Id of the linked person.
        linked_id: Uuid,
        /// Wall-clock time of the operation.
        timestamp: DateTime<Utc>,
    },
    /// A link from `person_id` to `unlinked_id` was removed.
    Unlinked {
        /// Id of the person the link originated from.
        person_id: Uuid,
        /// Id of the previously linked person.
        unlinked_id: Uuid,
        /// Wall-clock time of the operation.
        timestamp: DateTime<Utc>,
    },
}

impl PersonEvent {
    /// Return the wall-clock timestamp common to every variant.
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            PersonEvent::Created { timestamp, .. } => *timestamp,
            PersonEvent::Updated { timestamp, .. } => *timestamp,
            PersonEvent::Deleted { timestamp, .. } => *timestamp,
            PersonEvent::Merged { timestamp, .. } => *timestamp,
            PersonEvent::Linked { timestamp, .. } => *timestamp,
            PersonEvent::Unlinked { timestamp, .. } => *timestamp,
        }
    }

    /// Return the primary person id the event concerns.
    ///
    /// For [`Merged`](PersonEvent::Merged) this is the `source_id` (the
    /// record being absorbed); for [`Linked`](PersonEvent::Linked) /
    /// [`Unlinked`](PersonEvent::Unlinked) it is the `person_id` end of
    /// the relationship.
    pub fn person_id(&self) -> Uuid {
        match self {
            PersonEvent::Created { person, .. } => person.id,
            PersonEvent::Updated { person, .. } => person.id,
            PersonEvent::Deleted { person_id, .. } => *person_id,
            PersonEvent::Merged { source_id, .. } => *source_id,
            PersonEvent::Linked { person_id, .. } => *person_id,
            PersonEvent::Unlinked { person_id, .. } => *person_id,
        }
    }
}

/// Publishes [`PersonEvent`]s to a stream.
///
/// Implementations must be `Send + Sync` so a single producer can be
/// shared across request handlers behind an `Arc`.
pub trait EventProducer: Send + Sync {
    /// Publish one event, returning an error if the transport fails.
    fn publish(&self, event: PersonEvent) -> Result<()>;
}

pub use producer::InMemoryEventPublisher;

/// Consumes [`PersonEvent`]s from a stream (stub interface).
pub trait EventConsumer {
    /// Begin a subscription, establishing any underlying connection.
    fn subscribe(&mut self) -> Result<()>;

    /// Pull the next event, or `Ok(None)` when the stream is exhausted.
    fn next_event(&mut self) -> Result<Option<PersonEvent>>;
}
