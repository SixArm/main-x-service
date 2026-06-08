//! Event-streaming publisher (FR-18).
//!
//! Every CRUD operation on Course or CourseInstance emits an event.
//! MVP carries an in-memory `Vec` so tests can observe; a Fluvio
//! adapter is planned under a feature flag (see `spec.md §15`).

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One event in the Course stream. The `kind` discriminator names the
/// CRUD operation; payload is whatever the handler stored at the time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseEvent {
    /// Unique event id.
    pub id: Uuid,
    /// CRUD operation discriminator.
    pub kind: EventKind,
    /// Entity the event refers to — `course_id` for parent events,
    /// `instance_id` for instance events.
    pub entity_id: Uuid,
    /// For instance events, the parent `course_id`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Uuid>,
    /// Operation-specific JSON payload captured by the handler.
    pub payload: serde_json::Value,
    /// When the event was emitted.
    pub emitted_at: Timestamp,
}

impl CourseEvent {
    /// Build a parent-course event (no `parent_id`).
    pub fn course(kind: EventKind, course_id: Uuid, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind,
            entity_id: course_id,
            parent_id: None,
            payload,
            emitted_at: Timestamp::now(),
        }
    }
    /// Build an instance event, recording the parent `course_id` in
    /// `parent_id` and the `instance_id` as `entity_id`.
    pub fn instance(
        kind: EventKind,
        course_id: Uuid,
        instance_id: Uuid,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind,
            entity_id: instance_id,
            parent_id: Some(course_id),
            payload,
            emitted_at: Timestamp::now(),
        }
    }
}

/// The CRUD operation a [`CourseEvent`] represents. Serialises in
/// PascalCase (e.g. `"CourseCreated"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum EventKind {
    /// A course was created.
    CourseCreated,
    /// A course was updated.
    CourseUpdated,
    /// A course was (soft-)deleted.
    CourseDeleted,
    /// A course absorbed a duplicate via merge.
    CourseMerged,
    /// A course instance was created.
    CourseInstanceCreated,
    /// A course instance was updated.
    CourseInstanceUpdated,
    /// A course instance was (soft-)deleted.
    CourseInstanceDeleted,
}

/// Object-safe trait so `AppState` can carry `Arc<dyn EventPublisher>`.
/// `async_trait` covers async-in-trait until RPITIT is stable.
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publish one [`CourseEvent`] to the stream.
    async fn publish(&self, event: CourseEvent) -> crate::Result<()>;
}

/// MVP implementation — captures every event in a `Mutex<Vec<_>>`.
/// Useful for the future integration suite (T-12) which can read
/// `events()` after a CRUD call to assert FR-18 fired.
#[derive(Default)]
pub struct InMemoryEventPublisher {
    /// Thread-safe buffer of every event published so far.
    events: Arc<Mutex<Vec<CourseEvent>>>,
}

impl InMemoryEventPublisher {
    /// Create an empty in-memory publisher.
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot of all captured events in publish order.
    pub fn events(&self) -> Vec<CourseEvent> {
        self.events.lock().expect("events mutex poisoned").clone()
    }

    /// Number of events captured so far.
    pub fn count(&self) -> usize {
        self.events.lock().expect("events mutex poisoned").len()
    }
}

#[async_trait]
impl EventPublisher for InMemoryEventPublisher {
    /// Log the event and append it to the in-memory buffer.
    async fn publish(&self, event: CourseEvent) -> crate::Result<()> {
        tracing::debug!(?event.kind, ?event.entity_id, "course event emitted");
        self.events
            .lock()
            .map_err(|_| crate::Error::Streaming("events mutex poisoned".into()))?
            .push(event);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A published event is observable via `events()`.
    #[tokio::test]
    async fn publishes_and_observes() {
        let pub_ = InMemoryEventPublisher::new();
        let e = CourseEvent::course(
            EventKind::CourseCreated,
            Uuid::new_v4(),
            serde_json::json!({"name": "CS101"}),
        );
        pub_.publish(e.clone()).await.unwrap();
        let captured = pub_.events();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].kind, EventKind::CourseCreated);
    }

    /// `EventKind` serialises in PascalCase.
    #[test]
    fn event_kind_serialises_pascal_case() {
        let s = serde_json::to_string(&EventKind::CourseInstanceCreated).unwrap();
        assert_eq!(s, "\"CourseInstanceCreated\"");
    }
}
