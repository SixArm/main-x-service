//! Domain models: the `SeaORM` entity and CRUD helpers over the stored
//! `care_pathway_matcher::CarePathway` payload.

pub mod _entities;
pub mod audit_logs;
pub mod bulk_jobs;
pub mod care_pathways;
/// Cross-service outbound edges (`entity_links`) — the write side of
/// `agents/share/cross-service-linking.md` §4.1.
pub mod entity_links;
pub mod event_outbox;
pub mod merge_records;
/// The duplicate-review queue (§13 T-10) — raw SQL, no `SeaORM` entity.
pub mod review_queue;
