//! The import/export pipeline — the testable core of the bulk worker
//! (`agents/share/bulk-import-export.md` §6, §7).
//!
//! [`process_import_job`] and [`process_export_job`] carry the whole
//! per-row / per-job logic and take their collaborators (database
//! connection, repository, search engine) as arguments, so the loco
//! background worker ([`crate::bulk::worker`]) is a thin adapter and the
//! logic is exercised directly by DB-gated tests without booting the app
//! or the live `bg_pg` drain.
//!
//! **Import** (per row): parse → validate (the same validators as
//! single-create, so the same `422` reasons) → resolve the stable key
//! (§10.1). A row carrying a real key **upserts in place** when it
//! matches an existing record (idempotent re-import), else **creates**.
//! A **keyless** row ([`stable_key::is_keyless`] — no strong identifier,
//! no `tax_id`, no explicit `id` of its own) instead runs the same
//! search-blocking + matcher duplicate detection `POST /check-duplicates`
//! uses: a likely duplicate (score ≥ [`crate::bulk::IMPORT_REVIEW_THRESHOLD`])
//! still **creates** the row (a bulk load must never silently drop
//! legitimate data) but also queues a `provenance = "import"` pair in the
//! stored [`crate::db::review_queue`] referencing the new record and its
//! candidate, so an operator sees it flagged rather than discovering it
//! only on a later batch scan; no candidate above threshold ⇒ a plain
//! create, same as a keyed row with no match. Invalid rows are skipped
//! and recorded in the error report; they never abort the load. Each
//! written row goes through the repository, which emits its normal
//! event + audit.
//!
//! Both **JSONL** ([`jsonl`], the lossless reference) and **CSV**
//! ([`csv`], the operator/spreadsheet format) are accepted on import,
//! selected by the job's declared [`BulkFormat`].
//!
//! **Export**: honour the person list/search filter, streaming matching
//! records to a JSONL or CSV buffer per the job's [`BulkFormat`]. By
//! default (the [`MaskingProfile::Masked`] profile) every record is run
//! through [`crate::privacy::mask_person`] before encoding, so a bulk
//! export never reveals more than the masked read view (§8); the
//! privileged [`MaskingProfile::Full`] profile leaves records unmasked
//! and is gated at the handler.
//!
//! Deferred (noted, not built): Parquet; a real soft-deleted-record
//! export query (`include_soft_deleted = true` is rejected as
//! not-yet-supported rather than leaking or ignoring it).

use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection, EntityTrait, QueryFilter,
    Statement, TransactionTrait,
};
use uuid::Uuid;

use crate::Result;
use crate::db::PersonRepository;
use crate::db::models::person_identifiers;
use crate::db::review_queue::{self, NewReviewItem};
use crate::matching::PersonMatcher;
use crate::models::Person;
use crate::privacy::mask_person;
use crate::search::SearchEngine;

use super::BulkFormat;
use super::MaskingProfile;
use super::error_report::ErrorRow;
use super::stable_key::{StableKey, resolve_stable_key};
use super::{csv, jsonl, stable_key};

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
/// *also* counted here, so `rows_to_review <= rows_created`. It answers
/// "how many of the created rows need an operator's attention", not "how
/// many rows were withheld".
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

/// Parameters for an export run — the person list/search filter (§4)
/// plus the §8 privacy controls.
#[derive(Debug, Clone)]
pub struct ExportParams {
    /// Optional family-name search query; when set, uses the repository's
    /// `search`, else pages active records via `list_active`.
    pub query: Option<String>,
    /// Max records for the unfiltered listing path.
    pub limit: u64,
    /// Offset for the unfiltered listing path.
    pub offset: u64,
    /// Masking profile applied to every exported record (§8). Defaults to
    /// [`MaskingProfile::Masked`]; [`MaskingProfile::Full`] is privileged.
    pub masking_profile: MaskingProfile,
    /// Whether to include soft-deleted records (§8). Defaults to `false`
    /// (active-only). `true` is privileged **and** not yet supported by
    /// the repository, so [`process_export_job`] rejects it rather than
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
            format: BulkFormat::Jsonl,
            include_soft_deleted: false,
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
/// (SEC-B2), so an export can never be asked to buffer an unbounded result
/// set. Pure, so the worker's param mapping and its tests share one
/// definition of the ceiling.
#[must_use]
pub fn clamp_export_limit(requested: u64) -> u64 {
    requested.min(crate::bulk::MAX_EXPORT_ROWS)
}

/// Apply the export masking profile to a batch of records (§8): the
/// default [`MaskingProfile::Masked`] runs each record through
/// [`mask_person`]; [`MaskingProfile::Full`] returns them unchanged. Pure
/// and DB-free so it is unit-testable without a database.
#[must_use]
pub fn apply_masking(records: Vec<Person>, masking_profile: MaskingProfile) -> Vec<Person> {
    match masking_profile {
        MaskingProfile::Full => records,
        MaskingProfile::Masked => records.iter().map(mask_person).collect(),
    }
}

/// Look up the non-deleted person owning an identifier `(system, value)`,
/// if any. Returns the first live match.
async fn find_by_identifier(
    db: &DatabaseConnection,
    repo: &dyn PersonRepository,
    system: &str,
    value: &str,
) -> Result<Option<Person>> {
    let rows = person_identifiers::Entity::find()
        .filter(person_identifiers::Column::System.eq(system))
        .filter(person_identifiers::Column::Value.eq(value))
        .all(db)
        .await?;
    for row in rows {
        if let Some(person) = repo.get_by_id(&row.person_id).await? {
            return Ok(Some(person));
        }
    }
    Ok(None)
}

/// Resolve the existing record (if any) that `person`'s stable key points
/// at, so the caller can decide create-vs-upsert.
async fn find_existing(
    db: &DatabaseConnection,
    repo: &dyn PersonRepository,
    person: &Person,
) -> Result<Option<Person>> {
    match resolve_stable_key(person) {
        StableKey::Pid(id) => repo.get_by_id(&id).await,
        StableKey::Identifier { system, value } => {
            find_by_identifier(db, repo, &system, &value).await
        }
    }
}

/// The lock string for a [`StableKey`] — a stable, collision-resistant
/// rendering hashed by Postgres into the advisory-lock keyspace (SEC-B3).
///
/// The `system`/`value` boundary is made unambiguous by **length-prefixing**
/// the system rather than by a separator byte. This is not a stylistic
/// choice: the original rendering used a literal `\0` separator, and
/// Postgres `text` cannot represent a NUL byte, so binding the key raised
/// `invalid byte sequence for encoding "UTF8": 0x00` and **every**
/// identifier-keyed import row failed with a `database` error. A separator
/// drawn from any other character would only move the problem, since a
/// system or value may legitimately contain it; a length prefix is
/// injective for all inputs and contains no special bytes.
fn stable_key_lock_string(key: &StableKey) -> String {
    match key {
        StableKey::Pid(id) => format!("person-pid:{id}"),
        StableKey::Identifier { system, value } => {
            format!("person-id:{}:{system}{value}", system.len())
        }
    }
}

/// SEC-B3 — the idempotent per-row upsert, serialised across concurrent
/// importers by a **transaction-scoped advisory lock** on the row's stable
/// key. Without this, two importers of the same stable key both SELECT-miss
/// in [`find_existing`] and both `create`, duplicating the record (the
/// registry intentionally permits duplicate identifiers — dedup is a
/// workflow — so a `UNIQUE(system,value)` constraint is the wrong tool).
///
/// A guard transaction takes `pg_advisory_xact_lock(hashtext(key))` and
/// holds it across find → create/update → commit, so a second importer of
/// the same key blocks until the first commits and then observes the
/// just-written record and upserts it in place. The repository's own
/// create/update transactions commit within this window; the lock (not the
/// connection) provides the mutual exclusion. Returns `(saved, was_upsert)`.
async fn import_upsert_locked(
    db: &DatabaseConnection,
    repo: &dyn PersonRepository,
    person: &mut Person,
) -> Result<(Person, bool)> {
    let key = resolve_stable_key(person);
    let guard = db.begin().await?;
    guard
        .execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT pg_advisory_xact_lock(hashtext($1)::bigint)",
            [stable_key_lock_string(&key).into()],
        ))
        .await?;

    let existing = find_existing(db, repo, person).await?;
    let result = if let Some(existing) = existing {
        // Upsert in place: keep the existing record's pid so the stable key
        // maps to one record across re-imports and concurrent importers.
        person.id = existing.id;
        repo.update(person).await.map(|saved| (saved, true))
    } else {
        if person.id == Uuid::nil() {
            person.id = Uuid::new_v4();
        }
        repo.create(person).await.map(|saved| (saved, false))
    };
    // Commit the guard to release the advisory lock only after the write has
    // committed, so a waiter observes the row. On a write error the guard is
    // dropped (rolled back) and the lock releases anyway.
    guard.commit().await?;
    result
}

/// One decoded import row, format-agnostic — the shape [`decode_import_rows`]
/// normalises both [`jsonl`] and [`csv`] down to.
struct ImportRow {
    /// Whether the row itself carried an explicit, non-null `id` (see
    /// [`stable_key::row_has_explicit_id`] / `csv::decode`'s
    /// `had_explicit_id`).
    had_explicit_id: bool,
    /// The parsed person, or the per-row parse error message (§7).
    parsed: std::result::Result<Person, String>,
}

/// Decode `input` per `format` into per-row [`ImportRow`]s, enforcing the
/// SEC-B2 row cap uniformly regardless of format (`jsonl::split_lines_capped`
/// does this inline for JSONL; CSV is checked here since `csv::decode` has
/// no cap of its own).
fn decode_import_rows(input: &[u8], format: BulkFormat) -> Result<Vec<ImportRow>> {
    match format {
        BulkFormat::Jsonl => Ok(
            jsonl::split_lines_capped(input, crate::bulk::MAX_IMPORT_ROWS)?
                .into_iter()
                .map(|line| ImportRow {
                    had_explicit_id: stable_key::row_has_explicit_id(&line),
                    parsed: jsonl::parse_line(&line).map_err(|e| e.to_string()),
                })
                .collect(),
        ),
        BulkFormat::Csv => {
            let decoded = csv::decode(input)?;
            if decoded.len() > crate::bulk::MAX_IMPORT_ROWS {
                return Err(crate::Error::Validation(format!(
                    "bulk import exceeds the row cap: {} rows > {}",
                    decoded.len(),
                    crate::bulk::MAX_IMPORT_ROWS
                )));
            }
            Ok(decoded
                .into_iter()
                .map(|(had_explicit_id, parsed)| ImportRow {
                    had_explicit_id,
                    parsed: parsed.map_err(|e| e.to_string()),
                })
                .collect())
        }
    }
}

/// The confidence-band label for a match score, matching the classification
/// `POST /check-duplicates` and `POST /deduplicate` already use.
fn match_quality_label(score: f64) -> &'static str {
    if score >= 0.95 {
        "certain"
    } else if score >= 0.7 {
        "probable"
    } else {
        "possible"
    }
}

/// Find the best duplicate candidate for a **keyless** row, above
/// [`crate::bulk::IMPORT_REVIEW_THRESHOLD`], via the same search-blocking +
/// matcher path `POST /check-duplicates` uses (family-name + birth-year
/// blocking, then full scoring over the blocked set). `None` when the
/// blocking search finds nothing or nothing clears the threshold.
async fn find_keyless_duplicate(
    repo: &dyn PersonRepository,
    search: &SearchEngine,
    matcher: &dyn PersonMatcher,
    person: &Person,
) -> Option<crate::matching::MatchResult> {
    use chrono::Datelike;

    let birth_year = person.birth_date.map(|d| d.year());
    let candidate_ids = search
        .search_by_name_and_year(&person.name.family, birth_year, 50)
        .ok()?;

    let mut candidates = Vec::new();
    for id_str in candidate_ids {
        if let Ok(pid) = Uuid::parse_str(&id_str)
            && pid != person.id
            && let Ok(Some(p)) = repo.get_by_id(&pid).await
        {
            candidates.push(p);
        }
    }
    if candidates.is_empty() {
        return None;
    }

    matcher
        .find_matches(person, &candidates)
        .ok()?
        .into_iter()
        .filter(|m| m.score >= crate::bulk::IMPORT_REVIEW_THRESHOLD)
        .max_by(|a, b| a.score.total_cmp(&b.score))
}

/// Create a keyless row that matched a likely duplicate, and queue the
/// pair in the stored review queue (`provenance = "import"`). Split out
/// of [`process_import_job`] to keep its per-row dispatch readable; only
/// called on the non-dry-run path (dry-run classifies without this).
async fn create_and_queue_for_review(
    db: &DatabaseConnection,
    repo: &dyn PersonRepository,
    search: &SearchEngine,
    mut person: Person,
    duplicate: crate::matching::MatchResult,
) -> std::result::Result<Person, String> {
    if person.id == Uuid::nil() {
        person.id = Uuid::new_v4();
    }
    let saved = repo.create(&person).await.map_err(|e| e.to_string())?;
    if let Err(e) = search.index_person(&saved) {
        tracing::warn!("bulk import: failed to index person {}: {}", saved.id, e);
    }
    let item = NewReviewItem {
        record_id_a: saved.id,
        record_id_b: duplicate.person.id,
        match_score: duplicate.score,
        match_quality: match_quality_label(duplicate.score).to_string(),
        detection_method: "import_duplicate_detection".to_string(),
        score_breakdown: serde_json::to_value(&duplicate.breakdown).ok(),
        status: "pending".to_string(),
        provenance: "import".to_string(),
    };
    if let Err(e) = review_queue::upsert(db, &[item]).await {
        tracing::warn!(
            "bulk import: failed to queue review pair for {}: {}",
            saved.id,
            e
        );
    }
    Ok(saved)
}

/// Run a full import over an `input` byte buffer in the given `format`,
/// returning the reconciled [`ImportOutcome`] (including the per-row error
/// report).
///
/// Each successfully written row is persisted through `repo`, which emits
/// its normal `created`/`updated` event and audit record; the search
/// index is updated best-effort. On `params.dry_run`, rows are parsed,
/// validated, and classified but nothing is written (including no
/// review-queue row for a keyless duplicate).
///
/// # Errors
///
/// Returns an error only for a whole-job failure (e.g. non-UTF-8 input,
/// an unreadable CSV header, or the SEC-B2 row cap); per-row failures are
/// captured in [`ImportOutcome::errors`], not returned.
pub async fn process_import_job(
    db: &DatabaseConnection,
    repo: &dyn PersonRepository,
    search: &SearchEngine,
    matcher: &dyn PersonMatcher,
    input: &[u8],
    format: BulkFormat,
    params: &ImportParams,
) -> Result<ImportOutcome> {
    let rows = decode_import_rows(input, format)?;
    let mut outcome = ImportOutcome::default();

    for (idx, row) in rows.into_iter().enumerate() {
        let row_number = idx + 1;
        outcome.rows_total += 1;

        // Parse (§7: a bad row is recorded, never fatal).
        let mut person = match row.parsed {
            Ok(p) => p,
            Err(e) => {
                outcome.errors.push(ErrorRow::parse(row_number, e));
                outcome.rows_errored += 1;
                continue;
            }
        };

        // Validate with the single-create validators (same 422 reasons).
        let validation_errors = crate::validation::validate_person(&person);
        if !validation_errors.is_empty() {
            for ve in validation_errors {
                outcome
                    .errors
                    .push(ErrorRow::validation(row_number, ve.field, ve.message));
            }
            outcome.rows_errored += 1;
            continue;
        }

        // §6: a keyless row cannot idempotently upsert, so it runs through
        // duplicate detection instead of a blind create. A likely duplicate
        // still creates the row (never silently drop legitimate data) but
        // also queues it for an operator's attention.
        let keyless_duplicate = if stable_key::is_keyless(&person, row.had_explicit_id) {
            find_keyless_duplicate(repo, search, matcher, &person).await
        } else {
            None
        };

        if let Some(m) = keyless_duplicate {
            if params.dry_run {
                outcome.rows_created += 1;
                outcome.rows_to_review += 1;
                continue;
            }
            match create_and_queue_for_review(db, repo, search, person, m).await {
                Ok(_saved) => {
                    outcome.rows_created += 1;
                    outcome.rows_to_review += 1;
                }
                Err(e) => {
                    outcome.errors.push(ErrorRow::database(row_number, e));
                    outcome.rows_errored += 1;
                }
            }
            continue;
        }

        if params.dry_run {
            // Classify only; no lock/write. A concurrent create between this
            // read and a later real run is immaterial — dry-run commits
            // nothing.
            match find_existing(db, repo, &person).await {
                Ok(existing) => {
                    if existing.is_some() {
                        outcome.rows_upserted += 1;
                    } else {
                        outcome.rows_created += 1;
                    }
                }
                Err(e) => {
                    outcome
                        .errors
                        .push(ErrorRow::database(row_number, e.to_string()));
                    outcome.rows_errored += 1;
                }
            }
            continue;
        }

        // SEC-B3: find + create/update under a stable-key advisory lock so
        // concurrent importers of the same key produce exactly one record.
        match import_upsert_locked(db, repo, &mut person).await {
            Ok((saved, was_upsert)) => {
                if let Err(e) = search.index_person(&saved) {
                    tracing::warn!("bulk import: failed to index person {}: {}", saved.id, e);
                }
                if was_upsert {
                    outcome.rows_upserted += 1;
                } else {
                    outcome.rows_created += 1;
                }
            }
            Err(e) => {
                outcome
                    .errors
                    .push(ErrorRow::database(row_number, e.to_string()));
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
/// Uses the repository's family-name `search` when `params.query` is set,
/// else pages active records via `list_active`. Every record is then run
/// through [`apply_masking`] per `params.masking_profile`, so the default
/// (`Masked`) export never reveals more than the masked read view;
/// `Full` leaves records unmasked (gated at the handler).
///
/// # Errors
///
/// Returns [`crate::Error::Validation`] when `params.include_soft_deleted`
/// is `true` — the repository cannot express a soft-deleted listing
/// without a larger change, so rather than silently leaking or ignoring
/// the flag the export is rejected as not-yet-supported. Also returns an
/// error if the underlying repository query or the format encode fails.
pub async fn process_export_job(
    repo: &dyn PersonRepository,
    params: &ExportParams,
) -> Result<(Vec<u8>, u64)> {
    if params.include_soft_deleted {
        return Err(crate::Error::Validation(
            "include_soft_deleted=true is not yet supported for export".to_string(),
        ));
    }
    let records = if let Some(q) = params.query.as_ref().filter(|q| !q.trim().is_empty()) {
        repo.search(q).await?
    } else {
        // Defence-in-depth: clamp again here so the listing path is bounded
        // even if a caller reaches it via an unclamped param (SEC-B2).
        repo.list_active(clamp_export_limit(params.limit), params.offset)
            .await?
    };
    let records = apply_masking(records, params.masking_profile);
    let rows = u64::try_from(records.len()).unwrap_or(u64::MAX);
    let bytes = match params.format {
        BulkFormat::Jsonl => jsonl::encode(&records)?,
        BulkFormat::Csv => csv::encode(&records)?,
    };
    Ok((bytes, rows))
}

/// DB-gated (`#[ignore]`) tests for the import/export pipeline. They
/// require a migrated `PostgreSQL` via `DATABASE_URL` and are skipped by a
/// bare `cargo test`; run with
/// `DATABASE_URL=… cargo test --lib -- --ignored`. They MUST compile
/// under a bare `cargo test --lib`.
#[cfg(test)]
mod db_tests {
    use super::{
        BulkFormat, ExportParams, ImportParams, MaskingProfile, process_export_job,
        process_import_job,
    };
    use crate::bulk::{csv, jsonl};
    use crate::config::Config;
    use crate::db::{PersonRepository, SeaOrmPersonRepository};
    use crate::matching::ProbabilisticMatcher;
    use crate::models::{Gender, HumanName, Identifier, IdentifierType, Person};
    use crate::search::SearchEngine;

    async fn connect() -> sea_orm::DatabaseConnection {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for DB tests");
        sea_orm::Database::connect(&url)
            .await
            .expect("connect to DATABASE_URL")
    }

    fn search_engine() -> (tempfile::TempDir, SearchEngine) {
        let dir = tempfile::tempdir().unwrap();
        let engine = SearchEngine::new(dir.path()).unwrap();
        (dir, engine)
    }

    fn matcher() -> ProbabilisticMatcher {
        ProbabilisticMatcher::new(Config::default().matching)
    }

    fn person(family: &str) -> Person {
        Person::new(
            HumanName {
                use_type: None,
                family: family.to_string(),
                given: vec!["Test".to_string()],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Unknown,
        )
    }

    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn import_creates_then_upserts_idempotently_with_error_report() {
        let db = connect().await;
        let repo = SeaOrmPersonRepository::new(db.clone());
        let (_dir, search) = search_engine();

        // One record keyed by a unique SSN, one keyed by pid, and one
        // invalid record (blank family name) that must land in the report.
        let unique_ssn = format!("SSN-{}", uuid::Uuid::new_v4());
        let mut p_ssn = person("KeyedBySsn");
        p_ssn.identifiers.push(Identifier::new(
            IdentifierType::SSN,
            "http://hl7.org/fhir/sid/us-ssn".to_string(),
            unique_ssn,
        ));
        let p_pid = person("KeyedByPid");
        let mut bad = person("Ignored");
        bad.name.family = String::new();

        let mut input = jsonl::encode(&[p_ssn.clone(), p_pid.clone()]).unwrap();
        input.extend_from_slice(jsonl::to_line(&bad).unwrap().as_bytes());
        input.push(b'\n');

        // First run: two creates, one error.
        let first = process_import_job(
            &db,
            &repo,
            &search,
            &matcher(),
            &input,
            BulkFormat::Jsonl,
            &ImportParams::default(),
        )
        .await
        .unwrap();
        assert_eq!(first.rows_total, 3, "three record rows");
        assert_eq!(
            first.rows_created, 2,
            "two new records; errors: {:?}",
            first.errors
        );
        assert_eq!(first.rows_upserted, 0);
        assert_eq!(first.rows_errored, 1, "one invalid row");
        assert_eq!(first.errors.len(), 1);
        assert_eq!(first.errors[0].row_number, 3);
        assert_eq!(first.errors[0].code, "validation");
        assert_eq!(
            first.rows_total,
            first.rows_created + first.rows_upserted + first.rows_errored
        );

        // Re-run the identical file: the two valid rows upsert in place.
        let second = process_import_job(
            &db,
            &repo,
            &search,
            &matcher(),
            &input,
            BulkFormat::Jsonl,
            &ImportParams::default(),
        )
        .await
        .unwrap();
        assert_eq!(second.rows_created, 0, "re-import creates nothing");
        assert_eq!(second.rows_upserted, 2, "re-import upserts both");
        assert_eq!(second.rows_errored, 1);
    }

    /// SEC-B3: two concurrent imports of the **same** stable key must
    /// produce exactly one record — the advisory lock serialises the
    /// find→create/update so the second importer upserts the first's row
    /// instead of racing it into a duplicate.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn concurrent_imports_of_one_stable_key_yield_one_record() {
        use crate::db::models::person_identifiers;
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

        let db = connect().await;
        let system = "http://hl7.org/fhir/sid/us-ssn".to_string();
        let value = format!("SSN-{}", uuid::Uuid::new_v4());

        let mut p = person("Concurrent");
        p.identifiers.push(Identifier::new(
            IdentifierType::SSN,
            system.clone(),
            value.clone(),
        ));
        let input = jsonl::encode(std::slice::from_ref(&p)).unwrap();

        // Two importers racing the identical single-row file on a shared pool.
        let spawn_import = |db: sea_orm::DatabaseConnection, input: Vec<u8>| {
            tokio::spawn(async move {
                let repo = SeaOrmPersonRepository::new(db.clone());
                let dir = tempfile::tempdir().unwrap();
                let search = SearchEngine::new(dir.path()).unwrap();
                let out = process_import_job(
                    &db,
                    &repo,
                    &search,
                    &matcher(),
                    &input,
                    BulkFormat::Jsonl,
                    &ImportParams::default(),
                )
                .await
                .unwrap();
                // Keep the temp index alive until the job finishes.
                drop(dir);
                (out.rows_created, out.rows_upserted, out.rows_errored)
            })
        };
        let h1 = spawn_import(db.clone(), input.clone());
        let h2 = spawn_import(db.clone(), input.clone());
        let (a, b) = tokio::join!(h1, h2);
        let (c1, u1, e1) = a.unwrap();
        let (c2, u2, e2) = b.unwrap();

        assert_eq!(e1 + e2, 0, "no row errors");
        assert_eq!(
            c1 + c2,
            1,
            "exactly one importer created the record (got {c1} + {c2})"
        );
        assert_eq!(
            u1 + u2,
            1,
            "the other importer upserted it (got {u1} + {u2})"
        );

        // Ground truth: exactly one distinct person owns that identifier.
        let rows = person_identifiers::Entity::find()
            .filter(person_identifiers::Column::System.eq(system))
            .filter(person_identifiers::Column::Value.eq(value))
            .all(&db)
            .await
            .unwrap();
        let distinct: std::collections::HashSet<_> = rows.iter().map(|r| r.person_id).collect();
        assert_eq!(
            distinct.len(),
            1,
            "exactly one person may own the stable-key identifier"
        );
    }

    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn dry_run_commits_nothing() {
        let db = connect().await;
        let repo = SeaOrmPersonRepository::new(db.clone());
        let (_dir, search) = search_engine();

        let p = person("DryRun");
        let input = jsonl::encode(std::slice::from_ref(&p)).unwrap();
        let outcome = process_import_job(
            &db,
            &repo,
            &search,
            &matcher(),
            &input,
            BulkFormat::Jsonl,
            &ImportParams { dry_run: true },
        )
        .await
        .unwrap();
        assert_eq!(outcome.rows_created, 1, "classified as create");

        assert!(
            repo.get_by_id(&p.id).await.unwrap().is_none(),
            "dry-run must not persist the record"
        );
    }

    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn export_round_trips_through_jsonl() {
        let db = connect().await;
        let repo = SeaOrmPersonRepository::new(db.clone());

        let created = repo.create(&person("Exported")).await.unwrap();

        // Default profile is Masked, so use Full here to round-trip the
        // record unchanged.
        let (bytes, rows) = process_export_job(
            &repo,
            &ExportParams {
                query: Some("Exported".to_string()),
                masking_profile: MaskingProfile::Full,
                ..ExportParams::default()
            },
        )
        .await
        .unwrap();

        let lines = jsonl::split_lines(&bytes).unwrap();
        assert!(!lines.is_empty(), "export produced at least one line");
        assert_eq!(
            rows,
            u64::try_from(lines.len()).unwrap(),
            "returned row count matches the encoded lines"
        );
        let parsed: Vec<Person> = lines
            .iter()
            .map(|l| jsonl::parse_line(l).unwrap())
            .collect();
        assert!(
            parsed.iter().any(|p| p.id == created.id),
            "the created record round-trips through the export"
        );
    }

    /// A **default** (masked) export redacts a sensitive field, and the
    /// worker's audit path writes an export audit row (§8). A **full**
    /// export (privileged) leaves the field intact.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn export_masks_by_default_and_full_is_unmasked_and_audited() {
        use crate::db::{AuditContext, AuditLogRepository};
        use crate::models::{Identifier, IdentifierType};

        let db = connect().await;
        let repo = SeaOrmPersonRepository::new(db.clone());
        let audit = AuditLogRepository::new(db.clone());

        let mut p = person("MaskedExport");
        p.tax_id = Some("123-45-6789".to_string());
        // The identifier value must be unique per run: `person_identifiers`
        // carries `UNIQUE (system, value)`, so a fixed value made this test
        // pass once and then fail on every later run against the same
        // database. The masking assertion is about `tax_id`, which stays
        // fixed; the identifier is incidental to it.
        p.identifiers.push(Identifier::new(
            IdentifierType::SSN,
            "http://hl7.org/fhir/sid/us-ssn".to_string(),
            format!("SSN-{}", uuid::Uuid::new_v4()),
        ));
        let created = repo.create(&p).await.unwrap();

        let find = |bytes: &[u8], id| -> Person {
            jsonl::split_lines(bytes)
                .unwrap()
                .iter()
                .map(|l| jsonl::parse_line(l).unwrap())
                .find(|x: &Person| x.id == id)
                .expect("exported record present")
        };

        // Default (Masked): the tax id is redacted.
        let (masked_bytes, masked_rows) = process_export_job(
            &repo,
            &ExportParams {
                query: Some("MaskedExport".to_string()),
                ..ExportParams::default()
            },
        )
        .await
        .unwrap();
        assert!(masked_rows >= 1);
        assert_eq!(
            find(&masked_bytes, created.id).tax_id.as_deref(),
            Some("***-**-6789"),
            "default export masks the tax id"
        );

        // Full (privileged): the tax id is intact.
        let (full_bytes, _) = process_export_job(
            &repo,
            &ExportParams {
                query: Some("MaskedExport".to_string()),
                masking_profile: MaskingProfile::Full,
                ..ExportParams::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(
            find(&full_bytes, created.id).tax_id.as_deref(),
            Some("123-45-6789"),
            "full export leaves the tax id unmasked"
        );

        // The per-export audit row is written even for a query export.
        let job_id = uuid::Uuid::new_v4();
        let details = serde_json::json!({
            "kind": "export", "format": "jsonl", "masking_profile": "masked",
            "include_soft_deleted": false, "rows_total": masked_rows,
        });
        audit
            .log_export(
                "PersonBulkExport",
                job_id,
                details,
                &AuditContext::default(),
            )
            .await
            .unwrap();
        let logs = audit
            .get_logs_for_entity("PersonBulkExport", job_id, 10)
            .await
            .unwrap();
        assert_eq!(logs.len(), 1, "one export audit row");
        assert_eq!(logs[0].action, "EXPORT");
    }

    /// `include_soft_deleted=true` is rejected as not-yet-supported rather
    /// than leaking or silently ignoring the flag (§8).
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn export_rejects_include_soft_deleted() {
        let db = connect().await;
        let repo = SeaOrmPersonRepository::new(db.clone());
        let err = process_export_job(
            &repo,
            &ExportParams {
                include_soft_deleted: true,
                ..ExportParams::default()
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(err, crate::Error::Validation(_)));
    }

    /// A keyless row (no strong identifier, no `tax_id`, no explicit `id`)
    /// whose demographics closely match an existing record is **still
    /// created** (a bulk load must never silently drop legitimate data)
    /// **and** queued in the stored review queue with `provenance =
    /// "import"`, so an operator sees it flagged.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn keyless_row_with_a_likely_duplicate_creates_and_queues_for_review() {
        use crate::db::review_queue;
        use chrono::NaiveDate;

        let db = connect().await;
        let repo = SeaOrmPersonRepository::new(db.clone());
        let (_dir, search) = search_engine();

        // A unique family name (per test run) so this run's search-blocking
        // candidates are exactly the records this test creates. Kept under
        // Tantivy's default 40-byte token cutoff (`RemoveLongFilter`) — a
        // longer token is silently dropped at index time, which would make
        // this row unfindable via blocking regardless of matcher logic.
        let family = format!(
            "KeylessDup{}",
            &uuid::Uuid::new_v4().simple().to_string()[..8]
        );
        let mut existing = person(&family);
        existing.birth_date = Some(NaiveDate::from_ymd_opt(1980, 6, 15).unwrap());
        let existing = repo.create(&existing).await.unwrap();
        search.index_person(&existing).unwrap();

        // The keyless row: same family name + birth date + given name, but
        // no id, no identifiers, no tax_id — built by stripping `id` from
        // an otherwise-identical encoded person rather than hand-writing
        // JSON, so the wire shape can't drift from `Person`'s real fields.
        let mut incoming = person(&family);
        incoming.birth_date = existing.birth_date;
        let mut value = serde_json::to_value(&incoming).unwrap();
        value.as_object_mut().unwrap().remove("id");
        let line = serde_json::to_string(&value).unwrap();
        let input = format!("{line}\n").into_bytes();

        let outcome = process_import_job(
            &db,
            &repo,
            &search,
            &matcher(),
            &input,
            BulkFormat::Jsonl,
            &ImportParams::default(),
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
            .find(|r| r.record_id_a == existing.id || r.record_id_b == existing.id)
            .expect("a pending pair references the existing record");
        assert_eq!(pair.provenance, "import");
        assert_eq!(pair.detection_method, "import_duplicate_detection");
        assert!(pair.match_score >= crate::bulk::IMPORT_REVIEW_THRESHOLD);
    }

    /// The CSV import path: a header + one keyed row (via `tax_id`)
    /// creates, matching the JSONL path's semantics with a different wire
    /// format.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn csv_import_creates_a_keyed_row() {
        let db = connect().await;
        let repo = SeaOrmPersonRepository::new(db.clone());
        let (_dir, search) = search_engine();

        let mut p = person("CsvImported");
        p.tax_id = Some(format!("TAX-{}", uuid::Uuid::new_v4()));
        let input = csv::encode(std::slice::from_ref(&p)).unwrap();

        let outcome = process_import_job(
            &db,
            &repo,
            &search,
            &matcher(),
            &input,
            BulkFormat::Csv,
            &ImportParams::default(),
        )
        .await
        .unwrap();
        assert_eq!(outcome.rows_total, 1);
        assert_eq!(outcome.rows_created, 1, "errors: {:?}", outcome.errors);

        let saved = repo.get_by_id(&p.id).await.unwrap().expect("persisted");
        assert_eq!(saved.tax_id, p.tax_id);
    }

    /// The CSV export path round-trips a created record through
    /// `process_export_job` with `format: BulkFormat::Csv`.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn csv_export_round_trips() {
        let db = connect().await;
        let repo = SeaOrmPersonRepository::new(db.clone());

        let created = repo.create(&person("CsvExported")).await.unwrap();

        let (bytes, rows) = process_export_job(
            &repo,
            &ExportParams {
                query: Some("CsvExported".to_string()),
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
                .any(|(_, r)| r.as_ref().is_ok_and(|p| p.id == created.id)),
            "the created record round-trips through the CSV export"
        );
    }
}

/// DB-free unit tests for the pure export helpers (masking + the
/// privileged-path gate decision).
#[cfg(test)]
mod unit_tests {
    use super::{ExportParams, MaskingProfile, apply_masking, export_requires_elevation};
    use crate::models::{Gender, HumanName, Person};

    fn person_with_tax(tax: &str) -> Person {
        let mut p = Person::new(
            HumanName {
                use_type: None,
                family: "Doe".to_string(),
                given: vec!["Jane".to_string()],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Female,
        );
        p.tax_id = Some(tax.to_string());
        p
    }

    /// `Masked` (the default) redacts the tax id; `Full` leaves it intact.
    #[test]
    fn masking_applies_for_masked_and_skips_for_full() {
        let people = vec![person_with_tax("123-45-6789")];

        let masked = apply_masking(people.clone(), MaskingProfile::Masked);
        assert_eq!(
            masked[0].tax_id.as_deref(),
            Some("***-**-6789"),
            "Masked profile redacts the tax id"
        );

        let full = apply_masking(people, MaskingProfile::Full);
        assert_eq!(
            full[0].tax_id.as_deref(),
            Some("123-45-6789"),
            "Full profile leaves the tax id intact"
        );

        // The default profile is Masked.
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

    /// SEC-B3: the advisory-lock key string distinguishes stable-key kinds
    /// and keeps `system`/`value` boundaries unambiguous, so two different
    /// keys cannot collide onto the same lock.
    #[test]
    fn stable_key_lock_string_is_distinct_per_key() {
        use super::stable_key_lock_string;
        use crate::bulk::stable_key::StableKey;
        use uuid::Uuid;

        let pid = Uuid::from_u128(1);
        let by_pid = stable_key_lock_string(&StableKey::Pid(pid));
        assert!(by_pid.starts_with("person-pid:"));

        let a = stable_key_lock_string(&StableKey::Identifier {
            system: "sys".into(),
            value: "abc".into(),
        });
        let b = stable_key_lock_string(&StableKey::Identifier {
            system: "sys".into(),
            value: "abd".into(),
        });
        assert_ne!(a, b, "different values must yield different lock keys");
        // The length prefix prevents ("sy","stemabc") aliasing ("sys","temabc").
        let split_a = stable_key_lock_string(&StableKey::Identifier {
            system: "sys".into(),
            value: "temabc".into(),
        });
        let split_b = stable_key_lock_string(&StableKey::Identifier {
            system: "sy".into(),
            value: "stemabc".into(),
        });
        assert_ne!(split_a, split_b, "boundary must be unambiguous");

        // The key is bound as a Postgres `text` parameter, which cannot
        // carry a NUL. A key containing one made every identifier-keyed
        // import row fail; pin that it never comes back.
        for key in [&by_pid, &a, &b, &split_a, &split_b] {
            assert!(
                !key.contains('\0'),
                "lock key must be valid Postgres text: {key:?}"
            );
        }
    }

    /// SEC-B2: a caller-supplied export `limit` is clamped to the ceiling,
    /// so an export can never be asked to buffer an unbounded result set.
    #[test]
    fn export_limit_is_clamped_to_the_ceiling() {
        use super::clamp_export_limit;
        use crate::bulk::MAX_EXPORT_ROWS;
        assert_eq!(clamp_export_limit(10), 10, "under the cap is unchanged");
        assert_eq!(clamp_export_limit(MAX_EXPORT_ROWS), MAX_EXPORT_ROWS);
        assert_eq!(
            clamp_export_limit(u64::MAX),
            MAX_EXPORT_ROWS,
            "an absurd limit is clamped to the ceiling"
        );
    }
}
