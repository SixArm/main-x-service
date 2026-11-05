//! Physical artifact deletion sweep (SEC-B4 follow-up).
//!
//! `bulk::handlers`'s `artifact_expired` gate already stops an expired
//! job's artifact *reference* from being handed out — the status
//! endpoint `404`s once `bulk_jobs.expires_at` has passed. It does not
//! delete the underlying bytes; the [`ArtifactStore`] keeps them
//! indefinitely. This module is the other half: [`sweep`] finds every
//! job past its retention deadline that has not yet been physically
//! swept, deletes its artifacts (`input_url` / `result_url` /
//! `error_report_url`) via [`ArtifactStore::delete`], and stamps
//! `bulk_jobs.artifact_deleted_at` so the same row is never processed
//! twice.
//!
//! Run via the `bulk_artifact_sweep` loco task
//! ([`crate::tasks::bulk_artifact_sweep`]) — see that module's doc for
//! how to invoke and schedule it. This crate has no in-process
//! periodic-timer convention (the `BulkJobWorker` is dispatched per job
//! via `perform_later`, never on a clock), so scheduling is external
//! (cron, a Kubernetes `CronJob`, …) invoking the task, matching how
//! this crate already treats other operator-triggered maintenance
//! (`integrity_resign`) rather than inventing a new in-process
//! scheduling primitive.

use time::OffsetDateTime;

use crate::Result;
use crate::bulk::store::ArtifactStore;
use crate::db::bulk_jobs::{self, Model};

/// Pure eligibility check: does a job need a physical-deletion sweep pass
/// as of `now`, given its retention deadline and whether it has already
/// been swept?
///
/// Intentionally mirrors (rather than shares code with) the retention
/// check in `bulk::handlers::artifact_expired` — that function gates
/// *handing out a reference*; this one gates *deleting the bytes*, and
/// keeping them as separate pure functions means a change to one can
/// never silently change the other's behaviour. Both use the same "at or
/// past the deadline" rule (`now >= expires_at`). A job with no deadline
/// (a legacy row predating `expires_at`) is never swept; a job already
/// swept (`artifact_deleted_at` set) is never swept again.
#[must_use]
pub fn job_needs_sweep(
    expires_at: Option<OffsetDateTime>,
    artifact_deleted_at: Option<OffsetDateTime>,
    now: OffsetDateTime,
) -> bool {
    artifact_deleted_at.is_none() && expires_at.is_some_and(|exp| now >= exp)
}

/// The outcome of one [`sweep`] pass.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SweepOutcome {
    /// Jobs examined this pass (past their deadline, not yet swept, up
    /// to the row cap).
    pub jobs_examined: u64,
    /// Jobs whose artifacts were deleted (or found already gone) and
    /// whose row was stamped `artifact_deleted_at`.
    pub jobs_swept: u64,
    /// Jobs where a delete or the stamp update failed — left unswept so
    /// the next pass retries them, never a hard failure of the pass.
    pub jobs_failed: u64,
}

/// Delete every artifact `job` references (`input_url` / `result_url` /
/// `error_report_url`, skipping any that are unset). Tolerates an
/// individual artifact that is already gone —
/// [`ArtifactStore::delete`] is idempotent — so re-running this against
/// a partially-swept job is safe.
///
/// # Errors
///
/// Returns the first delete failure the store reports for an artifact
/// that is not simply missing (e.g. a confinement violation, or a real
/// I/O error).
async fn delete_job_artifacts(store: &dyn ArtifactStore, job: &Model) -> Result<()> {
    for reference in [&job.input_url, &job.result_url, &job.error_report_url]
        .into_iter()
        .flatten()
    {
        store.delete(reference).await?;
    }
    Ok(())
}

/// Run one sweep pass: find every job past its retention deadline that
/// has not yet been physically swept (up to `limit` rows, oldest
/// deadline first — [`bulk_jobs::list_artifact_sweep_candidates`]),
/// delete its artifacts, and stamp `artifact_deleted_at`.
///
/// A per-job failure (an artifact delete, or the stamp update) is logged
/// at `warn` and leaves that row unstamped for the next pass to retry —
/// it never aborts the whole sweep and never panics.
///
/// # Errors
///
/// Returns an error only if the initial candidate-row query itself
/// fails; per-job failures are counted in [`SweepOutcome::jobs_failed`],
/// not propagated.
pub async fn sweep(
    db: &sea_orm::DatabaseConnection,
    store: &dyn ArtifactStore,
    limit: u64,
) -> Result<SweepOutcome> {
    let now = OffsetDateTime::now_utc();
    let candidates = bulk_jobs::list_artifact_sweep_candidates(db, now, limit).await?;
    let mut outcome = SweepOutcome {
        jobs_examined: u64::try_from(candidates.len()).unwrap_or(u64::MAX),
        ..SweepOutcome::default()
    };
    for job in &candidates {
        match delete_job_artifacts(store, job).await {
            Ok(()) => match bulk_jobs::mark_artifact_deleted(db, job.id).await {
                Ok(()) => outcome.jobs_swept += 1,
                Err(e) => {
                    tracing::warn!(
                        "bulk job {}: artifacts deleted but failed to stamp artifact_deleted_at \
                         (will retry next pass and may re-attempt an already-deleted artifact, \
                         which is safe): {e}",
                        job.id
                    );
                    outcome.jobs_failed += 1;
                }
            },
            Err(e) => {
                tracing::warn!(
                    "bulk job {}: artifact sweep delete failed, will retry next pass: {e}",
                    job.id
                );
                outcome.jobs_failed += 1;
            }
        }
    }
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::job_needs_sweep;
    use time::{Duration, OffsetDateTime};

    /// The pure eligibility rule: past (or at) the deadline, and not yet
    /// swept. Mirrors `artifact_expired_only_at_or_past_the_deadline` in
    /// `bulk::handlers` for the shared "at or past the deadline" edge.
    #[test]
    fn needs_sweep_only_when_past_deadline_and_not_yet_swept() {
        let now = OffsetDateTime::UNIX_EPOCH + Duration::seconds(1_000);

        assert!(
            job_needs_sweep(Some(now), None, now),
            "at the deadline, unswept, is due"
        );
        assert!(
            job_needs_sweep(Some(now - Duration::seconds(1)), None, now),
            "past the deadline, unswept, is due"
        );
        assert!(
            !job_needs_sweep(Some(now + Duration::seconds(1)), None, now),
            "not yet past the deadline"
        );
        assert!(
            !job_needs_sweep(Some(now), Some(now), now),
            "already swept, even if past the deadline"
        );
        assert!(
            !job_needs_sweep(None, None, now),
            "a job with no deadline (legacy row) is never swept"
        );
    }

    /// `delete_job_artifacts` deletes every referenced artifact and
    /// tolerates one that was never stored (the common case: a job with
    /// no error report has `error_report_url = None`, and any reference
    /// that is simply missing is a no-op per `ArtifactStore::delete`).
    #[tokio::test]
    async fn delete_job_artifacts_deletes_every_reference_and_tolerates_a_missing_one() {
        use crate::bulk::store::{ArtifactStore, LocalFsArtifactStore};

        let dir = tempfile::tempdir().unwrap();
        let store = LocalFsArtifactStore::new(dir.path());
        let input_ref = store.put("jobs/x/input.jsonl", b"in").await.unwrap();
        let result_ref = store.put("jobs/x/export.jsonl", b"out").await.unwrap();
        // Never stored — a legitimate "no error report" reference shape,
        // resolved but pointing at nothing.
        let missing_ref = format!("file://{}", dir.path().join("jobs/x/errors.csv").display());

        let job = test_model(
            Some(input_ref.clone()),
            Some(result_ref.clone()),
            Some(missing_ref),
        );
        super::delete_job_artifacts(&store, &job)
            .await
            .expect("a missing artifact must not fail the pass");

        assert!(store.get(&input_ref).await.is_err());
        assert!(store.get(&result_ref).await.is_err());
    }

    /// A job with no artifact references at all (e.g. a `failed` import
    /// that never got as far as storing input) is trivially a no-op.
    #[tokio::test]
    async fn delete_job_artifacts_with_no_references_is_a_no_op() {
        use crate::bulk::store::LocalFsArtifactStore;

        let dir = tempfile::tempdir().unwrap();
        let store = LocalFsArtifactStore::new(dir.path());
        let job = test_model(None, None, None);
        super::delete_job_artifacts(&store, &job)
            .await
            .expect("no references to delete");
    }

    /// Build a `bulk_jobs::Model` with placeholder values everywhere
    /// except the three artifact references under test — enough to
    /// exercise `delete_job_artifacts` without a database.
    fn test_model(
        input_url: Option<String>,
        result_url: Option<String>,
        error_report_url: Option<String>,
    ) -> crate::db::models::bulk_jobs::Model {
        let now = OffsetDateTime::now_utc();
        crate::db::models::bulk_jobs::Model {
            id: uuid::Uuid::new_v4(),
            kind: "import".to_string(),
            entity: "person".to_string(),
            format: "jsonl".to_string(),
            status: "completed".to_string(),
            params: serde_json::json!({}),
            rows_total: Some(0),
            rows_processed: 0,
            rows_created: 0,
            rows_upserted: 0,
            rows_to_review: 0,
            rows_errored: 0,
            actor: None,
            idempotency_key: None,
            input_url,
            result_url,
            error_report_url,
            created_at: now,
            updated_at: now,
            expires_at: None,
            artifact_deleted_at: None,
        }
    }
}

/// DB-gated (`#[ignore]`) tests for the full sweep pass. They need a
/// migrated `PostgreSQL` via `DATABASE_URL`; a bare `cargo test` skips
/// them but they MUST compile under `cargo test --no-run`.
#[cfg(test)]
mod db_tests {
    use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
    use time::{Duration, OffsetDateTime};
    use uuid::Uuid;

    use crate::bulk::store::{ArtifactStore, LocalFsArtifactStore};
    use crate::db::bulk_jobs;

    async fn connect() -> DatabaseConnection {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for DB tests");
        sea_orm::Database::connect(&url)
            .await
            .expect("connect to DATABASE_URL")
    }

    /// Insert a bulk job row directly (bypassing `db::bulk_jobs::create`,
    /// which always stamps a 7-day-out `expires_at`), so the test can
    /// pick an already-past or not-yet-past deadline.
    async fn insert_job(
        db: &DatabaseConnection,
        expires_at: Option<OffsetDateTime>,
        input_url: Option<String>,
    ) -> Uuid {
        let id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc();
        let model = crate::db::models::bulk_jobs::ActiveModel {
            id: Set(id),
            kind: Set("import".to_string()),
            entity: Set("person".to_string()),
            format: Set("jsonl".to_string()),
            status: Set("completed".to_string()),
            params: Set(serde_json::json!({})),
            rows_total: Set(Some(0)),
            rows_processed: Set(0),
            rows_created: Set(0),
            rows_upserted: Set(0),
            rows_to_review: Set(0),
            rows_errored: Set(0),
            actor: Set(None),
            idempotency_key: Set(None),
            input_url: Set(input_url),
            result_url: Set(None),
            error_report_url: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            expires_at: Set(expires_at),
            artifact_deleted_at: Set(None),
        };
        model.insert(db).await.expect("insert test bulk job");
        id
    }

    /// The end-to-end contract: an expired job's artifact bytes are
    /// physically gone after a sweep pass and its row is stamped; a
    /// non-expired job's artifact survives; and re-running the sweep is
    /// a safe no-op (the swept row no longer qualifies).
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn sweep_deletes_only_expired_unswept_artifacts_and_is_idempotent() {
        let db = connect().await;
        let dir = tempfile::tempdir().unwrap();
        let store = LocalFsArtifactStore::new(dir.path());

        let expired_ref = store.put("expired.jsonl", b"gone soon").await.unwrap();
        let live_ref = store.put("live.jsonl", b"stays").await.unwrap();

        let now = OffsetDateTime::now_utc();
        let expired_id = insert_job(
            &db,
            Some(now - Duration::hours(1)),
            Some(expired_ref.clone()),
        )
        .await;
        let live_id = insert_job(&db, Some(now + Duration::hours(1)), Some(live_ref.clone())).await;

        // First pass: only the expired job is examined and swept.
        let outcome = super::sweep(&db, &store, 100).await.unwrap();
        assert_eq!(outcome.jobs_examined, 1);
        assert_eq!(outcome.jobs_swept, 1);
        assert_eq!(outcome.jobs_failed, 0);

        assert!(
            store.get(&expired_ref).await.is_err(),
            "the expired job's artifact bytes must be physically gone"
        );
        assert!(
            store.get(&live_ref).await.is_ok(),
            "the non-expired job's artifact must survive the pass"
        );

        let expired_row = bulk_jobs::find_by_id(&db, expired_id)
            .await
            .unwrap()
            .expect("expired job row still exists (soft artifact deletion, not row deletion)");
        assert!(
            expired_row.artifact_deleted_at.is_some(),
            "the row must be stamped once swept"
        );
        let live_row = bulk_jobs::find_by_id(&db, live_id).await.unwrap().unwrap();
        assert!(
            live_row.artifact_deleted_at.is_none(),
            "an unswept job must not be stamped"
        );

        // Second pass: the swept row no longer qualifies as a candidate —
        // re-running the sweep is a safe no-op, not a re-delete attempt
        // that would have to tolerate a missing file (which it also
        // would, per the store-level idempotency tests, but the query
        // itself should already exclude it).
        let second = super::sweep(&db, &store, 100).await.unwrap();
        assert_eq!(
            second.jobs_examined, 0,
            "an already-swept job must not be re-selected"
        );
        assert_eq!(second.jobs_swept, 0);
        assert_eq!(second.jobs_failed, 0);
    }
}
