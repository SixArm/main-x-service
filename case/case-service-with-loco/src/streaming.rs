//! Event stream — Phases 1 & 2 of the durable event bus.
//!
//! Every CRUD/merge action publishes a canonical [`Envelope`]. Two
//! transports sit behind the [`EventTransport`] selector (config
//! `CASE_EVENT_TRANSPORT`, default `memory`; see
//! [`agents/share/event-bus.md`](../../../agents/share/event-bus.md) §7):
//!
//! - **`memory`** (Phase 1, default) — a process-wide ring buffer behind
//!   the [`EventPublisher`] seam / [`InMemoryPublisher`]. Behaviour is
//!   identical to the original free-function ring buffer; no DB, no tx.
//! - **`outbox`** (Phase 2) — the [`OutboxPublisher`] inserts one
//!   `event_outbox` row **on the handler's transaction**, so the entity
//!   write and its event commit or roll back together (no committed
//!   change without its event, and vice versa). The transaction-aware
//!   write+emit path is exposed as [`create_and_emit`] /
//!   [`update_and_emit`] / [`delete_and_emit`] / [`merge_and_emit`], which
//!   both the native and FHIR controllers call so neither has to know the
//!   transport. Phase 3 (the Fluvio relay worker) is roadmap.
//!
//! The operator endpoint `GET /api/cases/events/recent` returns the flat
//! [`EventView`] projection (`{kind, pid, name, seq}`), a frozen wire
//! shape consumed by the front-end — served from the ring buffer
//! (`memory`) or the outbox (`outbox`) via [`recent_events`], identically.
//!
//! In loco there is no per-request shared state for this, so the buffer
//! is a `OnceLock`-initialised global.

use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};

use case_matcher::Case;
use loco_rs::prelude::ModelResult;
use sea_orm::{ConnectionTrait, DatabaseConnection, IntoActiveModel, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::audit_logs::Model as AuditModel;
use crate::models::cases::Model as CaseModel;
use crate::models::entity_links::{Model as EntityLinkModel, NewEdge};
use crate::models::event_outbox::{Model as OutboxRow, OutboxInsert};

/// Envelope schema version (§4). Bumped only on a breaking change to the
/// envelope shape; additive fields do not bump it.
pub const SCHEMA_VERSION: u32 = 1;

/// The `snake_case` entity name carried by every envelope (§4).
pub const ENTITY: &str = "case";

/// Serde default for [`Envelope::entity`] (it is never read from input).
fn default_entity() -> &'static str {
    ENTITY
}

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
    /// A cross-service outbound edge was asserted (`entity_links` write —
    /// `agents/share/cross-service-linking.md` §4.2). The edge detail
    /// rides in the envelope's [`Envelope::data`].
    Linked,
    /// A cross-service outbound edge was withdrawn (soft-deleted).
    Unlinked,
}

/// The canonical, versioned event envelope (event-bus design §4).
///
/// One shape across every entity and every transport. Phase 1 carries
/// the identity, ordering, and operator-label fields; `occurred_at` and
/// the full-record `data` snapshot arrive at the outbox stage (Phase 2),
/// so they are intentionally absent here (adding them is additive and
/// does not bump [`SCHEMA_VERSION`]).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    /// Idempotency / dedup key, end to end (UUID v4).
    pub event_id: Uuid,
    /// Envelope schema version; always [`SCHEMA_VERSION`] in Phase 1.
    pub schema_version: u32,
    /// The `snake_case` entity name; always [`ENTITY`] for this crate.
    /// It is a `&'static str` (the compile-time constant), so on the wire
    /// it serializes verbatim and on the way back in it is filled from
    /// the constant rather than borrowed from the input — a `&'static
    /// str` cannot borrow a runtime string.
    #[serde(skip_deserializing, default = "default_entity")]
    pub entity: &'static str,
    /// The kind of change.
    pub kind: EventKind,
    /// The record's public id.
    pub pid: String,
    /// Per-process monotonic sequence number.
    pub seq: u64,
    /// The user pid from the bearer token, when known.
    pub actor: Option<String>,
    /// The record's denormalised label at the time of the event.
    pub name: String,
    /// Optional structured event detail. Absent (and omitted from the
    /// wire) for the CRUD events, so their shape is byte-identical to
    /// before; carries the cross-service edge detail (§4.2) on
    /// `linked`/`unlinked`. Additive — does not bump [`SCHEMA_VERSION`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// The operator-facing projection of an [`Envelope`] (§4).
///
/// This is the exact wire shape of `GET /api/cases/events/recent`: the
/// flat `{kind, pid, name, seq}` an operator sees. It is deliberately a
/// strict subset of the envelope so the durable-bus internals can evolve
/// without changing the operator API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventView {
    /// The kind of change.
    pub kind: EventKind,
    /// The record's public id.
    pub pid: String,
    /// The record's denormalised label at the time of the event.
    pub name: String,
    /// Per-process monotonic sequence number.
    pub seq: u64,
}

impl From<&Envelope> for EventView {
    fn from(env: &Envelope) -> Self {
        Self {
            kind: env.kind,
            pid: env.pid.clone(),
            name: env.name.clone(),
            seq: env.seq,
        }
    }
}

/// The publisher seam (event-bus design §5).
///
/// Phase 1 ships the synchronous [`InMemoryPublisher`]; the durable
/// `OutboxPublisher` (Phase 2) and the Fluvio relay (Phase 3) slot in
/// behind the same trait without touching call sites.
pub trait EventPublisher: Send + Sync {
    /// Publish an event to the stream. Never fails the caller.
    fn publish(&self, env: Envelope);
    /// The most recent events (newest last), capped at `limit`, as the
    /// operator projection (§4).
    fn recent(&self, limit: usize) -> Vec<EventView>;
}

const CAPACITY: usize = 1000;

/// The in-memory ring-buffer publisher: today's behaviour, behind the
/// [`EventPublisher`] seam. Default for tests and single-node dev.
#[derive(Debug, Default)]
pub struct InMemoryPublisher {
    buffer: Mutex<VecDeque<Envelope>>,
}

impl InMemoryPublisher {
    /// Create an empty publisher.
    #[must_use]
    pub fn new() -> Self {
        Self {
            buffer: Mutex::new(VecDeque::with_capacity(CAPACITY)),
        }
    }
}

impl EventPublisher for InMemoryPublisher {
    /// Push an envelope; if the lock is poisoned the event is dropped
    /// (the audit log is the durable record).
    fn publish(&self, env: Envelope) {
        if let Ok(mut buf) = self.buffer.lock() {
            if buf.len() == CAPACITY {
                buf.pop_front();
            }
            buf.push_back(env);
        }
    }

    fn recent(&self, limit: usize) -> Vec<EventView> {
        self.buffer.lock().map_or_else(
            |_| Vec::new(),
            |buf| {
                buf.iter()
                    .rev()
                    .take(limit)
                    .rev()
                    .map(EventView::from)
                    .collect()
            },
        )
    }
}

/// The process-wide publisher (Phase 1: in-memory).
fn publisher() -> &'static InMemoryPublisher {
    static PUB: OnceLock<InMemoryPublisher> = OnceLock::new();
    PUB.get_or_init(InMemoryPublisher::new)
}

fn next_seq() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(1);
    SEQ.fetch_add(1, Ordering::Relaxed)
}

/// Build an [`Envelope`] for a change, assigning a fresh `event_id` and
/// the next per-process `seq`. Carries no structured [`Envelope::data`]
/// (the CRUD events); see [`envelope_with_data`] for the
/// `linked`/`unlinked` events.
fn envelope(kind: EventKind, pid: &str, name: &str, actor: Option<&str>) -> Envelope {
    Envelope {
        event_id: Uuid::new_v4(),
        schema_version: SCHEMA_VERSION,
        entity: ENTITY,
        kind,
        pid: pid.to_string(),
        seq: next_seq(),
        actor: actor.map(str::to_string),
        name: name.to_string(),
        data: None,
    }
}

/// Build an [`Envelope`] carrying structured [`Envelope::data`] — the
/// cross-service edge detail on `linked`/`unlinked` (§4.2).
fn envelope_with_data(
    kind: EventKind,
    pid: &str,
    name: &str,
    actor: Option<&str>,
    data: serde_json::Value,
) -> Envelope {
    Envelope {
        data: Some(data),
        ..envelope(kind, pid, name, actor)
    }
}

/// Publish an event to the in-memory stream with no known actor.
///
/// Back-compat shim over [`publish_with_actor`]; never fails.
pub fn publish(kind: EventKind, pid: &str, name: &str) {
    publish_with_actor(kind, pid, name, None);
}

/// Publish an event to the in-memory stream, recording the `actor`
/// (the verified caller `sub`) when a bearer token was presented.
/// Never fails; if the lock is poisoned the event is dropped (the audit
/// log is the durable record).
pub fn publish_with_actor(kind: EventKind, pid: &str, name: &str, actor: Option<&str>) {
    publisher().publish(envelope(kind, pid, name, actor));
}

/// The most recent events (newest last), capped at `limit`, as the
/// operator projection consumed by `/events/recent`.
#[must_use]
pub fn recent(limit: usize) -> Vec<EventView> {
    publisher().recent(limit)
}

/// Which event transport is active (event-bus.md §7). `memory` is the
/// default and today's behaviour; `outbox` durably enqueues each event on
/// the entity mutation's transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventTransport {
    /// Process-wide ring buffer; no DB, no transaction (default).
    Memory,
    /// Transactional outbox: an `event_outbox` row per event, written on
    /// the handler's transaction (Phase 2).
    Outbox,
}

impl EventTransport {
    /// Parse the `CASE_EVENT_TRANSPORT` value. `outbox` selects the
    /// outbox; `memory` and **any unrecognised value** fall back to
    /// `memory` (fail-safe to today's behaviour), case-insensitively.
    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "outbox" => Self::Outbox,
            _ => Self::Memory,
        }
    }

    /// Whether the outbox transport is selected.
    #[must_use]
    pub const fn is_outbox(self) -> bool {
        matches!(self, Self::Outbox)
    }
}

/// The process-wide transport, read once from `CASE_EVENT_TRANSPORT` and
/// cached (mirrors the auth `REQUIRE_AUTH` env pattern). Unset or
/// unrecognised ⇒ [`EventTransport::Memory`].
#[must_use]
pub fn transport() -> EventTransport {
    static TRANSPORT: OnceLock<EventTransport> = OnceLock::new();
    *TRANSPORT.get_or_init(|| {
        std::env::var("CASE_EVENT_TRANSPORT")
            .map_or(EventTransport::Memory, |v| EventTransport::parse(&v))
    })
}

/// The transactional-outbox publisher (event-bus.md §5), sitting alongside
/// [`InMemoryPublisher`]. Unlike the ring buffer, its `publish` is async
/// and takes a connection, so it can run **inside the handler's
/// transaction** — the outbox guarantee. It holds no state (the durable
/// store is Postgres), so it is a unit type.
#[derive(Debug, Clone, Copy, Default)]
pub struct OutboxPublisher;

impl OutboxPublisher {
    /// Insert one `event_outbox` row for `env` **on `conn`**. Pass a
    /// `&DatabaseTransaction` to share the entity mutation's commit
    /// boundary. `occurred_at` is stamped now (the Phase-1 envelope has no
    /// wall-clock time).
    ///
    /// # Errors
    ///
    /// When the envelope pid is not a UUID, or the insert fails.
    pub async fn publish<C: ConnectionTrait>(self, conn: &C, env: &Envelope) -> ModelResult<()> {
        let insert = OutboxInsert::from_envelope(env, chrono::Utc::now().into())?;
        insert.insert_on(conn).await?;
        Ok(())
    }

    /// The most recent outbox rows, newest first, projected to
    /// [`EventView`] (drives `/events/recent` under the `outbox`
    /// transport).
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn recent(
        self,
        db: &DatabaseConnection,
        limit: usize,
    ) -> ModelResult<Vec<EventView>> {
        OutboxRow::recent(db, u64::try_from(limit).unwrap_or(u64::MAX)).await
    }
}

/// Recent events for the operator endpoint, from whichever transport is
/// active: the ring buffer (`memory`) or the outbox table (`outbox`). The
/// wire shape — a `Vec<EventView>` — is identical either way.
///
/// # Errors
///
/// When the outbox query fails (`memory` never errors).
pub async fn recent_events(db: &DatabaseConnection, limit: usize) -> ModelResult<Vec<EventView>> {
    match transport() {
        EventTransport::Memory => Ok(recent(limit)),
        EventTransport::Outbox => OutboxPublisher.recent(db, limit).await,
    }
}

/// Best-effort audit on the pooled connection — the `memory`-transport
/// side channel. A failure is logged, never fatal: the request already
/// succeeded and audit is secondary here. (Under `outbox` the audit row
/// is written *inside* the transaction instead — `AuditModel::record(&txn,
/// …)?` — so entity + event + audit commit or roll back together, per
/// `event-bus.md` §3.)
async fn audit_best_effort(
    db: &DatabaseConnection,
    entity_pid: Uuid,
    action: &str,
    actor: Option<&str>,
    snapshot: Option<serde_json::Value>,
) {
    if let Err(err) = AuditModel::record(db, entity_pid, action, actor, snapshot).await {
        tracing::warn!(error = %err, action, "failed to write audit log");
    }
}

/// Create a case and emit its `Created` event, atomically under the active
/// transport. `memory`: insert on `db`, then push to the ring buffer
/// (today's behaviour). `outbox`: open one transaction, insert the row,
/// the `event_outbox` row, **and** the audit row on it, then commit — so a
/// crash can never persist one without the others.
///
/// # Errors
///
/// When the insert (or, for `outbox`, the transaction / outbox / audit
/// insert) fails; the transaction rolls back all writes on any error.
pub async fn create_and_emit(
    db: &DatabaseConnection,
    case: &Case,
    actor: Option<&str>,
) -> ModelResult<CaseModel> {
    match transport() {
        EventTransport::Memory => {
            let model = CaseModel::create(db, case).await?;
            publish_with_actor(
                EventKind::Created,
                &model.pid.to_string(),
                &model.title,
                actor,
            );
            audit_best_effort(db, model.pid, "created", actor, Some(model.data.clone())).await;
            Ok(model)
        }
        EventTransport::Outbox => {
            let txn = db.begin().await?;
            let model = CaseModel::create(&txn, case).await?;
            let env = envelope(
                EventKind::Created,
                &model.pid.to_string(),
                &model.title,
                actor,
            );
            OutboxPublisher.publish(&txn, &env).await?;
            // Audit joins the same transaction — atomic with entity + event.
            AuditModel::record(&txn, model.pid, "created", actor, Some(model.data.clone())).await?;
            txn.commit().await?;
            Ok(model)
        }
    }
}

/// Replace a case's payload and emit its `Updated` event, atomically under
/// the active transport (see [`create_and_emit`] for the two paths).
/// Consumes the fetched `model`.
///
/// # Errors
///
/// When the update (or, for `outbox`, the transaction / outbox insert)
/// fails.
pub async fn update_and_emit(
    db: &DatabaseConnection,
    model: CaseModel,
    case: &Case,
    actor: Option<&str>,
) -> ModelResult<CaseModel> {
    match transport() {
        EventTransport::Memory => {
            let updated = model.into_active_model().update_data(db, case).await?;
            publish_with_actor(
                EventKind::Updated,
                &updated.pid.to_string(),
                &updated.title,
                actor,
            );
            audit_best_effort(
                db,
                updated.pid,
                "updated",
                actor,
                Some(updated.data.clone()),
            )
            .await;
            Ok(updated)
        }
        EventTransport::Outbox => {
            let txn = db.begin().await?;
            let updated = model.into_active_model().update_data(&txn, case).await?;
            let env = envelope(
                EventKind::Updated,
                &updated.pid.to_string(),
                &updated.title,
                actor,
            );
            OutboxPublisher.publish(&txn, &env).await?;
            AuditModel::record(
                &txn,
                updated.pid,
                "updated",
                actor,
                Some(updated.data.clone()),
            )
            .await?;
            txn.commit().await?;
            Ok(updated)
        }
    }
}

/// Soft-delete a case and emit its `Deleted` event, atomically under the
/// active transport. Returns the record's `(pid, title)` — captured before
/// the delete — so the caller can audit and respond.
///
/// # Errors
///
/// When the soft-delete (or, for `outbox`, the transaction / outbox
/// insert) fails.
pub async fn delete_and_emit(
    db: &DatabaseConnection,
    model: CaseModel,
    actor: Option<&str>,
) -> ModelResult<(Uuid, String)> {
    let (pid, title) = (model.pid, model.title.clone());
    match transport() {
        EventTransport::Memory => {
            model.into_active_model().soft_delete(db).await?;
            publish_with_actor(EventKind::Deleted, &pid.to_string(), &title, actor);
            audit_best_effort(db, pid, "deleted", actor, None).await;
        }
        EventTransport::Outbox => {
            let txn = db.begin().await?;
            model.into_active_model().soft_delete(&txn).await?;
            let env = envelope(EventKind::Deleted, &pid.to_string(), &title, actor);
            OutboxPublisher.publish(&txn, &env).await?;
            AuditModel::record(&txn, pid, "deleted", actor, None).await?;
            txn.commit().await?;
        }
    }
    Ok((pid, title))
}

/// Fold a duplicate into a survivor and emit the pair of events (`Merged`
/// on the survivor, `Deleted` on the duplicate), atomically under the
/// active transport. Under `outbox`, both writes and both outbox rows
/// share one transaction. Returns `(merged_survivor, duplicate_pid,
/// duplicate_title)` for the caller's merge-record / audit follow-up.
///
/// # Errors
///
/// When either write (or, for `outbox`, the transaction / outbox inserts)
/// fails; the transaction rolls back the whole merge on any error.
pub async fn merge_and_emit(
    db: &DatabaseConnection,
    main: CaseModel,
    duplicate: CaseModel,
    merged_case: &Case,
    actor: Option<&str>,
) -> ModelResult<(CaseModel, Uuid, String)> {
    let (dup_pid, dup_title) = (duplicate.pid, duplicate.title.clone());
    match transport() {
        EventTransport::Memory => {
            let merged = main
                .into_active_model()
                .update_data(db, merged_case)
                .await?;
            duplicate.into_active_model().soft_delete(db).await?;
            publish_with_actor(
                EventKind::Merged,
                &merged.pid.to_string(),
                &merged.title,
                actor,
            );
            publish_with_actor(EventKind::Deleted, &dup_pid.to_string(), &dup_title, actor);
            audit_best_effort(db, merged.pid, "merged", actor, Some(merged.data.clone())).await;
            audit_best_effort(db, dup_pid, "merged_into", actor, None).await;
            Ok((merged, dup_pid, dup_title))
        }
        EventTransport::Outbox => {
            let txn = db.begin().await?;
            let merged = main
                .into_active_model()
                .update_data(&txn, merged_case)
                .await?;
            duplicate.into_active_model().soft_delete(&txn).await?;
            let merged_env = envelope(
                EventKind::Merged,
                &merged.pid.to_string(),
                &merged.title,
                actor,
            );
            OutboxPublisher.publish(&txn, &merged_env).await?;
            let deleted_env = envelope(EventKind::Deleted, &dup_pid.to_string(), &dup_title, actor);
            OutboxPublisher.publish(&txn, &deleted_env).await?;
            // Both audit rows join the same transaction as the merge.
            AuditModel::record(&txn, merged.pid, "merged", actor, Some(merged.data.clone()))
                .await?;
            AuditModel::record(&txn, dup_pid, "merged_into", actor, None).await?;
            txn.commit().await?;
            Ok((merged, dup_pid, dup_title))
        }
    }
}

/// The §4.2 `data` payload for a `linked` event — the full edge detail,
/// with `from_ref` reconstructed as this crate's `case:<from_pid>` URN.
fn edge_data(link: &EntityLinkModel) -> serde_json::Value {
    serde_json::json!({
        "edge_id": link.id,
        "from_ref": format!("{ENTITY}:{}", link.from_pid),
        "to_ref": link.to_ref,
        "edge_kind": link.kind,
        "role": link.role,
        "confidence": link.confidence,
        "provenance": link.provenance,
        "valid_from": link.valid_from,
        "valid_to": link.valid_to,
    })
}

/// The §4.2 `data` payload for an `unlinked` event — the edge id plus the
/// refs, so a consumer can remove the edge without re-deriving it.
fn unlink_data(link: &EntityLinkModel) -> serde_json::Value {
    serde_json::json!({
        "edge_id": link.id,
        "from_ref": format!("{ENTITY}:{}", link.from_pid),
        "to_ref": link.to_ref,
        "edge_kind": link.kind,
    })
}

/// Assert a cross-service outbound edge and emit its `linked` event,
/// atomically under the active transport (see [`create_and_emit`] for the
/// two paths). `name` is the originating case's title (the envelope's
/// denormalised label); the envelope `pid` is the case pid (`from` side)
/// and the edge detail rides in the envelope's `data` (§4.2). The upsert
/// is idempotent (§4.1), so a re-assert re-emits `linked` for the same
/// stable `edge_id`. Audits the write as the `linked` action.
///
/// # Errors
///
/// When the upsert (or, for `outbox`, the transaction / outbox / audit
/// insert) fails; the transaction rolls back all writes on any error.
pub async fn link_and_emit(
    db: &DatabaseConnection,
    edge: &NewEdge,
    name: &str,
    actor: Option<&str>,
) -> ModelResult<EntityLinkModel> {
    let from_pid = edge.from_pid.to_string();
    match transport() {
        EventTransport::Memory => {
            let link = EntityLinkModel::upsert(db, edge).await?;
            let data = edge_data(&link);
            publisher().publish(envelope_with_data(
                EventKind::Linked,
                &from_pid,
                name,
                actor,
                data.clone(),
            ));
            audit_best_effort(db, edge.from_pid, "linked", actor, Some(data)).await;
            Ok(link)
        }
        EventTransport::Outbox => {
            let txn = db.begin().await?;
            let link = EntityLinkModel::upsert(&txn, edge).await?;
            let data = edge_data(&link);
            let env = envelope_with_data(EventKind::Linked, &from_pid, name, actor, data.clone());
            OutboxPublisher.publish(&txn, &env).await?;
            AuditModel::record(&txn, edge.from_pid, "linked", actor, Some(data)).await?;
            txn.commit().await?;
            Ok(link)
        }
    }
}

/// Withdraw (soft-delete) a cross-service outbound edge and emit its
/// `unlinked` event, atomically under the active transport. Consumes the
/// loaded `link`; `name` is the originating case's title. Audits the
/// withdrawal as the `unlinked` action.
///
/// # Errors
///
/// When the soft-delete (or, for `outbox`, the transaction / outbox /
/// audit insert) fails.
pub async fn unlink_and_emit(
    db: &DatabaseConnection,
    link: EntityLinkModel,
    name: &str,
    actor: Option<&str>,
) -> ModelResult<()> {
    let from_pid_str = link.from_pid.to_string();
    let from_pid = link.from_pid;
    let data = unlink_data(&link);
    match transport() {
        EventTransport::Memory => {
            link.into_active_model().soft_delete(db).await?;
            publisher().publish(envelope_with_data(
                EventKind::Unlinked,
                &from_pid_str,
                name,
                actor,
                data.clone(),
            ));
            audit_best_effort(db, from_pid, "unlinked", actor, Some(data)).await;
        }
        EventTransport::Outbox => {
            let txn = db.begin().await?;
            link.into_active_model().soft_delete(&txn).await?;
            let env = envelope_with_data(
                EventKind::Unlinked,
                &from_pid_str,
                name,
                actor,
                data.clone(),
            );
            OutboxPublisher.publish(&txn, &env).await?;
            AuditModel::record(&txn, from_pid, "unlinked", actor, Some(data)).await?;
            txn.commit().await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the envelope wire contract: schema version is 1, `entity` is
    /// `"case"`, and a full serialize→deserialize round-trip preserves
    /// every field (including the `skip_deserializing` `entity`, refilled
    /// from the constant).
    #[test]
    fn envelope_serde_round_trip_and_schema_version() {
        let env = envelope(
            EventKind::Created,
            "pid-1",
            "Housing benefit appeal",
            Some("user-7"),
        );
        assert_eq!(env.schema_version, SCHEMA_VERSION);
        assert_eq!(env.schema_version, 1);
        assert_eq!(env.entity, "case");
        let json = serde_json::to_string(&env).expect("serialize envelope");
        let back: Envelope = serde_json::from_str(&json).expect("deserialize envelope");
        assert_eq!(back.event_id, env.event_id);
        assert_eq!(back.schema_version, 1);
        assert_eq!(back.entity, "case");
        assert_eq!(back.kind, EventKind::Created);
        assert_eq!(back.pid, "pid-1");
        assert_eq!(back.actor.as_deref(), Some("user-7"));
        assert_eq!(back.name, "Housing benefit appeal");
    }

    /// Pins the frozen operator projection: `EventView` serializes to
    /// exactly `{kind, pid, name, seq}` — the envelope's internal fields
    /// (`event_id`, `schema_version`, `entity`, `actor`) are not exposed.
    #[test]
    fn projection_has_exactly_the_frozen_keys() {
        let env = envelope(
            EventKind::Updated,
            "pid-9",
            "Tax credit overpayment",
            Some("user-1"),
        );
        let view = EventView::from(&env);
        let value = serde_json::to_value(&view).expect("serialize view");
        let map = value.as_object().expect("view is a JSON object");
        let mut keys: Vec<&str> = map.keys().map(String::as_str).collect();
        keys.sort_unstable();
        // The wire shape is frozen: exactly kind, pid, name, seq — the
        // envelope's event_id / schema_version / entity / actor are not
        // exposed to operators.
        assert_eq!(keys, ["kind", "name", "pid", "seq"]);
        assert_eq!(value["kind"], "updated");
        assert_eq!(value["pid"], "pid-9");
        assert_eq!(value["name"], "Tax credit overpayment");
    }

    /// A CRUD envelope carries no `data` and omits the field from the wire
    /// entirely (`skip_serializing_if`), so existing event shapes are
    /// byte-identical after the additive `data` field.
    #[test]
    fn crud_envelope_omits_data_on_the_wire() {
        let env = envelope(EventKind::Created, "pid-1", "Housing benefit appeal", None);
        assert!(env.data.is_none());
        let value = serde_json::to_value(&env).expect("serialize");
        assert!(
            !value.as_object().unwrap().contains_key("data"),
            "data must be absent for CRUD events"
        );
    }

    /// A `linked` envelope carries structured `data` (the edge detail),
    /// serializes it, and round-trips — while its `EventView` projection
    /// still exposes exactly the four frozen keys (edge detail is not
    /// leaked to the operator stream).
    #[test]
    fn linked_envelope_carries_data_but_projection_stays_frozen() {
        let data = serde_json::json!({ "edge_id": "e1", "edge_kind": "subject_of" });
        let env = envelope_with_data(
            EventKind::Linked,
            "case-pid",
            "Housing benefit appeal",
            Some("user-1"),
            data.clone(),
        );
        let value = serde_json::to_value(&env).expect("serialize");
        assert_eq!(value["kind"], "linked");
        assert_eq!(value["data"]["edge_kind"], "subject_of");
        let back: Envelope = serde_json::from_value(value).expect("round-trip");
        assert_eq!(back.data, Some(data));

        // The operator projection is unchanged: exactly {kind,name,pid,seq}.
        let view = serde_json::to_value(EventView::from(&env)).expect("view");
        let mut keys: Vec<&str> = view
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(keys, ["kind", "name", "pid", "seq"]);
    }

    /// Pins the in-memory publisher: published envelopes read back newest
    /// last, projected to `EventView`, with strictly increasing `seq`.
    #[test]
    fn in_memory_publisher_publish_and_read_back() {
        let publisher = InMemoryPublisher::new();
        publisher.publish(envelope(
            EventKind::Created,
            "pid-1",
            "Housing benefit appeal",
            None,
        ));
        publisher.publish(envelope(
            EventKind::Updated,
            "pid-1",
            "Housing benefit appeal (rev 2)",
            None,
        ));
        let events = publisher.recent(10);
        assert!(events.len() >= 2);
        let last = events.last().unwrap();
        assert_eq!(last.kind, EventKind::Updated);
        assert_eq!(last.name, "Housing benefit appeal (rev 2)");
        // Sequence numbers are monotonic.
        assert!(events.windows(2).all(|w| w[0].seq < w[1].seq));
    }

    /// The transport selector parses `outbox` (case-insensitively, with
    /// surrounding whitespace), and falls back to `memory` for `memory`,
    /// the empty string, and any unrecognised value (fail-safe to today's
    /// behaviour). The `is_outbox` helper agrees.
    #[test]
    fn transport_parse_defaults_to_memory() {
        assert_eq!(EventTransport::parse("outbox"), EventTransport::Outbox);
        assert_eq!(EventTransport::parse("  OutBox "), EventTransport::Outbox);
        assert_eq!(EventTransport::parse("memory"), EventTransport::Memory);
        assert_eq!(EventTransport::parse(""), EventTransport::Memory);
        assert_eq!(EventTransport::parse("junk"), EventTransport::Memory);
        assert!(EventTransport::parse("outbox").is_outbox());
        assert!(!EventTransport::parse("memory").is_outbox());
    }

    /// Pins actor capture on the envelope: `Some(actor)` when supplied,
    /// `None` otherwise.
    #[test]
    fn actor_is_recorded_or_none() {
        let with = envelope(
            EventKind::Created,
            "pid-1",
            "Housing benefit appeal",
            Some("user-42"),
        );
        assert_eq!(with.actor.as_deref(), Some("user-42"));
        let without = envelope(EventKind::Created, "pid-1", "Housing benefit appeal", None);
        assert_eq!(without.actor, None);
    }

    /// Pins the process-wide free functions (`publish_with_actor` /
    /// `publish` / `recent`): events for a pid come back in monotonic
    /// `seq` order via the global publisher.
    #[test]
    fn process_publish_is_monotonic_and_projects() {
        publish_with_actor(EventKind::Created, "proc-pid", "Initial", Some("user-3"));
        publish(EventKind::Updated, "proc-pid", "Revised");
        let events = recent(100);
        let mine: Vec<&EventView> = events.iter().filter(|e| e.pid == "proc-pid").collect();
        assert!(mine.len() >= 2);
        assert!(mine.windows(2).all(|w| w[0].seq < w[1].seq));
    }
}
