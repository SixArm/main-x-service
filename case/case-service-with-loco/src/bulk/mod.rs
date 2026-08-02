//! Bulk import/export — BLK-5.
//!
//! Implements the family-wide contract in
//! `agents/share/bulk-import-export.md` for the case service: async,
//! job-based bulk import and export driven by a Postgres-backed
//! background worker (loco `worker`), with **JSONL** as the lossless
//! reference format and **CSV** as the operator/spreadsheet format. Ports
//! the shape of the person service's `src/bulk/` (the family reference
//! implementation) onto this crate's loco-idiomatic layout, where the API
//! DTO **is** `case_matcher::Case` — stored verbatim as JSONB, with no
//! separate service-owned domain model to convert to/from.
//!
//! Module map:
//! - [`store`] — the [`ArtifactStore`](store::ArtifactStore) abstraction:
//!   an **async** trait (so a future object-store backend needs no
//!   breaking signature change) with a **local-filesystem-only**
//!   implementation for this rollout (§ "Scope" below).
//! - [`row`] — [`row::BulkCaseRow`], the bulk wire envelope: the stored
//!   `Case` plus an out-of-band `pid` the DTO itself does not carry.
//! - [`jsonl`] — the streaming JSONL codec (one [`row::BulkCaseRow`] per
//!   line).
//! - [`columns`] + [`csv`] — the CSV flattening declaration and codec
//!   (§5).
//! - [`stable_key`] — case's declared upsert key (§10.1 below): the
//!   agency-scoped `(agency_id, case_number)` pair, then `pid`.
//! - [`error_report`] — the per-row error report (§7).
//! - [`pipeline`] — the pure-ish
//!   [`process_import_job`](pipeline::process_import_job) /
//!   [`process_export_job`](pipeline::process_export_job) core (the
//!   testable heart of the worker).
//! - [`worker`] — the loco `BackgroundWorker` that drains `bulk_jobs`.
//! - [`handlers`] — the REST surface (§4): `POST/GET /api/cases/import[/{id}]`,
//!   `POST/GET /api/cases/export[/{id}]`, `GET /api/cases/bulk-jobs[/{id}]`.
//!
//! ## Scope (BLK-5)
//!
//! Per the task bound: **JSONL + CSV only** — no Parquet, no S3 artifact
//! store (those are person-specific extras built after its own CSV /
//! review-routing rollout steps). [`store::ArtifactStore`] is written as
//! an **async trait** (care-pathway's shape, not person's original sync
//! one) specifically so that a future S3 rollout is an additive
//! implementation, not a breaking signature change to every call site —
//! see [`store`]'s module docs for the full rationale. Only
//! [`store::LocalFsArtifactStore`] is implemented here.
//!
//! ## Export governance (case's elevated posture)
//!
//! Case data is personal data — it is what the `subject_of` cross-service
//! edge links to a person (`agents/share/cross-service-linking.md` §10) —
//! so bulk export gets the same governance as the single-record path,
//! not a laxer bulk side-door:
//!
//! - The default export **masking profile** reuses
//!   [`crate::controllers::cases::mask_case`] — the exact function behind
//!   `GET /{pid}/masked` and the masked branch of `GET /{pid}/export` —
//!   so a bulk export redacts `subjects` / `identifiers` / `same_as` /
//!   `case_number` exactly as the single-record path does. This is worth
//!   stating plainly: the family capability matrix
//!   (`agents/share/overview.md`) marks case's "Privacy masking module
//!   (`src/privacy`)" column as **absent**, and that is accurate for a
//!   *dedicated module* — but masking logic already exists, inline in
//!   the cases controller. Bulk export reuses it rather than duplicating
//!   it or inventing a new redaction rule as a side effect of this task.
//! - The privileged **`full`** (unmasked) profile requires the caller to
//!   clear [`crate::auth::authorize_record`] for
//!   [`authentication_verifier::Action::Destructive`] (mirroring the
//!   person reference implementation's export-elevation gate), a no-op
//!   when `CASE_REQUIRE_AUTH` is off. This check runs **synchronously at
//!   submission time**, against the live caller who made the `POST
//!   /api/cases/export` request.
//! - **Known, documented gap — not a silent omission:** unlike
//!   `GET /api/cases` (list) and `GET /api/cases/search`, which apply
//!   per-row SEC-G3 concealment via [`crate::auth::read_visibility`]
//!   against the *live* caller's verified claims, the bulk export
//!   **worker** does not re-apply record-level (`resource.case_type` /
//!   `status` / `priority`) ABAC per row. The reason is structural, not
//!   an oversight: the worker runs asynchronously with no live HTTP
//!   request or bearer token to evaluate against, and synthesising
//!   `Claims` from data stored at submission time — rather than an
//!   actually-verified token — would itself be an unverified
//!   privilege-check bypass path, which is a worse defect than not
//!   building the feature. **This matches the person reference
//!   implementation**, which also does not re-apply its own record-level
//!   ABAC inside its async bulk-export worker despite having the
//!   capability generally — so this is a family-wide limitation, not a
//!   case-specific shortcut, and is recorded as a follow-up in the crate
//!   spec (§13) rather than left implicit.
//! - `include_soft_deleted` defaults `false` and, like person's
//!   reference implementation, is rejected as not-yet-supported when
//!   requested `true` rather than silently leaking or ignoring it.
//! - **Every export is audited**, unconditionally (not gated behind the
//!   opt-in `CASE_AUDIT_READS` flag that governs
//!   [`crate::compliance::disclosure`] read-auditing) — actor, filter,
//!   format, row count, masking profile, and timestamp, via
//!   [`crate::models::audit_logs::Model::record`] — even for a zero-row
//!   export, and the write **gates delivery**: a failed audit write
//!   fails the job (SEC-B8), so the job never reaches `completed` with a
//!   `download_url` the audit trail cannot account for.

/// The shared column-flattening declaration (§5) — rendered by [`csv`].
pub mod columns;
/// CSV codec — the operator/spreadsheet format (§5 flattening).
pub mod csv;
/// The per-row error report (§7).
pub mod error_report;
/// REST handlers for the bulk import/export surface (§4).
pub mod handlers;
/// Streaming JSONL codec — the lossless reference format (§5).
pub mod jsonl;
/// The import/export per-row/per-job pipeline (the testable core).
pub mod pipeline;
/// The bulk wire envelope ([`row::BulkCaseRow`]) — `Case` plus the
/// out-of-band `pid` it does not itself carry.
pub mod row;
/// Case's declared upsert stable key (§10.1).
pub mod stable_key;
/// Artifact storage abstraction + local-filesystem implementation.
pub mod store;
/// The loco `BackgroundWorker` draining `bulk_jobs`.
pub mod worker;

use serde::{Deserialize, Serialize};

/// SEC-B2 — the maximum size, in bytes, of an uploaded **import** artifact.
///
/// The upload is read chunk-by-chunk and rejected the moment the running
/// total exceeds this, so an oversized (or unbounded / chunked-transfer)
/// upload can never be fully materialised in memory. 64 MiB is a generous
/// ceiling for a JSONL case load (tens of thousands of records) while
/// staying comfortably bounded — matches the person reference
/// implementation.
pub const MAX_IMPORT_BYTES: usize = 64 * 1024 * 1024;

/// SEC-B2 — the maximum number of record rows in a single **import**.
///
/// Bounds the per-job work so a file of millions of tiny lines cannot
/// enqueue millions of per-row validate + database round-trips.
pub const MAX_IMPORT_ROWS: usize = 1_000_000;

/// SEC-B2 — the maximum number of records a single **export** may
/// materialise. A caller-supplied `limit` is clamped to this
/// ([`pipeline::clamp_export_limit`]).
pub const MAX_EXPORT_ROWS: u64 = 1_000_000;

/// BLK-5 — the match-score threshold above which a **keyless** import
/// row's best duplicate candidate routes it to the review queue instead
/// of a plain create. Deliberately **looser** than
/// `case_matcher::MatchConfig::default().threshold` (0.85, the bar
/// [`crate::controllers::cases::check_duplicates`] uses for an
/// interactive `is_match` hit) — matching the person reference
/// implementation's own `IMPORT_REVIEW_THRESHOLD = 0.7`, since an
/// unattended bulk load should route a merely-*probable* duplicate to a
/// human rather than only a *certain* one; the row is still created
/// either way (§6 — a bulk load never silently withholds data).
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

/// The file format of a bulk job — `jsonl` (the lossless reference) or
/// `csv` (the operator/spreadsheet format). No Parquet in this rollout
/// (see the module docs' "Scope" section).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BulkFormat {
    /// JSON Lines — one [`row::BulkCaseRow`] per line (lossless reference).
    Jsonl,
    /// CSV — the operator/spreadsheet format (§5 flattening convention;
    /// [`csv`] codec).
    Csv,
}

impl BulkFormat {
    /// The persisted lowercase token (`jsonl` / `csv`).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            BulkFormat::Jsonl => "jsonl",
            BulkFormat::Csv => "csv",
        }
    }

    /// Parse the persisted token. Unrecognised tokens return `None`.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "jsonl" => Some(BulkFormat::Jsonl),
            "csv" => Some(BulkFormat::Csv),
            _ => None,
        }
    }
}

/// The masking profile of an **export** job (see the module docs' "Export
/// governance" section).
///
/// [`Masked`](MaskingProfile::Masked) (the default) runs every exported
/// record through [`crate::controllers::cases::mask_case`], so a bulk
/// export never reveals more than the masked read view.
/// [`Full`](MaskingProfile::Full) leaves records unmasked and requires
/// elevated authorisation.
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
    /// The whole job failed (e.g. the input artifact was unreadable, or
    /// the export audit write failed — SEC-B8).
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
        assert_eq!(
            BulkFormat::parse("parquet"),
            None,
            "parquet is out of scope for BLK-5"
        );
        assert_eq!(BulkFormat::parse("xml"), None);
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
