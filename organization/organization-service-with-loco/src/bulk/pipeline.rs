//! The import/export pipeline — the testable core of the bulk worker
//! (`agents/share/bulk-import-export.md` §6, §7; crate spec §10.7).
//!
//! [`process_import_job`] and [`process_export_job`] carry the whole
//! per-row / per-job logic and take a plain `&DatabaseConnection`, so the
//! loco background worker ([`crate::bulk::worker`]) is a thin adapter
//! and the logic is exercised directly by request-level tests without
//! going through the multipart/JSON handler layer.
//!
//! **Import** (per row): parse → validate (the same
//! [`crate::validation::problems`] single-create uses, so the same
//! `422`-shaped reasons) → resolve the stable key (§10.7: LEI → DUNS →
//! explicit `pid`). A row carrying a real key **upserts in place** when
//! it matches an existing record (idempotent re-import), else
//! **creates**. A **keyless** row ([`stable_key::is_keyless`]) instead
//! runs the same search-blocking + matcher duplicate detection `POST
//! /check-duplicates` uses: a likely duplicate (score ≥
//! [`crate::bulk::IMPORT_REVIEW_THRESHOLD`]) still **creates** the row (a
//! bulk load must never silently drop legitimate data) but also queues a
//! `provenance = "import"` pair in the stored
//! [`crate::models::review_queue`], so an operator sees it flagged rather
//! than discovering it only on a later batch scan; no candidate above
//! threshold ⇒ a plain create, same as a keyed row with no match. Invalid
//! rows are skipped and recorded in the error report; they never abort
//! the load. Every written row goes through
//! [`crate::streaming::create_and_emit`] /
//! [`crate::streaming::update_and_emit`], which emit the normal
//! `created`/`updated` event, write the normal audit row, and update the
//! search index — a bulk-imported organization is indistinguishable,
//! after the fact, from one created interactively.
//!
//! Both **JSONL** ([`jsonl`], the lossless reference) and **CSV**
//! ([`csv`], the operator/spreadsheet format) are accepted on import,
//! selected by the job's declared [`BulkFormat`].
//!
//! **Export**: honour the organization list/search filter, streaming
//! matching records to a JSONL or CSV buffer per the job's
//! [`BulkFormat`]. By default (the [`MaskingProfile::Masked`] profile)
//! every record is run through [`crate::privacy::mask_organization`]
//! before encoding, so a bulk export never reveals more than the masked
//! read view (§8); the privileged [`MaskingProfile::Full`] profile
//! leaves records unmasked and is gated at the handler.
//!
//! Deferred (noted, not built): a real soft-deleted-record export query
//! (`include_soft_deleted = true` is rejected — at the handler, before a
//! job is even created — as not-yet-supported rather than leaking or
//! ignoring it).

use sea_orm::{DatabaseBackend, DatabaseConnection, FromQueryResult, Statement};
use uuid::Uuid;

use loco_rs::Error;
use organization_matcher::{Confidence, MatchConfig, MatchResult, MatchingEngine, Organization};

use crate::controllers::organizations::CHECK_DUPLICATES_CANDIDATE_LIMIT;
use crate::models::organizations::Model as OrgModel;
use crate::models::review_queue::{self, NewReviewItem};
use crate::privacy::mask_organization;
use crate::streaming;

use super::error_report::ErrorRow;
use super::stable_key::{StableKey, resolve_stable_key};
use super::{BulkFormat, MaskingProfile, csv, jsonl, stable_key};

/// Parameters for an import run.
#[derive(Debug, Clone, Default)]
pub struct ImportParams {
    /// Validate + classify but commit nothing (§4). Counts reflect the
    /// would-be result; no records are written.
    pub dry_run: bool,
    /// The acting user pid (bearer `sub`), threaded into every emitted
    /// event/audit row, when known.
    pub actor: Option<String>,
}

/// The reconciled outcome of an import run. Invariant:
/// `rows_total == rows_created + rows_upserted + rows_errored`.
///
/// `rows_to_review` is **not** a fourth exclusive bucket — a keyless row
/// with a likely duplicate is still created (never silently dropped) and
/// *also* counted here, so `rows_to_review <= rows_created`.
#[derive(Debug, Clone, Default, PartialEq)]
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

/// Parameters for an export run — the organization search filter (§4)
/// plus the §8 privacy controls.
#[derive(Debug, Clone)]
pub struct ExportParams {
    /// Optional full-text search query; when set, uses the Tantivy index
    /// (exact mode), else pages active records via `list_paged`.
    pub query: Option<String>,
    /// Max records for the unfiltered listing path.
    pub limit: u64,
    /// Offset for the unfiltered listing path.
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
/// (SEC-B2), so an export can never be asked to buffer an unbounded result
/// set. Pure, so the worker's param mapping and its tests share one
/// definition of the ceiling.
#[must_use]
pub fn clamp_export_limit(requested: u64) -> u64 {
    requested.min(crate::bulk::MAX_EXPORT_ROWS)
}

/// Apply the export masking profile to a batch of `(pid, organization)`
/// rows (§8): the default [`MaskingProfile::Masked`] runs each record
/// through [`mask_organization`]; [`MaskingProfile::Full`] returns them
/// unchanged. Pure and DB-free so it is unit-testable without a database.
#[must_use]
pub fn apply_masking(
    rows: Vec<(Uuid, Organization)>,
    masking_profile: MaskingProfile,
) -> Vec<(Uuid, Organization)> {
    match masking_profile {
        MaskingProfile::Full => rows,
        MaskingProfile::Masked => rows
            .into_iter()
            .map(|(pid, org)| (pid, mask_organization(&org)))
            .collect(),
    }
}

/// Resolve the existing record (if any) that `key` points at, so the
/// caller can decide create-vs-upsert.
async fn find_existing(
    db: &DatabaseConnection,
    key: &StableKey,
) -> loco_rs::Result<Option<OrgModel>> {
    match key {
        StableKey::Pid(pid) => match OrgModel::find_by_pid(db, &pid.to_string()).await {
            Ok(model) => Ok(Some(model)),
            Err(loco_rs::model::ModelError::EntityNotFound) => Ok(None),
            Err(e) => Err(e.into()),
        },
        StableKey::Identifier { scheme, value } => find_by_identifier(db, scheme, value)
            .await
            .map_err(Into::into),
    }
}

/// Look up the non-deleted organization owning a `(scheme, value)`
/// identifier, if any, via a Postgres JSONB containment query over
/// `data->'identifiers'`. Returns the first match.
///
/// `organization_matcher::OrgIdentifier` serializes to exactly
/// `{"scheme": …, "value": …}`, so the array-containment operator (`@>`)
/// matching one such object against the stored `identifiers` array is an
/// exact element match, not a partial/subset one — there is no scheme
/// this crate stores that the needle object could accidentally
/// under-match.
async fn find_by_identifier(
    db: &DatabaseConnection,
    scheme: &organization_matcher::IdentifierScheme,
    value: &str,
) -> loco_rs::model::ModelResult<Option<OrgModel>> {
    let scheme_json =
        serde_json::to_value(scheme).map_err(|e| loco_rs::model::ModelError::Any(e.into()))?;
    let needle = serde_json::json!([{ "scheme": scheme_json, "value": value }]);
    let stmt = Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT * FROM organizations \
         WHERE deleted_at IS NULL AND data -> 'identifiers' @> $1::jsonb \
         LIMIT 1",
        [sea_orm::Value::Json(Some(Box::new(needle)))],
    );
    Ok(OrgModel::find_by_statement(stmt).one(db).await?)
}

/// The idempotent per-row upsert for a **keyed** row: find the existing
/// record by stable key, then update or create it via
/// [`streaming::update_and_emit`] / [`streaming::create_and_emit`] —
/// exactly the interactive handlers' own write path, so a bulk-imported
/// row gets the same event/audit/index side effects as one created
/// interactively. Returns `(saved, was_upsert)`.
///
/// **Known limitation (not SEC-B3-hardened).** This is a plain
/// find-then-write, not wrapped in a stable-key advisory lock the way
/// person's bulk pipeline is: `streaming::create_and_emit` /
/// `update_and_emit` are hard-coded to `&DatabaseConnection` (they open
/// their *own* transaction internally under the `outbox` transport), so
/// a lock held on a separate guard transaction would hold one pooled
/// connection while these need a second — under a small pool (this
/// crate's own `config/test.yaml` runs `max_connections: 1`) that
/// deadlocks every single import, not just a concurrent one. Two
/// importers racing the *same* stable key in the same instant can
/// therefore both miss in [`find_existing`] and both create a row. This
/// is judged an acceptable, documented gap for BLK-5 (operator-driven
/// bulk loads are not typically run concurrently against the same
/// file/key); closing it properly needs a `ConnectionTrait`-generic
/// `streaming::create_and_emit`/`update_and_emit`, which is a
/// `src/streaming.rs`-wide change out of this task's scope.
async fn import_upsert(
    db: &DatabaseConnection,
    key: &StableKey,
    org: &Organization,
    actor: Option<&str>,
) -> loco_rs::Result<(OrgModel, bool)> {
    let existing = find_existing(db, key).await?;
    if let Some(existing) = existing {
        Ok((
            streaming::update_and_emit(db, existing, org, actor).await?,
            true,
        ))
    } else {
        Ok((streaming::create_and_emit(db, org, actor).await?, false))
    }
}

/// One decoded import row, format-agnostic — the shape
/// [`decode_import_rows`] normalises both [`jsonl`] and [`csv`] down to.
///
/// There is no separate "had an explicit pid" flag here (unlike
/// person's `Person::id`, which defaults to a fresh UUID on
/// deserialize): `pid` is a genuine `Option<Uuid>`, so `pid.is_some()`
/// already answers that question exactly — see [`super::columns`]'s
/// module docs.
struct ImportRow {
    /// The row's own pid, when given.
    pid: Option<Uuid>,
    /// The parsed organization, or the per-row parse error message (§7).
    parsed: std::result::Result<Organization, String>,
}

/// Decode `input` per `format` into per-row [`ImportRow`]s, enforcing the
/// SEC-B2 row cap uniformly regardless of format
/// (`jsonl::split_lines_capped` does this inline for JSONL; CSV is
/// checked here since `csv::decode` has no cap of its own).
fn decode_import_rows(input: &[u8], format: BulkFormat) -> loco_rs::Result<Vec<ImportRow>> {
    match format {
        BulkFormat::Jsonl => Ok(
            jsonl::split_lines_capped(input, crate::bulk::MAX_IMPORT_ROWS)?
                .into_iter()
                .map(|line| match jsonl::parse_line(&line) {
                    Ok((_had_explicit_pid, pid, org)) => ImportRow {
                        pid,
                        parsed: Ok(org),
                    },
                    Err(e) => ImportRow {
                        pid: None,
                        parsed: Err(e),
                    },
                })
                .collect(),
        ),
        // CSV and TSV share the codec; the format supplies the byte.
        BulkFormat::Csv | BulkFormat::Tsv => {
            let decoded = csv::decode(input, format.delimiter().unwrap_or(b','))?;
            if decoded.len() > crate::bulk::MAX_IMPORT_ROWS {
                return Err(Error::Message(format!(
                    "bulk import exceeds the row cap: {} rows > {}",
                    decoded.len(),
                    crate::bulk::MAX_IMPORT_ROWS
                )));
            }
            Ok(decoded
                .into_iter()
                .map(|row| match row {
                    Ok((_had_explicit_pid, pid, org)) => ImportRow {
                        pid,
                        parsed: Ok(org),
                    },
                    Err(e) => ImportRow {
                        pid: None,
                        parsed: Err(e),
                    },
                })
                .collect())
        }
    }
}

/// The confidence-band label for a match result, matching the
/// classification `POST /check-duplicates` and `POST /deduplicate`
/// already use (`organization_matcher::Confidence`, `Debug`-lowercased).
fn match_quality_label(confidence: Confidence) -> String {
    format!("{confidence:?}").to_lowercase()
}

/// Find the best duplicate candidate for a **keyless** row, above
/// [`crate::bulk::IMPORT_REVIEW_THRESHOLD`], via the same
/// search-blocking-and-matcher path `POST /check-duplicates` uses.
/// `None` when the search index is unavailable (logged, not fatal — a
/// bulk load must never silently drop legitimate data just because the
/// index is degraded), the blocking search finds nothing, or nothing
/// clears the threshold.
async fn find_keyless_duplicate(
    db: &DatabaseConnection,
    org: &Organization,
) -> Option<(OrgModel, MatchResult)> {
    let Some(engine) = crate::search::engine() else {
        tracing::warn!(
            "bulk import: search index unavailable; keyless-row duplicate detection skipped"
        );
        return None;
    };
    let hits = match engine.candidates(org, CHECK_DUPLICATES_CANDIDATE_LIMIT) {
        Ok(hits) => hits,
        Err(e) => {
            tracing::warn!("bulk import: candidate search failed: {e}");
            return None;
        }
    };
    if hits.is_empty() {
        return None;
    }
    let pids: Vec<Uuid> = hits
        .iter()
        .filter_map(|s| Uuid::parse_str(s).ok())
        .collect();
    let rows = match OrgModel::find_by_pids(db, &pids).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!("bulk import: candidate lookup failed: {e}");
            return None;
        }
    };

    let matcher = MatchingEngine::new(MatchConfig::default());
    let mut best: Option<(OrgModel, MatchResult)> = None;
    for row in rows {
        let Ok(candidate) = row.to_org() else {
            continue;
        };
        let result = matcher.match_organizations(org, &candidate);
        if result.score < crate::bulk::IMPORT_REVIEW_THRESHOLD {
            continue;
        }
        if best.as_ref().is_none_or(|(_, b)| result.score > b.score) {
            best = Some((row, result));
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
    org: &Organization,
    duplicate: &OrgModel,
    result: &MatchResult,
    actor: Option<&str>,
) -> loco_rs::Result<OrgModel> {
    let saved = streaming::create_and_emit(db, org, actor).await?;
    let item = NewReviewItem {
        record_id_a: saved.pid,
        record_id_b: duplicate.pid,
        match_score: result.score,
        match_quality: match_quality_label(result.confidence),
        detection_method: "import_duplicate_detection".to_string(),
        score_breakdown: serde_json::to_value(&result.breakdown).ok(),
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

/// Run a full import over an `input` byte buffer in the given `format`,
/// returning the reconciled [`ImportOutcome`] (including the per-row
/// error report).
///
/// Each successfully written row is persisted through
/// [`streaming::create_and_emit`] / [`streaming::update_and_emit`],
/// which emit the normal event + audit row and update the search index;
/// on `params.dry_run`, rows are parsed, validated, and classified but
/// nothing is written (including no review-queue row for a keyless
/// duplicate).
///
/// # Errors
///
/// Returns an error only for a whole-job failure (e.g. non-UTF-8 input,
/// an unreadable CSV header, or the SEC-B2 row cap); per-row failures
/// are captured in [`ImportOutcome::errors`], not returned.
pub async fn process_import_job(
    db: &DatabaseConnection,
    input: &[u8],
    format: BulkFormat,
    params: &ImportParams,
) -> loco_rs::Result<ImportOutcome> {
    let rows = decode_import_rows(input, format)?;
    let mut outcome = ImportOutcome::default();
    let actor = params.actor.as_deref();

    for (idx, row) in rows.into_iter().enumerate() {
        let row_number = idx + 1;
        outcome.rows_total += 1;

        let org = match row.parsed {
            Ok(o) => o,
            Err(e) => {
                outcome.errors.push(ErrorRow::parse(row_number, e));
                outcome.rows_errored += 1;
                continue;
            }
        };

        let problems = crate::validation::problems(&org);
        if !problems.is_empty() {
            for p in problems {
                outcome.errors.push(ErrorRow::validation(row_number, "", p));
            }
            outcome.rows_errored += 1;
            continue;
        }

        let Some(key) = resolve_stable_key(row.pid, &org) else {
            // §6: a keyless row cannot idempotently upsert, so it runs
            // through duplicate detection instead of a blind create.
            debug_assert!(stable_key::is_keyless(row.pid, &org));
            let duplicate = find_keyless_duplicate(db, &org).await;
            if let Some((dup_row, result)) = duplicate {
                if params.dry_run {
                    outcome.rows_created += 1;
                    outcome.rows_to_review += 1;
                    continue;
                }
                match create_and_queue_for_review(db, &org, &dup_row, &result, actor).await {
                    Ok(_saved) => {
                        outcome.rows_created += 1;
                        outcome.rows_to_review += 1;
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

            if params.dry_run {
                outcome.rows_created += 1;
                continue;
            }
            match streaming::create_and_emit(db, &org, actor).await {
                Ok(_saved) => outcome.rows_created += 1,
                Err(e) => {
                    outcome
                        .errors
                        .push(ErrorRow::database(row_number, e.to_string()));
                    outcome.rows_errored += 1;
                }
            }
            continue;
        };

        if params.dry_run {
            match find_existing(db, &key).await {
                Ok(Some(_)) => outcome.rows_upserted += 1,
                Ok(None) => outcome.rows_created += 1,
                Err(e) => {
                    outcome
                        .errors
                        .push(ErrorRow::database(row_number, e.to_string()));
                    outcome.rows_errored += 1;
                }
            }
            continue;
        }

        // Find + create/update by stable key (see `import_upsert`'s docs
        // for the known non-concurrent-safe limitation).
        match import_upsert(db, &key, &org, actor).await {
            Ok((_saved, was_upsert)) => {
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
/// Uses the Tantivy index (exact mode) when `params.query` is set, else
/// pages active records via `OrgModel::list_paged`. Every record is then
/// run through [`apply_masking`] per `params.masking_profile`, so the
/// default (`Masked`) export never reveals more than the masked read
/// view; `Full` leaves records unmasked (gated at the handler).
///
/// # Errors
///
/// Returns an error when `params.include_soft_deleted` is `true` — the
/// model layer cannot express a soft-deleted listing without a larger
/// change, so rather than silently leaking or ignoring the flag the
/// export is rejected as not-yet-supported (the handler additionally
/// rejects this before a job is ever created; this is the defence-in-depth
/// check for a direct pipeline caller or a hand-edited job row). Also
/// returns an error if the underlying query, search, or format encode
/// fails.
pub async fn process_export_job(
    db: &DatabaseConnection,
    params: &ExportParams,
) -> loco_rs::Result<(Vec<u8>, u64)> {
    if params.include_soft_deleted {
        return Err(Error::Message(
            "include_soft_deleted=true is not yet supported for export".to_string(),
        ));
    }

    let models: Vec<OrgModel> =
        if let Some(q) = params.query.as_ref().filter(|q| !q.trim().is_empty()) {
            let Some(engine) = crate::search::engine() else {
                return Err(Error::Message(
                    "the search index is unavailable, so a query export cannot run".to_string(),
                ));
            };
            let limit = usize::try_from(clamp_export_limit(params.limit)).unwrap_or(usize::MAX);
            let offset = usize::try_from(params.offset).unwrap_or(usize::MAX);
            let (pids, _total) =
                engine.search_page(q, crate::search::SearchMode::Exact, limit, offset)?;
            let pids: Vec<Uuid> = pids
                .iter()
                .filter_map(|s| Uuid::parse_str(s).ok())
                .collect();
            OrgModel::find_by_pids(db, &pids).await?
        } else {
            // Defence-in-depth: clamp again here so the listing path is
            // bounded even if a caller reaches it via an unclamped param
            // (SEC-B2).
            OrgModel::list_paged(db, clamp_export_limit(params.limit), params.offset).await?
        };

    let mut rows = Vec::with_capacity(models.len());
    for model in models {
        rows.push((model.pid, model.to_org()?));
    }
    let rows = apply_masking(rows, params.masking_profile);
    let count = u64::try_from(rows.len()).unwrap_or(u64::MAX);

    let export_rows: Vec<(Option<Uuid>, Organization)> = rows
        .into_iter()
        .map(|(pid, org)| (Some(pid), org))
        .collect();
    let bytes = match params.format {
        BulkFormat::Jsonl => jsonl::encode(&export_rows)?,
        BulkFormat::Csv | BulkFormat::Tsv => {
            csv::encode(&export_rows, params.format.delimiter().unwrap_or(b','))?
        }
    };
    Ok((bytes, count))
}

/// DB-free unit tests for the pure helpers.
#[cfg(test)]
mod tests {
    use super::{ExportParams, MaskingProfile, apply_masking, export_requires_elevation};
    use organization_matcher::Organization;
    use uuid::Uuid;

    fn org_with_email(email: &str) -> Organization {
        Organization {
            email: Some(email.to_string()),
            ..Organization::new("Acme")
        }
    }

    /// `Masked` (the default) redacts the email local part; `Full`
    /// leaves it intact.
    #[test]
    fn masking_applies_for_masked_and_skips_for_full() {
        let pid = Uuid::new_v4();
        let rows = vec![(pid, org_with_email("accounts@acme.example"))];

        let masked = apply_masking(rows.clone(), MaskingProfile::Masked);
        assert_eq!(
            masked[0].1.email.as_deref(),
            Some("a*******@acme.example"),
            "Masked profile redacts the email"
        );

        let full = apply_masking(rows, MaskingProfile::Full);
        assert_eq!(
            full[0].1.email.as_deref(),
            Some("accounts@acme.example"),
            "Full profile leaves the email intact"
        );

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
    /// ceiling, so an export can never be asked to buffer an unbounded
    /// result set.
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
