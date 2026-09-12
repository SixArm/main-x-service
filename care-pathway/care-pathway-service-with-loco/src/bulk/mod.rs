//! Bulk operations — durable job state, artifact storage, and (§13
//! T-10) the native bulk import/export API.
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
//! The **native** bulk import/export API (§13 T-10) reuses that same
//! table and store rather than a second migration, scoped — mirroring
//! organization's (BLK-5) and case's own rollout — to **JSONL + CSV +
//! TSV** (no Parquet) and the existing **local-filesystem / S3**
//! [`store::ArtifactStore`] this crate already carries for FHIR Bulk
//! Data (unlike organization/case, which shipped local-only and added S3
//! later — this crate's `store.rs` already had both, so native bulk
//! inherits S3 for free).
//!
//! Module map:
//! - [`store`] — the [`ArtifactStore`](store::ArtifactStore) abstraction
//!   (local-filesystem + S3, shared with FHIR Bulk Data).
//! - [`columns`] — the shared row-flattening declaration: the wire "bulk
//!   row" shape and the CSV column set built from it (§9.4).
//! - [`jsonl`] — the streaming JSONL codec (one bulk row per line).
//! - [`csv`] — the CSV/TSV codec (§9.4 flattening).
//! - [`stable_key`] — care-pathway's declared upsert key (§9.4:
//!   deterministic identifier → provider-scoped pathway code → `pid`).
//! - [`error_report`] — the per-row error report (§7).
//! - [`pipeline`] — the pure-ish
//!   [`process_import_job`](pipeline::process_import_job) /
//!   [`process_export_job`](pipeline::process_export_job) core (the
//!   testable heart of the worker). Reuses
//!   [`crate::streaming::create_and_emit`] /
//!   [`crate::streaming::update_and_emit`] for every written row, so a
//!   bulk-imported pathway gets exactly the same event + audit +
//!   search-index side effects as one created interactively.
//! - [`worker`] — the loco `BackgroundWorker` that drains `bulk_jobs`.
//! - [`handlers`] — the REST surface (§4): `POST`/`GET
//!   /api/care-pathways/import[/{id}]`, `POST`/`GET
//!   /api/care-pathways/export[/{id}]`, `GET
//!   /api/care-pathways/bulk-jobs`.
//!
//! Export masking + `include_soft_deleted` gating are implemented: an
//! export defaults to the **masked** view ([`MaskingProfile::Masked`],
//! reusing [`crate::privacy::mask_pathway`]) and only an elevated caller
//! may request the **full** (unmasked) profile; every export is audited,
//! and the audit write **gates delivery** (SEC-B8) — a failed audit write
//! fails the job rather than silently handing back an unaudited export.
//!
//! Deferred (noted, not built): Parquet, a real soft-deleted-record
//! export query (`include_soft_deleted = true` is rejected — at the
//! handler, before a job is even created — as not-yet-supported rather
//! than silently leaking or ignoring the flag), and bulk import of the
//! `active` column (see [`columns`]'s module docs).

/// The shared row-flattening declaration (§9.4) — the wire "bulk row"
/// shape and the CSV column set built from it.
pub mod columns;
/// CSV/TSV codec — the operator/spreadsheet format (§9.4 flattening).
pub mod csv;
/// The per-row error report (§7).
pub mod error_report;
/// REST handlers for the native bulk import/export surface (§4).
pub mod handlers;
/// Streaming JSONL codec — the lossless reference format.
pub mod jsonl;
/// The import/export per-row/per-job pipeline (the testable core).
pub mod pipeline;
/// Care-pathway's declared upsert stable key (§9.4).
pub mod stable_key;
/// Artifact storage abstraction + local-filesystem/S3 implementations.
pub mod store;
/// The loco `BackgroundWorker` draining `bulk_jobs`.
pub mod worker;

use serde::{Deserialize, Serialize};

/// SEC-B2 — the maximum size, in bytes, of an uploaded **import**
/// artifact. The upload is read chunk-by-chunk and rejected with `413
/// Payload Too Large` the moment the running total exceeds this, so an
/// oversized (or unbounded / chunked-transfer) upload can never be fully
/// materialised in memory. 64 MiB is a generous ceiling for a JSONL/CSV
/// care-pathway load (tens of thousands of records) while staying
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

/// The match-score threshold above which a **keyless** import row's best
/// duplicate candidate routes it to the review queue instead of a fresh
/// create. `0.7` matches `care_pathway_matcher::Confidence::Medium`'s
/// lower bound — the same bar `POST /check-duplicates` already
/// classifies as a probable match — so a keyless row is judged by the
/// same bar a human caller would be shown. The blocked-candidate set
/// itself reuses
/// [`crate::controllers::care_pathways::CHECK_DUPLICATES_CANDIDATE_LIMIT`].
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

/// The file format of a bulk job. Native-bulk scope is **JSONL, CSV, and
/// TSV** — no Parquet (§9.4/§12's export-only Parquet lean was a
/// person-specific later extra that this rollout does not depend on).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BulkFormat {
    /// JSON Lines — one bulk row per line (lossless reference).
    Jsonl,
    /// CSV — the operator/spreadsheet format (§9.4 flattening
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

    /// Parse the persisted token. Unrecognised tokens return `None`
    /// (including `"ndjson"` — that token names an FHIR Bulk Data job in
    /// this shared table, not a native-bulk one).
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
/// record through [`crate::privacy::mask_pathway`] so a bulk export
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

#[cfg(test)]
mod tests {
    use super::{BulkFormat, BulkKind, MaskingProfile};

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
    fn format_supports_jsonl_csv_and_tsv_only() {
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
            "Parquet is out of scope for this rollout"
        );
        assert_eq!(
            BulkFormat::parse("ndjson"),
            None,
            "ndjson names an FHIR Bulk Data job in the shared table, not a native-bulk one"
        );
        for f in [BulkFormat::Jsonl, BulkFormat::Csv, BulkFormat::Tsv] {
            assert_eq!(BulkFormat::parse(f.as_str()), Some(f), "round-trips");
        }
    }
}
