//! Event decode + the internal **apply seam** (spec §2.1 / §6 FR-1/2).
//!
//! [`apply_event`] is the single function a future bus consumer (or the
//! DB-gated integration tests) calls to fold one event envelope into the
//! read-model: `created`/`deleted` → presence, `linked` → edge upsert,
//! `unlinked` → edge removal, `merged` → central edge repointing onto the
//! survivor (spec §13 T-9 / design §5.3). Every consumed event advances
//! the per-topic freshness watermark. The actual Fluvio consumer,
//! idempotency via `processed_events`, and lazy verify-on-read are
//! v1-deferred (spec §13 T-6/T-10).

use chrono::{DateTime, FixedOffset, Utc};
use loco_rs::prelude::*;
use sea_orm::ConnectionTrait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use entity_ref::{EdgeKind, EntityRef, EntityType};

use crate::models::{consumer_offsets, edges, entity_presence};

/// The bus event envelope (event-bus.md §4), decoded for the aggregator.
/// Only the fields this service reads are modelled; unknown fields are
/// ignored. `entity` + `pid` name the **from** side of the event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
    /// The owning entity type token (`person`, `organization`, …).
    pub entity: String,
    /// The record's public UUID (the `from` side for link events).
    pub pid: Uuid,
    /// The event kind: `created` | `deleted` | `merged` | `linked` |
    /// `unlinked` | `updated` | …
    pub kind: String,
    /// The envelope id (idempotency key); optional in v1.
    #[serde(default)]
    pub event_id: Option<Uuid>,
    /// The per-entity monotonic sequence (bus offset), optional in v1.
    #[serde(default)]
    pub seq: Option<i64>,
    /// When the event occurred at the source (freshness watermark).
    #[serde(default)]
    pub occurred_at: Option<DateTime<FixedOffset>>,
    /// The kind-specific payload (e.g. the §4.2 `linked` detail).
    #[serde(default)]
    pub data: serde_json::Value,
}

/// Deserialize an [`EdgeKind`] from its wire token, rejecting unknown
/// kinds (the registry is closed — spec §5.4).
fn de_edge_kind<'de, D>(d: D) -> std::result::Result<EdgeKind, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    EdgeKind::from_token(&s)
        .ok_or_else(|| serde::de::Error::custom(format!("unknown edge_kind: {s:?}")))
}

/// The `linked` event `data` shape (cross-service-linking.md §4.2).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LinkedEvent {
    /// The edge id (primary key of the resulting `edges` row).
    pub edge_id: Uuid,
    /// The originating (`from`) endpoint.
    pub from_ref: EntityRef,
    /// The far (`to`) endpoint.
    pub to_ref: EntityRef,
    /// The edge kind (validated against the closed registry).
    #[serde(deserialize_with = "de_edge_kind")]
    pub edge_kind: EdgeKind,
    /// Optional role (e.g. job title for `employed_by`).
    #[serde(default)]
    pub role: Option<String>,
    /// Confidence in `[0,1]`.
    #[serde(default)]
    pub confidence: Option<f64>,
    /// Provenance wire token (stored verbatim; not interpreted in v1).
    pub provenance: String,
    /// Affiliation start (nullable).
    #[serde(default)]
    pub valid_from: Option<chrono::NaiveDate>,
    /// Affiliation end (nullable).
    #[serde(default)]
    pub valid_to: Option<chrono::NaiveDate>,
}

/// The `unlinked` event `data` shape (cross-service-linking.md §4.2):
/// the `edge_id` to remove, plus the refs for context.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct UnlinkedEvent {
    /// The edge id to remove.
    pub edge_id: Uuid,
    /// The originating endpoint (context; optional).
    #[serde(default)]
    pub from_ref: Option<EntityRef>,
    /// The far endpoint (context; optional).
    #[serde(default)]
    pub to_ref: Option<EntityRef>,
}

/// The `merged` event `data` shape (event-bus.md §4): the survivor is the
/// envelope's `entity`+`pid`; `merged_from` is the retired duplicate's
/// pid (same entity type).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct MergedEvent {
    /// The public UUID of the record that was merged away (the duplicate).
    pub merged_from: Uuid,
}

/// The bus topic name for an entity (`mxi.<entity>.events`).
#[must_use]
pub fn topic_for(entity: &str) -> String {
    format!("mxi.{entity}.events")
}

/// The entity token embedded in a `mxi.<entity>.events` topic name, or
/// the whole string if it does not match the expected shape.
#[must_use]
pub fn entity_from_topic(topic: &str) -> &str {
    topic
        .strip_prefix("mxi.")
        .and_then(|s| s.strip_suffix(".events"))
        .unwrap_or(topic)
}

/// Fold one event envelope into the read-model — the internal apply seam
/// (spec §2.1). Dispatches by `kind`; every consumed event advances the
/// per-topic freshness watermark (spec §6 FR-1). Unknown / ignored kinds
/// (`updated`) and unknown entity tokens only advance the watermark.
///
/// # Errors
///
/// Propagates decode errors (malformed `linked`/`unlinked` payloads,
/// unknown `edge_kind`) and any database error.
pub async fn apply_event<C: ConnectionTrait>(db: &C, envelope: Envelope) -> Result<()> {
    let occurred_at = envelope
        .occurred_at
        .unwrap_or_else(|| Utc::now().fixed_offset());
    let seq = envelope.seq.unwrap_or(0);

    match envelope.kind.as_str() {
        "created" | "deleted" => {
            if let Some(r) = presence_ref(&envelope) {
                let alive = envelope.kind == "created";
                entity_presence::Model::mark(db, &r, alive, seq).await?;
                // FR-10: a presence flip may change incident edge status.
                edges::Model::recompute_status_for(db, &r).await?;
            } else {
                tracing::warn!(entity = %envelope.entity, "unknown entity token; presence skipped");
            }
        }
        "linked" => {
            let ev: LinkedEvent = serde_json::from_value(envelope.data.clone())?;
            let source_event_id = envelope.event_id.unwrap_or(ev.edge_id);
            edges::Model::apply_linked(db, &ev, source_event_id, occurred_at).await?;
            // Governed (case↔person) links are audited on write too, with
            // no actor (bus-driven) — design §10.
            if crate::auth::is_governed(ev.edge_kind) {
                crate::models::audit_log::Model::record(
                    db,
                    &crate::models::audit_log::AuditContext::default(),
                    "apply_linked",
                    ev.edge_kind.as_str(),
                    &ev.from_ref.to_string(),
                    &ev.to_ref.to_string(),
                )
                .await?;
            }
        }
        "unlinked" => {
            let ev: UnlinkedEvent = serde_json::from_value(envelope.data.clone())?;
            edges::Model::apply_unlinked(db, ev.edge_id).await?;
        }
        "merged" => {
            // Merge-repointing (spec §13 T-9 / design §5.3): the survivor
            // is this envelope's ref; rewrite every edge referencing the
            // merged-away duplicate onto the survivor, centrally.
            if let Some(survivor) = presence_ref(&envelope) {
                let ev: MergedEvent = serde_json::from_value(envelope.data.clone())?;
                let merged_from = EntityRef::new(survivor.entity_type, ev.merged_from);
                // The duplicate is retired; record it and move its edges.
                entity_presence::Model::mark(db, &merged_from, false, seq).await?;
                let repointed = edges::Model::repoint_all(db, &merged_from, &survivor).await?;
                // The survivor is alive; edges now pointing at it may leave
                // `dangling` for `verified`/`unverified`.
                edges::Model::recompute_status_for(db, &survivor).await?;
                tracing::debug!(%survivor, %merged_from, repointed, "merge repointed edges");
            } else {
                tracing::warn!(entity = %envelope.entity, "merged: unknown entity token; skipped");
            }
        }
        other => {
            tracing::trace!(kind = other, "event kind ignored for graph state");
        }
    }

    let topic = topic_for(&envelope.entity);
    consumer_offsets::Model::record(db, &topic, seq, occurred_at).await?;
    Ok(())
}

/// The presence `EntityRef` for a `created`/`deleted` envelope, or `None`
/// when the entity token is not a known [`EntityType`].
fn presence_ref(envelope: &Envelope) -> Option<EntityRef> {
    EntityType::from_token(&envelope.entity).map(|t| EntityRef::new(t, envelope.pid))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn topic_helpers_round_trip() {
        assert_eq!(topic_for("person"), "mxi.person.events");
        assert_eq!(entity_from_topic("mxi.person.events"), "person");
        assert_eq!(entity_from_topic("mxi.care_pathway.events"), "care_pathway");
        // Non-matching input passes through unchanged.
        assert_eq!(entity_from_topic("weird"), "weird");
    }

    #[test]
    fn linked_event_decodes_with_typed_kind_and_refs() {
        let data = json!({
            "edge_id": "0c4f1e2a-0000-4000-8000-000000000001",
            "from_ref": "person:0c4f1e2a-0000-4000-8000-000000000002",
            "to_ref": "organization:0c4f1e2a-0000-4000-8000-000000000003",
            "edge_kind": "works_at",
            "role": "Nurse",
            "confidence": 1.0,
            "provenance": "operator",
            "valid_from": "2019-04-01",
            "valid_to": null
        });
        let ev: LinkedEvent = serde_json::from_value(data).unwrap();
        assert_eq!(ev.edge_kind, EdgeKind::WorksAt);
        assert_eq!(ev.from_ref.entity_type, EntityType::Person);
        assert_eq!(ev.to_ref.entity_type, EntityType::Organization);
        assert_eq!(ev.role.as_deref(), Some("Nurse"));
        assert_eq!(ev.provenance, "operator");
        assert_eq!(
            ev.valid_from,
            Some(chrono::NaiveDate::from_ymd_opt(2019, 4, 1).unwrap())
        );
        assert!(ev.valid_to.is_none());
    }

    #[test]
    fn linked_event_defaults_optional_fields() {
        let data = json!({
            "edge_id": "0c4f1e2a-0000-4000-8000-000000000001",
            "from_ref": "person:0c4f1e2a-0000-4000-8000-000000000002",
            "to_ref": "worker:0c4f1e2a-0000-4000-8000-000000000003",
            "edge_kind": "same_identity",
            "provenance": "import"
        });
        let ev: LinkedEvent = serde_json::from_value(data).unwrap();
        assert_eq!(ev.edge_kind, EdgeKind::SameIdentity);
        assert!(ev.role.is_none());
        assert!(ev.confidence.is_none());
        assert!(ev.valid_from.is_none());
    }

    #[test]
    fn linked_event_rejects_unknown_edge_kind() {
        let data = json!({
            "edge_id": "0c4f1e2a-0000-4000-8000-000000000001",
            "from_ref": "person:0c4f1e2a-0000-4000-8000-000000000002",
            "to_ref": "organization:0c4f1e2a-0000-4000-8000-000000000003",
            "edge_kind": "befriends",
            "provenance": "operator"
        });
        let err = serde_json::from_value::<LinkedEvent>(data).unwrap_err();
        assert!(err.to_string().contains("befriends"), "got: {err}");
    }

    #[test]
    fn linked_event_rejects_bad_ref_urn() {
        let data = json!({
            "edge_id": "0c4f1e2a-0000-4000-8000-000000000001",
            "from_ref": "widget:0c4f1e2a-0000-4000-8000-000000000002",
            "to_ref": "organization:0c4f1e2a-0000-4000-8000-000000000003",
            "edge_kind": "works_at",
            "provenance": "operator"
        });
        assert!(serde_json::from_value::<LinkedEvent>(data).is_err());
    }

    #[test]
    fn unlinked_event_decodes_with_optional_context_refs() {
        let full = json!({
            "edge_id": "0c4f1e2a-0000-4000-8000-000000000001",
            "from_ref": "person:0c4f1e2a-0000-4000-8000-000000000002",
            "to_ref": "organization:0c4f1e2a-0000-4000-8000-000000000003"
        });
        let ev: UnlinkedEvent = serde_json::from_value(full).unwrap();
        assert!(ev.from_ref.is_some() && ev.to_ref.is_some());

        let minimal = json!({ "edge_id": "0c4f1e2a-0000-4000-8000-000000000001" });
        let ev: UnlinkedEvent = serde_json::from_value(minimal).unwrap();
        assert!(ev.from_ref.is_none() && ev.to_ref.is_none());
    }

    #[test]
    fn envelope_decodes_and_resolves_presence_ref() {
        let env: Envelope = serde_json::from_value(json!({
            "entity": "person",
            "pid": "0c4f1e2a-0000-4000-8000-000000000009",
            "kind": "created",
            "seq": 7
        }))
        .unwrap();
        assert_eq!(env.kind, "created");
        assert_eq!(env.seq, Some(7));
        let r = presence_ref(&env).unwrap();
        assert_eq!(r.entity_type, EntityType::Person);
        assert_eq!(r.to_string(), "person:0c4f1e2a-0000-4000-8000-000000000009");

        // An unknown entity token resolves to no presence ref.
        let bad: Envelope = serde_json::from_value(json!({
            "entity": "widget", "pid": "0c4f1e2a-0000-4000-8000-000000000009", "kind": "created"
        }))
        .unwrap();
        assert!(presence_ref(&bad).is_none());
    }
}
