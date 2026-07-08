//! Domain models: the `SeaORM` entities plus the hand-written CRUD and
//! query helpers layered over them.
//!
//! The `_entities` module holds the machine-generated table shapes; the
//! sibling modules (`organizations`, `audit_logs`, `merge_records`) add
//! the `impl Model`/`impl ActiveModel` helpers (create, find, search,
//! soft delete, audit/merge record) that the controllers call.

/// Machine-generated `SeaORM` entity definitions (raw table shapes).
pub mod _entities;
/// Audit-trail recording and query helpers over the `audit_logs` entity.
pub mod audit_logs;
/// Durable event-bus Phase-2 transactional-outbox write + relay helpers
/// over the `event_outbox` entity (`OutboxInsert::from_envelope`,
/// `insert_on`, `recent`, relay poll/ack).
pub mod event_outbox;
/// Merge-history recording and query helpers over the `merge_records` entity.
pub mod merge_records;
/// CRUD/search helpers over the `organizations` entity (stored payload).
pub mod organizations;
