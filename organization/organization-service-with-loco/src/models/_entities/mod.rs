//! `SeaORM` Entities.
//!
//! Machine-generated entity modules (one per table). They define the raw
//! `Model` / `ActiveModel` / `Column` shapes only; the hand-written CRUD
//! and query helpers that the controllers call live one level up in
//! `models/{organizations,audit_logs,merge_records}.rs`, which re-export
//! these types.

/// `audit_logs` entity — CRUD audit-trail rows.
pub mod audit_logs;
/// `bulk_jobs` entity — durable state for async bulk import/export (BLK-5).
pub mod bulk_jobs;
/// `event_outbox` entity — durable event-bus Phase-2 hand-off buffer.
pub mod event_outbox;
/// `merge_records` entity — record-merge history rows.
pub mod merge_records;
/// `organizations` entity — the primary table (JSONB payload + lookup cols).
pub mod organizations;
/// Convenience re-exports of the entity `Entity` handles.
pub mod prelude;
