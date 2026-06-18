//! `SeaORM` Entities.
//!
//! Machine-generated entity modules (one per table). They define the raw
//! `Model` / `ActiveModel` / `Column` shapes only; the hand-written CRUD
//! and query helpers that the controllers call live one level up in
//! `models/{organizations,audit_logs,merge_records}.rs`, which re-export
//! these types.

/// `audit_logs` entity — CRUD audit-trail rows.
pub mod audit_logs;
/// `merge_records` entity — record-merge history rows.
pub mod merge_records;
/// `organizations` entity — the primary table (JSONB payload + lookup cols).
pub mod organizations;
/// Convenience re-exports of the entity `Entity` handles.
pub mod prelude;
