//! Bulk import/export — rollout step 1 (the reference entity).
//!
//! Implements the family-wide contract in
//! `agents/share/bulk-import-export.md` for the person service: async,
//! job-based bulk import and export driven by a Postgres-backed
//! background worker (`bg_pg`), with **JSONL** as the lossless reference
//! format.
//!
//! Module map:
//! - [`store`] — the [`ArtifactStore`](store::ArtifactStore) abstraction
//!   (local-filesystem impl for dev/test; S3 in deployment).
//! - [`jsonl`] — the streaming JSONL codec (person wire type per line).
//! - [`stable_key`] — person's declared upsert key (§10.1).
//! - [`error_report`] — the per-row error report (§7).
//! - [`pipeline`] — the pure-ish
//!   [`process_import_job`](pipeline::process_import_job) /
//!   [`process_export_job`](pipeline::process_export_job) core (the
//!   testable heart of the worker).
//! - [`worker`] — the loco `BackgroundWorker` that drains `bulk_jobs`.
//! - [`handlers`] — the REST surface (§4).
//!
//! Deferred to later rollout steps (noted, not built): CSV + Parquet
//! formats, export masking profiles + `include_soft_deleted` gating,
//! keyless-row → duplicate-review routing, and the S3 artifact store.

/// The per-row error report (§7).
pub mod error_report;
/// REST handlers for the bulk import/export surface (§4).
pub mod handlers;
/// Streaming JSONL codec — the lossless reference format (§5).
pub mod jsonl;
/// The import/export per-row/per-job pipeline (the testable core).
pub mod pipeline;
/// Person's declared upsert stable key (§10.1).
pub mod stable_key;
/// Artifact storage abstraction + local-filesystem implementation (§12).
pub mod store;
/// The loco `BackgroundWorker` draining `bulk_jobs`.
pub mod worker;

use serde::{Deserialize, Serialize};

/// The kind of a bulk job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BulkKind {
    /// Load records from an uploaded file.
    Import,
    /// Extract records to a downloadable file.
    Export,
}

impl BulkKind {
    /// The persisted lowercase token (`import` / `export`).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            BulkKind::Import => "import",
            BulkKind::Export => "export",
        }
    }

    /// Parse the persisted token, or `None` if unrecognized.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "import" => Some(BulkKind::Import),
            "export" => Some(BulkKind::Export),
            _ => None,
        }
    }
}

/// The file format of a bulk job. Only [`Jsonl`](BulkFormat::Jsonl) is
/// supported in rollout step 1; CSV and Parquet are later steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BulkFormat {
    /// JSON Lines — one person wire record per line (lossless reference).
    Jsonl,
}

impl BulkFormat {
    /// The persisted lowercase token (`jsonl`).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            BulkFormat::Jsonl => "jsonl",
        }
    }

    /// Parse the persisted token. Unknown / unsupported formats (`csv`,
    /// `parquet`) return `None` in step 1.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "jsonl" => Some(BulkFormat::Jsonl),
            _ => None,
        }
    }
}

/// The lifecycle status of a bulk job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    /// Enqueued, not yet picked up by the worker.
    Queued,
    /// The worker is draining the job.
    Running,
    /// Finished with zero per-row errors.
    Completed,
    /// Finished, but at least one row landed in the error report (§7).
    CompletedWithErrors,
    /// The whole job failed (e.g. the input artifact was unreadable).
    Failed,
}

impl JobStatus {
    /// The persisted `snake_case` token.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            JobStatus::Queued => "queued",
            JobStatus::Running => "running",
            JobStatus::Completed => "completed",
            JobStatus::CompletedWithErrors => "completed_with_errors",
            JobStatus::Failed => "failed",
        }
    }

    /// Parse the persisted token, or `None` if unrecognized.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "queued" => Some(JobStatus::Queued),
            "running" => Some(JobStatus::Running),
            "completed" => Some(JobStatus::Completed),
            "completed_with_errors" => Some(JobStatus::CompletedWithErrors),
            "failed" => Some(JobStatus::Failed),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BulkFormat, BulkKind, JobStatus};

    #[test]
    fn kind_round_trips() {
        for k in [BulkKind::Import, BulkKind::Export] {
            assert_eq!(BulkKind::parse(k.as_str()), Some(k));
        }
        assert_eq!(BulkKind::parse("nope"), None);
    }

    #[test]
    fn format_only_supports_jsonl_in_step_1() {
        assert_eq!(BulkFormat::parse("jsonl"), Some(BulkFormat::Jsonl));
        assert_eq!(BulkFormat::parse("csv"), None);
        assert_eq!(BulkFormat::parse("parquet"), None);
    }

    #[test]
    fn status_round_trips() {
        for s in [
            JobStatus::Queued,
            JobStatus::Running,
            JobStatus::Completed,
            JobStatus::CompletedWithErrors,
            JobStatus::Failed,
        ] {
            assert_eq!(JobStatus::parse(s.as_str()), Some(s));
        }
    }
}
