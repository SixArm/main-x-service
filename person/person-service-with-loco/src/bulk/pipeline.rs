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
//! (§10.1) → **upsert in place** when it matches an existing record
//! (idempotent re-import), else **create**. Invalid rows are skipped and
//! recorded in the error report; they never abort the load. Each written
//! row goes through the repository, which emits its normal event + audit.
//!
//! **Export**: honour the person list/search filter, streaming matching
//! records to a JSONL buffer.
//!
//! Deferred (noted, not built): keyless-row → duplicate-detection →
//! review-queue routing (a keyless row simply creates in step 1);
//! export masking profiles + `include_soft_deleted` gating.

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::Result;
use crate::db::PersonRepository;
use crate::db::models::person_identifiers;
use crate::models::Person;
use crate::search::SearchEngine;

use super::error_report::ErrorRow;
use super::jsonl;
use super::stable_key::{StableKey, resolve_stable_key};

/// Parameters for an import run.
#[derive(Debug, Clone, Default)]
pub struct ImportParams {
    /// Validate + classify but commit nothing (§4). Counts reflect the
    /// would-be result; no records are written.
    pub dry_run: bool,
}

/// The reconciled outcome of an import run. Invariant:
/// `rows_total == rows_created + rows_upserted + rows_errored`
/// (`rows_to_review` is always 0 in step 1 — routing is deferred).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportOutcome {
    /// Total non-blank record rows seen.
    pub rows_total: u64,
    /// Rows inserted as new records.
    pub rows_created: u64,
    /// Rows upserted onto an existing record.
    pub rows_upserted: u64,
    /// Rows routed to review (always 0 in step 1).
    pub rows_to_review: u64,
    /// Rows that failed parse/validation/persistence.
    pub rows_errored: u64,
    /// The per-row error report (§7).
    pub errors: Vec<ErrorRow>,
}

/// Parameters for an export run — the person list/search filter (§4).
#[derive(Debug, Clone)]
pub struct ExportParams {
    /// Optional family-name search query; when set, uses the repository's
    /// `search`, else pages active records via `list_active`.
    pub query: Option<String>,
    /// Max records for the unfiltered listing path.
    pub limit: u64,
    /// Offset for the unfiltered listing path.
    pub offset: u64,
}

impl Default for ExportParams {
    fn default() -> Self {
        Self {
            query: None,
            limit: 10_000,
            offset: 0,
        }
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

/// Run a full import over a JSONL byte buffer, returning the reconciled
/// [`ImportOutcome`] (including the per-row error report).
///
/// Each successfully written row is persisted through `repo`, which emits
/// its normal `created`/`updated` event and audit record; the search
/// index is updated best-effort. On `params.dry_run`, rows are parsed,
/// validated, and classified but nothing is written.
///
/// # Errors
///
/// Returns an error only for a whole-job failure (e.g. non-UTF-8 input);
/// per-row failures are captured in [`ImportOutcome::errors`], not
/// returned.
pub async fn process_import_job(
    db: &DatabaseConnection,
    repo: &dyn PersonRepository,
    search: &SearchEngine,
    input: &[u8],
    params: &ImportParams,
) -> Result<ImportOutcome> {
    let lines = jsonl::split_lines(input)?;
    let mut outcome = ImportOutcome::default();

    for (idx, line) in lines.iter().enumerate() {
        let row_number = idx + 1;
        outcome.rows_total += 1;

        // Parse (§7: a bad line is recorded, never fatal).
        let mut person = match jsonl::parse_line(line) {
            Ok(p) => p,
            Err(e) => {
                outcome
                    .errors
                    .push(ErrorRow::parse(row_number, e.to_string()));
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

        let existing = match find_existing(db, repo, &person).await {
            Ok(existing) => existing,
            Err(e) => {
                outcome
                    .errors
                    .push(ErrorRow::database(row_number, e.to_string()));
                outcome.rows_errored += 1;
                continue;
            }
        };

        if params.dry_run {
            if existing.is_some() {
                outcome.rows_upserted += 1;
            } else {
                outcome.rows_created += 1;
            }
            continue;
        }

        let (written, was_upsert) = if let Some(existing) = existing {
            // Upsert in place: keep the existing record's pid so the
            // stable key maps to one record across re-imports.
            person.id = existing.id;
            (repo.update(&person).await, true)
        } else {
            if person.id == Uuid::nil() {
                person.id = Uuid::new_v4();
            }
            (repo.create(&person).await, false)
        };

        match written {
            Ok(saved) => {
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

/// Run an export, returning the JSONL byte buffer of matching records.
///
/// Uses the repository's family-name `search` when `params.query` is set,
/// else pages active records via `list_active`. Masking profiles and
/// soft-deleted inclusion are deferred (step 3); this returns the normal
/// read view.
///
/// # Errors
///
/// Returns an error if the underlying repository query or JSONL encode
/// fails.
pub async fn process_export_job(
    repo: &dyn PersonRepository,
    params: &ExportParams,
) -> Result<Vec<u8>> {
    let records = if let Some(q) = params.query.as_ref().filter(|q| !q.trim().is_empty()) {
        repo.search(q).await?
    } else {
        repo.list_active(params.limit, params.offset).await?
    };
    jsonl::encode(&records)
}

/// DB-gated (`#[ignore]`) tests for the import/export pipeline. They
/// require a migrated `PostgreSQL` via `DATABASE_URL` and are skipped by a
/// bare `cargo test`; run with
/// `DATABASE_URL=… cargo test --lib -- --ignored`. They MUST compile
/// under a bare `cargo test --lib`.
#[cfg(test)]
mod db_tests {
    use super::{ExportParams, ImportParams, process_export_job, process_import_job};
    use crate::bulk::jsonl;
    use crate::db::{PersonRepository, SeaOrmPersonRepository};
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
        let first = process_import_job(&db, &repo, &search, &input, &ImportParams::default())
            .await
            .unwrap();
        assert_eq!(first.rows_total, 3, "three record rows");
        assert_eq!(first.rows_created, 2, "two new records");
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
        let second = process_import_job(&db, &repo, &search, &input, &ImportParams::default())
            .await
            .unwrap();
        assert_eq!(second.rows_created, 0, "re-import creates nothing");
        assert_eq!(second.rows_upserted, 2, "re-import upserts both");
        assert_eq!(second.rows_errored, 1);
    }

    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn dry_run_commits_nothing() {
        let db = connect().await;
        let repo = SeaOrmPersonRepository::new(db.clone());
        let (_dir, search) = search_engine();

        let p = person("DryRun");
        let input = jsonl::encode(std::slice::from_ref(&p)).unwrap();
        let outcome =
            process_import_job(&db, &repo, &search, &input, &ImportParams { dry_run: true })
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

        let bytes = process_export_job(
            &repo,
            &ExportParams {
                query: Some("Exported".to_string()),
                ..ExportParams::default()
            },
        )
        .await
        .unwrap();

        let lines = jsonl::split_lines(&bytes).unwrap();
        assert!(!lines.is_empty(), "export produced at least one line");
        let parsed: Vec<Person> = lines
            .iter()
            .map(|l| jsonl::parse_line(l).unwrap())
            .collect();
        assert!(
            parsed.iter().any(|p| p.id == created.id),
            "the created record round-trips through the export"
        );
    }
}
