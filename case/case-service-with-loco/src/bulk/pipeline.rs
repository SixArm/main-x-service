//! The import/export pipeline — the testable core of the bulk worker
//! (`agents/share/bulk-import-export.md` §6, §7).
//!
//! [`process_import_job`] and [`process_export_job`] carry the whole
//! per-row / per-job logic and take their collaborators (database
//! connection) as arguments, so the loco background worker
//! ([`crate::bulk::worker`]) is a thin adapter and the logic is
//! exercised directly by DB-gated tests without booting the app or the
//! live worker drain.
//!
//! **Import** (per row): parse → validate (the same [`crate::validation`]
//! rules single-create uses, so the same `422` reasons) → resolve the
//! stable key (§10.1). A row carrying a real key **upserts in place**
//! when it matches an existing record (idempotent re-import), else
//! **creates**. A **keyless** row ([`stable_key::is_keyless`] — no
//! agency-scoped case number, no pid of its own) instead runs the same
//! search-blocked matcher duplicate detection
//! [`crate::controllers::cases::check_duplicates`] uses: a likely
//! duplicate (score ≥ [`crate::bulk::IMPORT_REVIEW_THRESHOLD`]) still
//! **creates** the row (a bulk load must never silently drop legitimate
//! data) but also queues a `provenance = "import"` pair in
//! [`crate::models::review_queue`], so an operator sees it flagged.
//! Invalid rows are skipped and recorded in the error report; they never
//! abort the load. Each written row goes through
//! [`crate::streaming::create_and_emit`] /
//! [`crate::streaming::update_and_emit`], which emit the row's normal
//! event + audit exactly as the interactive `POST`/`PUT` handlers do.
//!
//! **Concurrency note (deferred, documented — not half-built):** unlike
//! the person reference implementation's `SEC-B3` advisory-lock guard
//! (which wraps its repository's find-then-write in one Postgres
//! transaction), this crate's [`crate::streaming::create_and_emit`] /
//! [`update_and_emit`](crate::streaming::update_and_emit) own their
//! *own* transaction internally (for the `outbox` event transport) and
//! offer no hook to nest an externally-held advisory lock around that
//! write without duplicating their event/audit/index logic here. Rather
//! than build a lock that only wraps the read-then-decide step — which
//! would not actually close the create-vs-update race it exists to
//! prevent, and so would be worse than no lock at all — **this rollout
//! implements and tests sequential idempotency only** (re-running the
//! same file twice upserts in place; this is what the task's required
//! test covers). True concurrent-importer race safety is left as a
//! follow-up (crate spec §13), needing either a `ConnectionTrait`-generic
//! `create_and_emit`/`update_and_emit` variant or a dedicated
//! transaction-scoped bulk write path.
//!
//! Both **JSONL** ([`jsonl`], the lossless reference) and **CSV**
//! ([`csv`], the operator/spreadsheet format) are accepted on import.
//!
//! **Export**: honours the case list/search filter, paging matching
//! records into a JSONL or CSV buffer per the job's [`BulkFormat`]. By
//! default (the [`MaskingProfile::Masked`] profile) every record is run
//! through [`crate::controllers::cases::mask_case`] before encoding —
//! see [`crate::bulk`]'s module docs, "Export governance", for why this
//! reuses the single-record masked view rather than inventing a new
//! redaction rule, and for the documented per-row-ABAC scope limitation.
//! The privileged [`MaskingProfile::Full`] profile leaves records
//! unmasked and is gated at the handler.

use case_matcher::{Case, MatchConfig, MatchingEngine};
use loco_rs::model::{ModelError, ModelResult};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::controllers::cases::mask_case;
use crate::models::cases::Model as CaseModel;
use crate::models::review_queue::{self, NewReviewItem};
use crate::streaming;

use super::BulkFormat;
use super::MaskingProfile;
use super::error_report::ErrorRow;
use super::row::BulkCaseRow;
use super::stable_key::{self, StableKey};
use super::{csv, jsonl};

/// Parameters for an import run.
#[derive(Debug, Clone, Default)]
pub struct ImportParams {
    /// Validate + classify but commit nothing (§4). Counts reflect the
    /// would-be result; no records are written.
    pub dry_run: bool,
}

/// The reconciled outcome of an import run. Invariant:
/// `rows_total == rows_created + rows_upserted + rows_errored`.
///
/// `rows_to_review` is **not** a fourth exclusive bucket — a keyless row
/// with a likely duplicate is still created (never silently dropped) and
/// *also* counted here, so `rows_to_review <= rows_created`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportOutcome {
    /// Total non-blank record rows seen.
    pub rows_total: u64,
    /// Rows inserted as new records (includes any queued for review).
    pub rows_created: u64,
    /// Rows upserted onto an existing record.
    pub rows_upserted: u64,
    /// Of `rows_created`, how many were also queued in the review queue
    /// (`provenance = "import"`) because a keyless row matched a likely
    /// duplicate above [`crate::bulk::IMPORT_REVIEW_THRESHOLD`].
    pub rows_to_review: u64,
    /// Rows that failed parse/validation/persistence.
    pub rows_errored: u64,
    /// The per-row error report (§7).
    pub errors: Vec<ErrorRow>,
}

/// Parameters for an export run — the case list/search filter (§4) plus
/// the §8 privacy controls.
#[derive(Debug, Clone)]
pub struct ExportParams {
    /// Optional title search query; when set, uses
    /// [`CaseModel::search_paged`], else pages active records via
    /// [`CaseModel::list_paged`].
    pub query: Option<String>,
    /// Max records for the listing path.
    pub limit: u64,
    /// Offset for the listing path.
    pub offset: u64,
    /// Masking profile applied to every exported record (§8). Defaults
    /// to [`MaskingProfile::Masked`]; [`MaskingProfile::Full`] is
    /// privileged.
    pub masking_profile: MaskingProfile,
    /// Whether to include soft-deleted records (§8). Defaults to
    /// `false` (active-only). `true` is privileged **and** not yet
    /// supported, so [`process_export_job`] rejects it rather than
    /// silently leaking or ignoring it.
    pub include_soft_deleted: bool,
    /// Output format — [`jsonl`] (the lossless reference) or [`csv`]
    /// (the operator/spreadsheet format). Defaults to
    /// [`BulkFormat::Jsonl`].
    pub format: BulkFormat,
}

impl Default for ExportParams {
    fn default() -> Self {
        Self {
            query: None,
            limit: 10_000,
            offset: 0,
            masking_profile: MaskingProfile::Masked,
            include_soft_deleted: false,
            format: BulkFormat::Jsonl,
        }
    }
}

/// Whether an export request needs **elevated authorisation** (§8): the
/// unmasked [`MaskingProfile::Full`] profile or soft-deleted inclusion.
/// The default (masked, active-only) export is not privileged. Pure, so
/// the handler and its tests share one definition of "privileged".
#[must_use]
pub fn export_requires_elevation(
    masking_profile: MaskingProfile,
    include_soft_deleted: bool,
) -> bool {
    masking_profile.is_full() || include_soft_deleted
}

/// Clamp a caller-supplied export `limit` to [`crate::bulk::MAX_EXPORT_ROWS`]
/// (SEC-B2), so an export can never be asked to buffer an unbounded
/// result set. Pure, so the worker's param mapping and its tests share
/// one definition of the ceiling.
#[must_use]
pub fn clamp_export_limit(requested: u64) -> u64 {
    requested.min(crate::bulk::MAX_EXPORT_ROWS)
}

/// Apply the export masking profile to a batch of records (§8): the
/// default [`MaskingProfile::Masked`] runs each record through
/// [`mask_case`]; [`MaskingProfile::Full`] returns them unchanged. Pure
/// and DB-free so it is unit-testable without a database.
#[must_use]
pub fn apply_masking(records: Vec<Case>, masking_profile: MaskingProfile) -> Vec<Case> {
    match masking_profile {
        MaskingProfile::Full => records,
        MaskingProfile::Masked => records.iter().map(mask_case).collect(),
    }
}

/// Resolve the existing record (if any) `key` points at.
async fn find_existing(db: &DatabaseConnection, key: &StableKey) -> ModelResult<Option<CaseModel>> {
    match key {
        StableKey::Pid(pid) => match CaseModel::find_by_pid(db, &pid.to_string()).await {
            Ok(model) => Ok(Some(model)),
            Err(ModelError::EntityNotFound) => Ok(None),
            Err(e) => Err(e),
        },
        StableKey::AgencyCaseNumber {
            agency_id,
            case_number,
        } => CaseModel::find_by_agency_case_number(db, agency_id, case_number).await,
    }
}

/// The idempotent per-row upsert: find the existing record (if any) by
/// the caller-resolved `key`, then update-in-place or create — through
/// [`streaming::update_and_emit`] / [`streaming::create_and_emit`], so
/// the row's normal event + audit are emitted exactly as the interactive
/// path's. See this module's docs for the deferred SEC-B3 concurrent-race
/// guarantee. Returns `(saved, was_upsert)`.
async fn import_upsert(
    db: &DatabaseConnection,
    key: &StableKey,
    row: &BulkCaseRow,
    actor: Option<&str>,
) -> ModelResult<(CaseModel, bool)> {
    if let Some(existing) = find_existing(db, key).await? {
        let updated = streaming::update_and_emit(db, existing, &row.case, actor).await?;
        Ok((updated, true))
    } else {
        let created = streaming::create_and_emit(db, &row.case, actor).await?;
        Ok((created, false))
    }
}

/// Parse index hits into UUIDs, dropping any that will not parse (same
/// posture as [`crate::controllers::cases`]'s private `parse_pids`).
fn parse_pids(hits: &[String]) -> Vec<Uuid> {
    hits.iter()
        .filter_map(|s| Uuid::parse_str(s).ok())
        .collect()
}

/// One decoded import row, format-agnostic — the shape
/// [`decode_import_rows`] normalises both [`jsonl`] and [`csv`] down to.
struct ImportRow {
    /// The parsed row, or the per-row parse error message (§7).
    parsed: std::result::Result<BulkCaseRow, String>,
}

/// Decode `input` per `format` into per-row [`ImportRow`]s, enforcing the
/// SEC-B2 row cap uniformly regardless of format.
fn decode_import_rows(input: &[u8], format: BulkFormat) -> loco_rs::Result<Vec<ImportRow>> {
    match format {
        BulkFormat::Jsonl => Ok(
            jsonl::split_lines_capped(input, crate::bulk::MAX_IMPORT_ROWS)?
                .into_iter()
                .map(|line| ImportRow {
                    parsed: jsonl::parse_line(&line).map_err(|e| e.to_string()),
                })
                .collect(),
        ),
        BulkFormat::Csv => {
            let decoded = csv::decode(input)?;
            if decoded.len() > crate::bulk::MAX_IMPORT_ROWS {
                return Err(loco_rs::Error::Message(format!(
                    "bulk import exceeds the row cap: {} rows > {}",
                    decoded.len(),
                    crate::bulk::MAX_IMPORT_ROWS
                )));
            }
            Ok(decoded
                .into_iter()
                .map(|parsed| ImportRow {
                    parsed: parsed.map_err(|e| e.to_string()),
                })
                .collect())
        }
    }
}

/// The confidence-band label for a match score, matching the
/// classification [`crate::controllers::cases::check_duplicates`] uses.
fn match_quality_label(score: f64) -> &'static str {
    if score >= 0.95 {
        "certain"
    } else if score >= 0.7 {
        "probable"
    } else {
        "possible"
    }
}

/// One candidate found for a keyless row: the existing case it matched,
/// and the match outcome.
struct KeylessDuplicate {
    /// The matched, already-stored case.
    model: CaseModel,
    /// The match score in `[0.0, 1.0]`.
    score: f64,
    /// The matcher's per-component breakdown, for the review-queue row.
    breakdown: serde_json::Value,
}

/// Find the best duplicate candidate for a **keyless** row, above
/// [`crate::bulk::IMPORT_REVIEW_THRESHOLD`], via the same search-blocking
/// + matcher path [`crate::controllers::cases::check_duplicates`] uses.
///
/// `None` when the search index is unavailable, the blocking search
/// finds nothing, or nothing clears the threshold.
async fn find_keyless_duplicate(db: &DatabaseConnection, case: &Case) -> Option<KeylessDuplicate> {
    let engine = crate::search::engine()?;
    let candidate_pids = engine
        .candidates(
            case,
            crate::controllers::cases::CHECK_DUPLICATES_CANDIDATE_LIMIT,
        )
        .ok()?;
    let rows = CaseModel::find_by_pids(db, &parse_pids(&candidate_pids))
        .await
        .ok()?;

    let matcher = MatchingEngine::new(MatchConfig::default());
    let mut best: Option<KeylessDuplicate> = None;
    for row in rows {
        let Ok(candidate) = row.to_case() else {
            continue;
        };
        let result = matcher.match_cases(case, &candidate);
        if result.score < crate::bulk::IMPORT_REVIEW_THRESHOLD {
            continue;
        }
        if best.as_ref().is_none_or(|b| result.score > b.score) {
            best = Some(KeylessDuplicate {
                model: row,
                score: result.score,
                breakdown: serde_json::to_value(&result.breakdown)
                    .unwrap_or(serde_json::Value::Null),
            });
        }
    }
    best
}

/// Create a keyless row that matched a likely duplicate, and queue the
/// pair in the stored review queue (`provenance = "import"`). Split out
/// of [`process_import_job`] to keep its per-row dispatch readable; only
/// called on the non-dry-run path (dry-run classifies without this).
async fn create_and_queue_for_review(
    db: &DatabaseConnection,
    case: &Case,
    actor: Option<&str>,
    duplicate: KeylessDuplicate,
) -> std::result::Result<CaseModel, String> {
    let saved = streaming::create_and_emit(db, case, actor)
        .await
        .map_err(|e| e.to_string())?;
    let item = NewReviewItem {
        record_id_a: saved.pid,
        record_id_b: duplicate.model.pid,
        match_score: duplicate.score,
        match_quality: match_quality_label(duplicate.score).to_string(),
        detection_method: "import_duplicate_detection".to_string(),
        score_breakdown: Some(duplicate.breakdown),
        status: "pending".to_string(),
        provenance: "import".to_string(),
    };
    if let Err(e) = review_queue::upsert(db, &[item]).await {
        tracing::warn!(
            "bulk import: failed to queue review pair for {}: {}",
            saved.pid,
            e
        );
    }
    Ok(saved)
}

/// The outcome of processing one already-decoded, already-parsed row —
/// what [`process_row`] classifies it as, before [`process_import_job`]
/// folds it into the running [`ImportOutcome`] counters.
enum RowOutcome {
    /// A plain create (keyed with no existing match, or keyless with no
    /// duplicate found).
    Created,
    /// A keyless row created **and** queued in the review queue.
    CreatedForReview,
    /// An existing record updated in place.
    Upserted,
    /// One or more problems — validation (possibly several), parse, or
    /// database — none of which committed anything for this row.
    Errors(Vec<ErrorRow>),
}

/// Classify and (unless `params.dry_run`) persist one already-parsed row.
/// Split out of [`process_import_job`] to keep it short: validation, the
/// keyless-vs-keyed dispatch (§6), and the stable-key upsert each live
/// here rather than inline in the loop.
async fn process_row(
    db: &DatabaseConnection,
    row_number: usize,
    row: BulkCaseRow,
    params: &ImportParams,
    actor: Option<&str>,
) -> RowOutcome {
    // Validate with the single-create validators (same 422 reasons).
    // `crate::validation::problems` already prefixes each message with
    // its field path (e.g. "identifiers[0]: value must not be blank"),
    // so each becomes its own error-report row rather than one row with
    // every problem concatenated.
    let problems = crate::validation::problems(&row.case);
    if !problems.is_empty() {
        return RowOutcome::Errors(
            problems
                .into_iter()
                .map(|p| ErrorRow::validation(row_number, "", p))
                .collect(),
        );
    }

    // §6: a keyless row has no stable key to look an existing record up
    // by, so it can never upsert — it runs through duplicate detection
    // instead of a blind create.
    if stable_key::is_keyless(&row) {
        return process_keyless_row(db, row_number, row, params, actor).await;
    }

    // Non-keyless: a real stable key always resolves here (`is_keyless`
    // was just checked `false`), but this is handled without a panic
    // (SEC-M4 never-panic-on-untrusted-input) rather than `.expect()`-ing
    // it — an unreachable branch is still one row's database-class error,
    // never a crash of the whole import.
    let Some(key) = stable_key::resolve_stable_key(&row) else {
        return RowOutcome::Errors(vec![ErrorRow::database(
            row_number,
            "internal: a non-keyless row unexpectedly resolved no stable key",
        )]);
    };

    if params.dry_run {
        // Classify only; no write. A concurrent create between this read
        // and a later real run is immaterial — dry-run commits nothing.
        return match find_existing(db, &key).await {
            Ok(Some(_)) => RowOutcome::Upserted,
            Ok(None) => RowOutcome::Created,
            Err(e) => RowOutcome::Errors(vec![ErrorRow::database(row_number, e.to_string())]),
        };
    }

    match import_upsert(db, &key, &row, actor).await {
        Ok((_saved, true)) => RowOutcome::Upserted,
        Ok((_saved, false)) => RowOutcome::Created,
        Err(e) => RowOutcome::Errors(vec![ErrorRow::database(row_number, e.to_string())]),
    }
}

/// The keyless branch of [`process_row`] (§6): a likely duplicate still
/// creates the row (never silently drop legitimate data) but also queues
/// it for an operator's attention; no duplicate found is a plain create,
/// exactly like a keyed row with no match.
async fn process_keyless_row(
    db: &DatabaseConnection,
    row_number: usize,
    row: BulkCaseRow,
    params: &ImportParams,
    actor: Option<&str>,
) -> RowOutcome {
    let duplicate = find_keyless_duplicate(db, &row.case).await;
    if params.dry_run {
        return if duplicate.is_some() {
            RowOutcome::CreatedForReview
        } else {
            RowOutcome::Created
        };
    }
    if let Some(dup) = duplicate {
        return match create_and_queue_for_review(db, &row.case, actor, dup).await {
            Ok(_saved) => RowOutcome::CreatedForReview,
            Err(e) => RowOutcome::Errors(vec![ErrorRow::database(row_number, e)]),
        };
    }
    match streaming::create_and_emit(db, &row.case, actor).await {
        Ok(_saved) => RowOutcome::Created,
        Err(e) => RowOutcome::Errors(vec![ErrorRow::database(row_number, e.to_string())]),
    }
}

/// Run a full import over an `input` byte buffer in the given `format`,
/// returning the reconciled [`ImportOutcome`] (including the per-row
/// error report).
///
/// Each successfully written row is persisted through
/// [`streaming::create_and_emit`] / [`streaming::update_and_emit`], which
/// emit the normal `Created`/`Updated` event and audit record. On
/// `params.dry_run`, rows are parsed, validated, and classified but
/// nothing is written (including no review-queue row for a keyless
/// duplicate).
///
/// # Errors
///
/// Returns an error only for a whole-job failure (e.g. non-UTF-8 input,
/// an unreadable CSV header, or the SEC-B2 row cap); per-row failures are
/// captured in [`ImportOutcome::errors`], not returned.
pub async fn process_import_job(
    db: &DatabaseConnection,
    input: &[u8],
    format: BulkFormat,
    params: &ImportParams,
    actor: Option<&str>,
) -> loco_rs::Result<ImportOutcome> {
    let rows = decode_import_rows(input, format)?;
    let mut outcome = ImportOutcome::default();

    for (idx, row) in rows.into_iter().enumerate() {
        let row_number = idx + 1;
        outcome.rows_total += 1;

        let row = match row.parsed {
            Ok(r) => r,
            Err(e) => {
                outcome.errors.push(ErrorRow::parse(row_number, e));
                outcome.rows_errored += 1;
                continue;
            }
        };

        match process_row(db, row_number, row, params, actor).await {
            RowOutcome::Created => outcome.rows_created += 1,
            RowOutcome::CreatedForReview => {
                outcome.rows_created += 1;
                outcome.rows_to_review += 1;
            }
            RowOutcome::Upserted => outcome.rows_upserted += 1,
            RowOutcome::Errors(errors) => {
                outcome.errors.extend(errors);
                outcome.rows_errored += 1;
            }
        }
    }

    Ok(outcome)
}

/// Run an export, returning the encoded byte buffer of matching records
/// **and** the number of records exported (for the audit row, §8). The
/// encoding is [`jsonl`] or [`csv`] per `params.format`.
///
/// Uses [`CaseModel::search_paged`] when `params.query` is set, else
/// [`CaseModel::list_paged`]. Every record is then run through
/// [`apply_masking`] per `params.masking_profile` (see [`crate::bulk`]'s
/// module docs, "Export governance").
///
/// # Errors
///
/// Returns [`loco_rs::Error::Message`] when `params.include_soft_deleted`
/// is `true` — not yet supported, so rather than silently leaking or
/// ignoring the flag the export is rejected. Also returns an error if
/// the underlying query or the format encode fails.
pub async fn process_export_job(
    db: &DatabaseConnection,
    params: &ExportParams,
) -> loco_rs::Result<(Vec<u8>, u64)> {
    if params.include_soft_deleted {
        return Err(loco_rs::Error::Message(
            "include_soft_deleted=true is not yet supported for export".to_string(),
        ));
    }
    let limit = clamp_export_limit(params.limit);
    let models = if let Some(q) = params.query.as_ref().filter(|q| !q.trim().is_empty()) {
        CaseModel::search_paged(db, q, limit, params.offset).await?
    } else {
        CaseModel::list_paged(db, limit, params.offset).await?
    };

    let cases: Vec<Case> = models.iter().filter_map(|m| m.to_case().ok()).collect();
    let masked = apply_masking(cases, params.masking_profile);
    let rows: Vec<BulkCaseRow> = models
        .iter()
        .zip(masked)
        .map(|(m, case)| BulkCaseRow::with_pid(m.pid, case))
        .collect();
    let row_count = u64::try_from(rows.len()).unwrap_or(u64::MAX);
    let bytes = match params.format {
        BulkFormat::Jsonl => jsonl::encode(&rows)?,
        BulkFormat::Csv => csv::encode(&rows)?,
    };
    Ok((bytes, row_count))
}

/// DB-free unit tests for the pure export helpers.
#[cfg(test)]
mod unit_tests {
    use super::{ExportParams, MaskingProfile, apply_masking, export_requires_elevation};
    use case_matcher::Case;

    fn case_with_case_number() -> Case {
        Case {
            case_number: Some("CN-1".to_string()),
            agency_id: Some("dhs".to_string()),
            subjects: vec!["person:abc".to_string()],
            identifiers: vec![],
            same_as: vec!["https://example.gov/x".to_string()],
            ..Case::new("A case")
        }
    }

    /// `Masked` (the default) redacts `subjects`/`same_as`/`case_number` via
    /// `mask_case`; `Full` leaves them intact.
    #[test]
    fn masking_applies_for_masked_and_skips_for_full() {
        let cases = vec![case_with_case_number()];

        let masked = apply_masking(cases.clone(), MaskingProfile::Masked);
        assert!(masked[0].subjects.is_empty(), "Masked redacts subjects");
        assert!(masked[0].same_as.is_empty(), "Masked redacts same_as");
        assert!(
            masked[0].case_number.is_none(),
            "Masked redacts case_number"
        );

        let full = apply_masking(cases, MaskingProfile::Full);
        assert_eq!(full[0].case_number.as_deref(), Some("CN-1"));
        assert_eq!(full[0].subjects, vec!["person:abc".to_string()]);

        assert_eq!(
            ExportParams::default().masking_profile,
            MaskingProfile::Masked
        );
        assert!(!ExportParams::default().include_soft_deleted);
    }

    /// Only the unmasked `Full` profile or soft-deleted inclusion needs
    /// elevation; the default (masked, active-only) does not.
    #[test]
    fn elevation_required_only_for_full_or_soft_deleted() {
        assert!(!export_requires_elevation(MaskingProfile::Masked, false));
        assert!(export_requires_elevation(MaskingProfile::Full, false));
        assert!(export_requires_elevation(MaskingProfile::Masked, true));
        assert!(export_requires_elevation(MaskingProfile::Full, true));
    }

    /// SEC-B2: a caller-supplied export `limit` is clamped to the
    /// ceiling.
    #[test]
    fn export_limit_is_clamped_to_the_ceiling() {
        use super::clamp_export_limit;
        use crate::bulk::MAX_EXPORT_ROWS;
        assert_eq!(clamp_export_limit(10), 10, "under the cap is unchanged");
        assert_eq!(clamp_export_limit(MAX_EXPORT_ROWS), MAX_EXPORT_ROWS);
        assert_eq!(clamp_export_limit(u64::MAX), MAX_EXPORT_ROWS);
    }
}

/// DB-gated (`#[ignore]`) tests for the import/export pipeline. They
/// require a migrated `PostgreSQL` via `DATABASE_URL` and are skipped by
/// a bare `cargo test`; run with
/// `DATABASE_URL=… cargo test --lib -- --ignored`. They MUST compile
/// under a bare `cargo test --lib`.
#[cfg(test)]
mod db_tests {
    use super::{
        BulkFormat, ExportParams, ImportParams, MaskingProfile, process_export_job,
        process_import_job,
    };
    use crate::bulk::row::BulkCaseRow;
    use crate::bulk::{csv, jsonl};
    use crate::models::cases::Model as CaseModel;
    use case_matcher::Case;
    use sea_orm::DatabaseConnection;
    use uuid::Uuid;

    async fn connect() -> DatabaseConnection {
        use migration::MigratorTrait as _;
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for DB tests");
        let db = sea_orm::Database::connect(&url)
            .await
            .expect("connect to DATABASE_URL");
        // Unlike the request-level suites under `tests/requests/`, which
        // boot the loco `App` (auto-migrating), these tests connect
        // directly — so migrations are applied here explicitly. Idempotent:
        // `sea-orm-migration` tracks applied migrations and skips them on
        // a later call, so running this per test function is safe.
        migration::Migrator::up(&db, None)
            .await
            .expect("run migrations");
        db
    }

    fn case_with_agency_number(title: &str, agency_id: &str, case_number: &str) -> Case {
        Case {
            case_number: Some(case_number.to_string()),
            agency_id: Some(agency_id.to_string()),
            ..Case::new(title)
        }
    }

    /// The `(agency_id, case_number)` stable key: a first import creates,
    /// and re-running the identical file upserts the same row in place
    /// (idempotent re-import).
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn import_is_idempotent_on_the_agency_case_number_key() {
        let db = connect().await;
        let agency = format!("agency-{}", Uuid::new_v4());
        let case = case_with_agency_number("Keyed by agency+number", &agency, "CN-100");
        let row = BulkCaseRow::keyless(case);
        let input = jsonl::encode(std::slice::from_ref(&row)).unwrap();

        let first = process_import_job(
            &db,
            &input,
            BulkFormat::Jsonl,
            &ImportParams::default(),
            None,
        )
        .await
        .unwrap();
        assert_eq!(first.rows_total, 1);
        assert_eq!(first.rows_created, 1, "errors: {:?}", first.errors);
        assert_eq!(first.rows_upserted, 0);
        assert_eq!(first.rows_errored, 0);

        // Re-run the identical file: upserts in place, no new row.
        let second = process_import_job(
            &db,
            &input,
            BulkFormat::Jsonl,
            &ImportParams::default(),
            None,
        )
        .await
        .unwrap();
        assert_eq!(second.rows_created, 0, "re-import creates nothing new");
        assert_eq!(second.rows_upserted, 1, "re-import upserts the same row");
        assert_eq!(second.rows_errored, 0);

        // Ground truth: exactly one row owns this (agency_id, case_number).
        let found = CaseModel::find_by_agency_case_number(&db, &agency, "CN-100")
            .await
            .unwrap()
            .expect("the row exists");
        assert_eq!(found.title, "Keyed by agency+number");
    }

    /// A row that fails validation (blank title) lands in the error
    /// report and never aborts a load carrying other valid rows.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn invalid_row_is_reported_not_fatal() {
        let db = connect().await;
        let good =
            case_with_agency_number("Valid row", &format!("agency-{}", Uuid::new_v4()), "CN-1");
        let mut bad = Case::new("   "); // blank title
        bad.agency_id = Some("agency-x".to_string());

        let mut input = jsonl::encode(&[BulkCaseRow::keyless(good)]).unwrap();
        input.extend_from_slice(
            jsonl::to_line(&BulkCaseRow::keyless(bad))
                .unwrap()
                .as_bytes(),
        );
        input.push(b'\n');

        let outcome = process_import_job(
            &db,
            &input,
            BulkFormat::Jsonl,
            &ImportParams::default(),
            None,
        )
        .await
        .unwrap();
        assert_eq!(outcome.rows_total, 2);
        assert_eq!(outcome.rows_created, 1);
        assert_eq!(outcome.rows_errored, 1);
        assert_eq!(outcome.errors[0].row_number, 2);
        assert_eq!(outcome.errors[0].code, "validation");
    }

    /// `dry_run` classifies rows (create vs upsert vs review) but writes
    /// nothing.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn dry_run_commits_nothing() {
        let db = connect().await;
        let agency = format!("agency-{}", Uuid::new_v4());
        let case = case_with_agency_number("DryRun", &agency, "CN-1");
        let input = jsonl::encode(&[BulkCaseRow::keyless(case)]).unwrap();

        let outcome = process_import_job(
            &db,
            &input,
            BulkFormat::Jsonl,
            &ImportParams { dry_run: true },
            None,
        )
        .await
        .unwrap();
        assert_eq!(outcome.rows_created, 1, "classified as create");

        assert!(
            CaseModel::find_by_agency_case_number(&db, &agency, "CN-1")
                .await
                .unwrap()
                .is_none(),
            "dry-run must not persist the record"
        );
    }

    /// A **keyless** row (no agency-scoped case number, no pid) whose
    /// title/subjects closely match an existing record is **still
    /// created** (never silently withheld) **and** queued in the stored
    /// review queue with `provenance = "import"`.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn keyless_row_with_a_likely_duplicate_creates_and_queues_for_review() {
        use crate::models::review_queue;
        use crate::streaming;

        let db = connect().await;

        // A unique title (per test run) so this run's search-blocking
        // candidates are exactly the record this test creates.
        let title = format!(
            "KeylessDupCase {}",
            &Uuid::new_v4().simple().to_string()[..8]
        );
        let existing_case = Case {
            subjects: vec!["person:shared-subject".to_string()],
            ..Case::new(&title)
        };
        let existing = streaming::create_and_emit(&db, &existing_case, None)
            .await
            .unwrap();

        // The keyless row: same title + subjects, but no agency-scoped
        // case number and no pid.
        let incoming_case = Case {
            subjects: vec!["person:shared-subject".to_string()],
            ..Case::new(&title)
        };
        let input = jsonl::encode(&[BulkCaseRow::keyless(incoming_case)]).unwrap();

        let outcome = process_import_job(
            &db,
            &input,
            BulkFormat::Jsonl,
            &ImportParams::default(),
            None,
        )
        .await
        .unwrap();

        assert_eq!(outcome.rows_total, 1);
        assert_eq!(outcome.rows_errored, 0, "errors: {:?}", outcome.errors);
        assert_eq!(outcome.rows_created, 1, "the row is created, not withheld");
        assert_eq!(
            outcome.rows_to_review, 1,
            "the likely duplicate is queued for review"
        );

        let queued = review_queue::list(&db, Some("pending"), 50).await.unwrap();
        let pair = queued
            .iter()
            .find(|r| r.record_id_a == existing.pid || r.record_id_b == existing.pid)
            .expect("a pending pair references the existing record");
        assert_eq!(pair.provenance, "import");
        assert_eq!(pair.detection_method, "import_duplicate_detection");
        assert!(pair.match_score >= crate::bulk::IMPORT_REVIEW_THRESHOLD);
    }

    /// The CSV import path creates a keyed row, matching the JSONL
    /// path's semantics with a different wire format.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn csv_import_creates_a_keyed_row() {
        let db = connect().await;
        let agency = format!("agency-{}", Uuid::new_v4());
        let case = case_with_agency_number("CsvImported", &agency, "CN-1");
        let input = csv::encode(&[BulkCaseRow::keyless(case)]).unwrap();

        let outcome =
            process_import_job(&db, &input, BulkFormat::Csv, &ImportParams::default(), None)
                .await
                .unwrap();
        assert_eq!(outcome.rows_total, 1);
        assert_eq!(outcome.rows_created, 1, "errors: {:?}", outcome.errors);

        let saved = CaseModel::find_by_agency_case_number(&db, &agency, "CN-1")
            .await
            .unwrap()
            .expect("persisted");
        assert_eq!(saved.title, "CsvImported");
    }

    /// JSONL export round-trips a created record.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn export_round_trips_through_jsonl() {
        use crate::streaming;

        let db = connect().await;
        let title = format!("Exported {}", Uuid::new_v4());
        let created = streaming::create_and_emit(&db, &Case::new(&title), None)
            .await
            .unwrap();

        let (bytes, rows) = process_export_job(
            &db,
            &ExportParams {
                query: Some(title.clone()),
                masking_profile: MaskingProfile::Full,
                ..ExportParams::default()
            },
        )
        .await
        .unwrap();

        let lines = jsonl::split_lines(&bytes).unwrap();
        assert!(!lines.is_empty());
        assert_eq!(rows, u64::try_from(lines.len()).unwrap());
        let parsed: Vec<BulkCaseRow> = lines
            .iter()
            .map(|l| jsonl::parse_line(l).unwrap())
            .collect();
        assert!(parsed.iter().any(|r| r.pid == Some(created.pid)));
    }

    /// CSV export round-trips a created record.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn export_round_trips_through_csv() {
        use crate::streaming;

        let db = connect().await;
        let title = format!("CsvExported {}", Uuid::new_v4());
        let created = streaming::create_and_emit(&db, &Case::new(&title), None)
            .await
            .unwrap();

        let (bytes, rows) = process_export_job(
            &db,
            &ExportParams {
                query: Some(title.clone()),
                masking_profile: MaskingProfile::Full,
                format: BulkFormat::Csv,
                ..ExportParams::default()
            },
        )
        .await
        .unwrap();
        assert!(rows >= 1);

        let parsed = csv::decode(&bytes).unwrap();
        assert!(
            parsed
                .iter()
                .any(|r| r.as_ref().is_ok_and(|row| row.pid == Some(created.pid))),
            "the created record round-trips through the CSV export"
        );
    }

    /// A **default** (masked) export redacts `subjects`/`same_as`/
    /// `case_number` via `mask_case`; a **full** export leaves them
    /// intact.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn export_masks_by_default_and_full_is_unmasked() {
        use crate::streaming;

        let db = connect().await;
        let title = format!("MaskedExport {}", Uuid::new_v4());
        let case = Case {
            case_number: Some("CN-SENSITIVE".to_string()),
            agency_id: Some("dhs".to_string()),
            subjects: vec!["person:sensitive-subject".to_string()],
            ..Case::new(&title)
        };
        let created = streaming::create_and_emit(&db, &case, None).await.unwrap();

        let find = |bytes: &[u8], pid: Uuid| -> BulkCaseRow {
            jsonl::split_lines(bytes)
                .unwrap()
                .iter()
                .map(|l| jsonl::parse_line(l).unwrap())
                .find(|r: &BulkCaseRow| r.pid == Some(pid))
                .expect("exported record present")
        };

        let (masked_bytes, masked_rows) = process_export_job(
            &db,
            &ExportParams {
                query: Some(title.clone()),
                ..ExportParams::default()
            },
        )
        .await
        .unwrap();
        assert!(masked_rows >= 1);
        let masked_row = find(&masked_bytes, created.pid);
        assert!(
            masked_row.case.subjects.is_empty(),
            "masked: subjects redacted"
        );
        assert!(
            masked_row.case.case_number.is_none(),
            "masked: case_number redacted"
        );

        let (full_bytes, _) = process_export_job(
            &db,
            &ExportParams {
                query: Some(title.clone()),
                masking_profile: MaskingProfile::Full,
                ..ExportParams::default()
            },
        )
        .await
        .unwrap();
        let full_row = find(&full_bytes, created.pid);
        assert_eq!(
            full_row.case.case_number.as_deref(),
            Some("CN-SENSITIVE"),
            "full: case_number intact"
        );
        assert_eq!(
            full_row.case.subjects,
            vec!["person:sensitive-subject".to_string()],
            "full: subjects intact"
        );
    }

    /// `include_soft_deleted=true` is rejected as not-yet-supported
    /// rather than leaking or silently ignoring the flag (§8).
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn export_rejects_include_soft_deleted() {
        let db = connect().await;
        let err = process_export_job(
            &db,
            &ExportParams {
                include_soft_deleted: true,
                ..ExportParams::default()
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(err, loco_rs::Error::Message(_)));
    }
}
