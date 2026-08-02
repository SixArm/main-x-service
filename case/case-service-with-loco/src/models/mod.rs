//! Domain models: the `SeaORM` entity and CRUD helpers over the stored
//! `case_matcher::Case` payload.

pub mod _entities;
pub mod audit_logs;
/// Durable state for one asynchronous bulk import/export operation
/// (BLK-5; `agents/share/bulk-import-export.md` §3).
pub mod bulk_jobs;
pub mod cases;
pub mod entity_links;
pub mod event_outbox;
pub mod merge_records;
/// The stored duplicate-review queue, including keyless bulk-import
/// duplicates (BLK-5; `agents/share/bulk-import-export.md` §6).
pub mod review_queue;
