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
//! Export masking + `include_soft_deleted` gating (rollout step 3) are
//! implemented: an export defaults to the **masked** read view
//! ([`MaskingProfile::Masked`]) and only an elevated caller may request
//! the **full** (unmasked) profile or soft-deleted inclusion; every
//! export is audited.
//!
//! Deferred to later rollout steps (noted, not built): CSV + Parquet
//! formats, keyless-row → duplicate-review routing, the S3 artifact
//! store, and a real soft-deleted-record export query (today
//! `include_soft_deleted = true` is rejected as not-yet-supported rather
//! than silently leaking or ignoring the flag).

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

/// SEC-B2 — the maximum size, in bytes, of an uploaded **import** artifact.
///
/// The upload is read chunk-by-chunk and rejected with `413 Payload Too
/// Large` the moment the running total exceeds this, so an oversized (or
/// unbounded / chunked-transfer) upload can never be fully materialised in
/// memory — the pre-materialisation guard against an out-of-memory
/// denial-of-service. 64 MiB is a
/// generous ceiling for a JSONL person load (tens of thousands of records)
/// while staying comfortably bounded.
pub const MAX_IMPORT_BYTES: usize = 64 * 1024 * 1024;

/// SEC-B2 — the maximum number of record rows in a single **import**.
///
/// Even within [`MAX_IMPORT_BYTES`], a file of millions of tiny lines would
/// enqueue millions of per-row validate + database round-trips. The import
/// pipeline rejects the whole job (marking it `failed`) when the non-blank
/// row count exceeds this, bounding the per-job work.
pub const MAX_IMPORT_ROWS: usize = 1_000_000;

/// SEC-B2 — the maximum number of records a single **export** may
/// materialise. A caller-supplied `limit` is clamped to this
/// ([`pipeline::clamp_export_limit`](pipeline::clamp_export_limit)), so an
/// export can never be asked to buffer an unbounded result set.
pub const MAX_EXPORT_ROWS: u64 = 1_000_000;

/// SEC-B4 — the lifetime, in seconds, of a bulk job and its artifacts.
///
/// Set as `expires_at = created_at + this` when a job is created; a job (and
/// its download / error-report URL) is treated as **gone** once past this,
/// so a stale export of personal data is not indefinitely retrievable. 7
/// days is a generous window for an operator to collect a result. Physical
/// artifact deletion (a sweep of the object store) is a follow-up; the
/// expiry gate at the status handler stops the reference being handed out.
pub const BULK_ARTIFACT_TTL_SECS: i64 = 7 * 24 * 60 * 60;

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

/// The masking profile of an **export** job
/// (`agents/share/bulk-import-export.md` §8).
///
/// [`Masked`](MaskingProfile::Masked) (the default) runs every exported
/// record through [`crate::privacy::mask_person`] so a bulk export never
/// reveals more than the masked read view. [`Full`](MaskingProfile::Full)
/// leaves records unmasked and requires elevated authorisation (§8) — a
/// full extract of personal data must never be reachable by a caller who
/// could only read masked records one at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MaskingProfile {
    /// Redact sensitive fields (the default read view).
    #[default]
    Masked,
    /// Leave records unmasked (privileged — elevated authorisation).
    Full,
}

impl MaskingProfile {
    /// The persisted lowercase token (`masked` / `full`).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            MaskingProfile::Masked => "masked",
            MaskingProfile::Full => "full",
        }
    }

    /// Parse the wire token; `None` for anything but `masked` / `full`.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "masked" => Some(MaskingProfile::Masked),
            "full" => Some(MaskingProfile::Full),
            _ => None,
        }
    }

    /// Whether this is the privileged (unmasked) profile.
    #[must_use]
    pub fn is_full(self) -> bool {
        matches!(self, MaskingProfile::Full)
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
    use super::{BulkFormat, BulkKind, JobStatus, MaskingProfile};

    #[test]
    fn masking_profile_round_trips_and_defaults_masked() {
        for p in [MaskingProfile::Masked, MaskingProfile::Full] {
            assert_eq!(MaskingProfile::parse(p.as_str()), Some(p));
        }
        assert_eq!(MaskingProfile::parse("nope"), None);
        assert_eq!(MaskingProfile::default(), MaskingProfile::Masked);
        assert!(MaskingProfile::Full.is_full());
        assert!(!MaskingProfile::Masked.is_full());
    }

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
