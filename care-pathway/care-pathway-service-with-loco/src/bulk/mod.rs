//! Bulk operations — durable job state and artifact storage.
//!
//! FHIR Bulk Data `$export` ([`crate::compliance::bulk`]) originally kept
//! its jobs in a process-local registry: they did not survive a restart,
//! were invisible to another replica, and a client polling through a load
//! balancer could get a `404` for a job that had genuinely succeeded. The
//! honest limits in the spec said so.
//!
//! This module supplies the durable half — the `bulk_jobs` table
//! ([`crate::models::bulk_jobs`]) plus the [`store::ArtifactStore`] the
//! output NDJSON is written to — following the family contract in
//! [`agents/share/bulk-import-export.md`](../../../../agents/share/bulk-import-export.md)
//! §3 and §12, and matching the person service's existing implementation
//! so the two do not drift.
//!
//! The native bulk import/export API (§13 T-10) is a separate, larger
//! piece of work; the table and the store are deliberately shaped for
//! both so it needs no second migration.

/// Artifact storage: the `ArtifactStore` trait and its local-filesystem
/// development backend.
pub mod store;
