//! Bulk import/export (BLK-5) — the organization half of the family-wide
//! contract in
//! `agents/share/bulk-import-export.md`: async, job-based bulk import and
//! export driven by a Postgres-backed loco background worker, with
//! **JSONL** as the lossless reference format and **CSV** as the
//! operator/spreadsheet format.
//!
//! Scope for this rollout step is deliberately bounded (see the crate
//! spec §10.7 for the full rationale): **JSONL + CSV only** (no
//! Parquet — that was a later, person-specific extra built after CSV
//! landed) and a **local-filesystem-only** [`store::ArtifactStore`] (no
//! S3 backend — a future rollout can add one behind a cargo feature
//! without a breaking trait change, since the trait is already async).
//!
//! Module map:
//! - [`store`] — the [`ArtifactStore`](store::ArtifactStore) abstraction
//!   (local-filesystem implementation only, this rollout step).
//! - [`columns`] — the shared row-flattening declaration: the wire "bulk
//!   row" shape (an optional `pid` plus every `Organization` field), and
//!   the CSV column set built from it (§10.7).
//! - [`jsonl`] — the streaming JSONL codec (one bulk row per line).
//! - [`csv`] — the CSV codec (§10.7 flattening convention).
//! - [`stable_key`] — organization's declared upsert key (§10.7: LEI →
//!   DUNS → `pid`).
//! - [`error_report`] — the per-row error report (§7).
//! - [`pipeline`] — the pure-ish
//!   [`process_import_job`](pipeline::process_import_job) /
//!   [`process_export_job`](pipeline::process_export_job) core (the
//!   testable heart of the worker). Reuses
//!   [`crate::streaming::create_and_emit`] /
//!   [`crate::streaming::update_and_emit`] for every written row, so a
//!   bulk-imported organization gets exactly the same event + audit +
//!   search-index side effects as one created interactively.
//! - [`worker`] — the loco `BackgroundWorker` that drains `bulk_jobs`.
//! - [`handlers`] — the REST surface (§4): `POST`/`GET
//!   /api/organizations/import[/{id}]`, `POST`/`GET
//!   /api/organizations/export[/{id}]`, `GET
//!   /api/organizations/bulk-jobs`.
//!
//! ## The wire "bulk row" shape
//!
//! Unlike person's `Person`, the `organization_matcher::Organization`
//! DTO carries **no id of its own** — the service assigns `pid` on
//! create (see `src/models/organizations.rs`). A bulk row is therefore
//! the organization's own fields (unchanged) plus a top-level, optional
//! `pid` — see [`columns::to_row_value`] / [`columns::from_row_value`].
//! Because `pid` is a real `Option<Uuid>` (not a field with a
//! `serde(default = "Uuid::new_v4")` fallback), there is no ambiguity
//! between "no pid given" and "a pid happened to be generated" the way
//! person's `Person::id` has — so this crate does not need person's
//! raw-line "did the row carry an explicit id" sniff
//! ([`stable_key::row_has_explicit_pid`] docs explain the "why simpler
//! here" reasoning).
//!
//! Export masking + `include_soft_deleted` gating are implemented: an
//! export defaults to the **masked** view
//! ([`MaskingProfile::Masked`], reusing [`crate::privacy::mask_organization`])
//! and only an elevated caller may request the **full** (unmasked)
//! profile; every export is audited, and the audit write **gates
//! delivery** (SEC-B8) — a failed audit write fails the job rather than
//! silently handing back an unaudited export.
//!
//! Deferred (noted, not built): the S3 artifact store, Parquet, and a
//! real soft-deleted-record export query (`include_soft_deleted = true`
//! is rejected — at the handler, before a job is even created — as
//! not-yet-supported rather than silently leaking or ignoring the flag).

/// The shared row-flattening declaration (§10.7) — the wire "bulk row"
/// shape and the CSV column set built from it.
pub mod columns;
/// CSV codec — the operator/spreadsheet format (§10.7 flattening).
pub mod csv;
/// The per-row error report (§7).
pub mod error_report;
/// REST handlers for the bulk import/export surface (§4).
pub mod handlers;
/// Streaming JSONL codec — the lossless reference format.
pub mod jsonl;
/// The import/export per-row/per-job pipeline (the testable core).
pub mod pipeline;
/// Organization's declared upsert stable key (§10.7: LEI → DUNS → `pid`).
pub mod stable_key;
/// Artifact storage abstraction + local-filesystem implementation.
pub mod store;
/// The loco `BackgroundWorker` draining `bulk_jobs`.
pub mod worker;

use serde::{Deserialize, Serialize};

/// SEC-B2 — the maximum size, in bytes, of an uploaded **import**
/// artifact. The upload is read chunk-by-chunk and rejected with `413
/// Payload Too Large` the moment the running total exceeds this, so an
/// oversized (or unbounded / chunked-transfer) upload can never be fully
/// materialised in memory. 64 MiB is a generous ceiling for a JSONL/CSV
/// organization load (tens of thousands of records) while staying
/// comfortably bounded.
pub const MAX_IMPORT_BYTES: usize = 64 * 1024 * 1024;

/// SEC-B2 — the maximum number of record rows in a single **import**.
/// Even within [`MAX_IMPORT_BYTES`], a file of millions of tiny lines
/// would enqueue millions of per-row validate + database round-trips.
/// The import pipeline rejects the whole job (marking it `failed`) when
/// the non-blank row count exceeds this, bounding the per-job work.
pub const MAX_IMPORT_ROWS: usize = 1_000_000;

/// SEC-B2 — the maximum number of records a single **export** may
/// materialise. A caller-supplied `limit` is clamped to this
/// ([`pipeline::clamp_export_limit`]), so an export can never be asked
/// to buffer an unbounded result set.
pub const MAX_EXPORT_ROWS: u64 = 1_000_000;

/// SEC-B4 — the lifetime, in seconds, of a bulk job and its artifacts.
/// Set as `expires_at = created_at + this` when a job is created; a job
/// (and its download/error-report reference) is treated as **gone**
/// once past this, so a stale export is not indefinitely retrievable. 7
/// days is a generous window for an operator to collect a result.
pub const BULK_ARTIFACT_TTL_SECS: i64 = 7 * 24 * 60 * 60;

/// The match-score threshold above which a **keyless** import row's best
/// duplicate candidate routes it to the review queue instead of a fresh
/// create. `0.7` matches `organization_matcher::Confidence::Medium`'s
/// lower bound — the same bar `POST /check-duplicates` and `POST
/// /deduplicate` already classify as a probable match — so a keyless
/// row is judged by the same bar a human caller would be shown. The
/// blocked-candidate set itself reuses
/// [`crate::controllers::organizations::CHECK_DUPLICATES_CANDIDATE_LIMIT`].
pub const IMPORT_REVIEW_THRESHOLD: f64 = 0.7;

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

/// The file format of a bulk job. BLK-5 scope is **JSONL and CSV only**
/// — no Parquet (§12's export-only Parquet lean was a person-specific
/// later extra; this rollout step depends only on BLK-1..2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BulkFormat {
    /// JSON Lines — one bulk row per line (lossless reference).
    Jsonl,
    /// CSV — the operator/spreadsheet format (§10.7 flattening
    /// convention; [`csv`] codec).
    Csv,
    /// TSV — the same flattening convention and the same codec as
    /// [`Csv`](BulkFormat::Csv), separated by tabs instead of commas.
    ///
    /// A separate format rather than a CSV option because it is what the
    /// caller names on the wire, and because the two are not
    /// interchangeable on read: a delimiter cannot be inferred safely.
    Tsv,
}

impl BulkFormat {
    /// The persisted lowercase token (`jsonl` / `csv` / `tsv`).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            BulkFormat::Jsonl => "jsonl",
            BulkFormat::Csv => "csv",
            BulkFormat::Tsv => "tsv",
        }
    }

    /// Parse the persisted token. Unrecognised tokens return `None`.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "jsonl" => Some(BulkFormat::Jsonl),
            "csv" => Some(BulkFormat::Csv),
            "tsv" => Some(BulkFormat::Tsv),
            _ => None,
        }
    }

    /// The field delimiter for the delimited-text formats, or `None` for
    /// a format that is not delimited text.
    ///
    /// Exists so the codec is chosen once, here, rather than by a `match`
    /// at every call site — CSV and TSV differ in exactly this byte, and
    /// a second place to decide it is a second place for them to drift.
    #[must_use]
    pub fn delimiter(self) -> Option<u8> {
        match self {
            BulkFormat::Jsonl => None,
            BulkFormat::Csv => Some(b','),
            BulkFormat::Tsv => Some(b'\t'),
        }
    }
}

/// The masking profile of an **export** job
/// (`agents/share/bulk-import-export.md` §8).
///
/// [`Masked`](MaskingProfile::Masked) (the default) runs every exported
/// record through [`crate::privacy::mask_organization`] so a bulk export
/// never reveals more than the masked read view.
/// [`Full`](MaskingProfile::Full) leaves records unmasked and requires
/// elevated authorisation (§8) — a full extract must never be reachable
/// by a caller who could only read masked records one at a time.
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
    fn format_supports_only_jsonl_and_csv() {
        assert_eq!(BulkFormat::parse("jsonl"), Some(BulkFormat::Jsonl));
        assert_eq!(BulkFormat::parse("csv"), Some(BulkFormat::Csv));
        assert_eq!(BulkFormat::parse("tsv"), Some(BulkFormat::Tsv));
        assert_eq!(
            BulkFormat::Jsonl.delimiter(),
            None,
            "JSONL is not delimited text"
        );
        assert_eq!(BulkFormat::Csv.delimiter(), Some(b','));
        assert_eq!(BulkFormat::Tsv.delimiter(), Some(b'\t'));
        assert_eq!(
            BulkFormat::parse("parquet"),
            None,
            "Parquet is out of scope for this rollout step"
        );
        assert_eq!(BulkFormat::parse("xml"), None, "an unknown token is None");
        for f in [BulkFormat::Jsonl, BulkFormat::Csv] {
            assert_eq!(BulkFormat::parse(f.as_str()), Some(f), "round-trips");
        }
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
