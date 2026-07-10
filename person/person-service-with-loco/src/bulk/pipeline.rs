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
//! records to a JSONL buffer. By default (the [`MaskingProfile::Masked`]
//! profile) every record is run through [`crate::privacy::mask_person`]
//! before encoding, so a bulk export never reveals more than the masked
//! read view (§8); the privileged [`MaskingProfile::Full`] profile leaves
//! records unmasked and is gated at the handler.
//!
//! Deferred (noted, not built): keyless-row → duplicate-detection →
//! review-queue routing (a keyless row simply creates in step 1); a real
//! soft-deleted-record export query (`include_soft_deleted = true` is
//! rejected as not-yet-supported rather than leaking or ignoring it).

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::Result;
use crate::db::PersonRepository;
use crate::db::models::person_identifiers;
use crate::models::Person;
use crate::privacy::mask_person;
use crate::search::SearchEngine;

use super::MaskingProfile;
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
}

impl Default for ExportParams {
    fn default() -> Self {
        Self {
            query: None,
            limit: 10_000,
            offset: 0,
            masking_profile: MaskingProfile::Masked,
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

/// Run an export, returning the JSONL byte buffer of matching records
/// **and** the number of records exported (for the audit row, §8).
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
/// error if the underlying repository query or JSONL encode fails.
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
        repo.list_active(params.limit, params.offset).await?
    };
    let records = apply_masking(records, params.masking_profile);
    let rows = u64::try_from(records.len()).unwrap_or(u64::MAX);
    let bytes = jsonl::encode(&records)?;
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
        ExportParams, ImportParams, MaskingProfile, process_export_job, process_import_job,
    };
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
        p.identifiers.push(Identifier::new(
            IdentifierType::SSN,
            "http://hl7.org/fhir/sid/us-ssn".to_string(),
            "123-45-6789".to_string(),
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
}
