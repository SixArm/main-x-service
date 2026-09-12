//! `bulk_jobs` model — durable lifecycle for one asynchronous bulk
//! operation.
//!
//! Replaces the process-local export registry that
//! [`crate::compliance::bulk`] used to keep, so a job survives a restart
//! and is visible to every replica. Follows the family contract in
//! [`agents/share/bulk-import-export.md`](../../../../agents/share/bulk-import-export.md)
//! §3.
//!
//! ## Status machine
//!
//! ```text
//! queued ──▶ running ──▶ completed
//!    │          │    └─▶ completed_with_errors  (import only — §7)
//!    │          └──────▶ failed
//!    └─────────────────▶ cancelled
//! ```
//!
//! `cancelled` is reachable from `queued` or `running` (the Bulk Data IG's
//! `DELETE` on the status endpoint). `completed_with_errors` is reachable
//! only from `running`, and only for an **import** job — an export never
//! has per-row failures of its own kind. A terminal status never changes
//! again, which is what lets the status endpoint be read without a lock.
//!
//! This table serves two consumers: the FHIR Bulk Data `$export`
//! operation (`entity = "care_pathway"`, `kind = "export"`,
//! `format = "ndjson"` — [`crate::workers::bulk_export`]) and the native
//! bulk import/export API (§13 T-10; `format = "jsonl" | "csv" | "tsv"` —
//! [`crate::bulk`]). They share one row shape and one table rather than
//! two, per the family contract; the `format` column is what
//! distinguishes an FHIR job from a native one when listing.

use chrono::SubsecRound as _;
use loco_rs::prelude::*;
use sea_orm::{ColumnTrait, ConnectionTrait, QueryFilter, QueryOrder, QuerySelect};
use uuid::Uuid;

/// Re-export the generated `bulk_jobs` entity.
pub use super::_entities::bulk_jobs::{self, ActiveModel, Entity, Model};

/// Default `SeaORM` active-model behaviour — no custom hooks.
impl ActiveModelBehavior for super::_entities::bulk_jobs::ActiveModel {}

/// Job kinds.
pub mod kind {
    /// An export job.
    pub const EXPORT: &str = "export";
    /// An import job (the native bulk API; not implemented yet).
    pub const IMPORT: &str = "import";
}

/// Job statuses.
pub mod status {
    /// Submitted, not yet picked up by a worker.
    pub const QUEUED: &str = "queued";
    /// A worker is materialising the job.
    pub const RUNNING: &str = "running";
    /// Finished; `result_url` references the output artifact.
    pub const COMPLETED: &str = "completed";
    /// Finished (import only): at least one row landed in the per-row
    /// error report (§7). `rows_created`/`rows_upserted`/`rows_to_review`
    /// still reflect every row that *did* commit — one bad row never
    /// aborts the rest of the load.
    pub const COMPLETED_WITH_ERRORS: &str = "completed_with_errors";
    /// Finished with a hard failure; `error` says why.
    pub const FAILED: &str = "failed";
    /// Cancelled by the client.
    pub const CANCELLED: &str = "cancelled";
}

/// Whether a status is terminal — no worker will move it again.
#[must_use]
pub fn is_terminal(s: &str) -> bool {
    matches!(
        s,
        status::COMPLETED | status::COMPLETED_WITH_ERRORS | status::FAILED | status::CANCELLED
    )
}

/// How long a completed **FHIR Bulk Data `$export`** job and its artifact
/// stay retrievable ([`submit`](Model::submit)).
///
/// The Bulk Data IG leaves retention to the server. Fifteen minutes
/// matches what the in-process registry offered, so adopting the durable
/// store does not silently extend how long clinical data sits in an
/// artifact.
pub const JOB_TTL_SECS: i64 = 900;

/// How long a **native bulk import/export** job and its artifacts stay
/// retrievable ([`submit_with_ttl`](Model::submit_with_ttl)).
///
/// A native bulk load is an operator-initiated, often large, one-off
/// action — the FHIR Bulk Data IG's fifteen-minute window (above) would
/// routinely expire before an operator downloads the result or the error
/// report. Seven days matches the family reference (organization/case
/// BLK-5).
pub const NATIVE_BULK_TTL_SECS: i64 = 7 * 24 * 60 * 60;

/// Saturating `u64` → `i64`, for the count columns.
fn i64_of(n: u64) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

impl Model {
    /// Submit a **FHIR Bulk Data** job, or return the existing one for the
    /// same `idempotency_key`, on the [`JOB_TTL_SECS`] retention window.
    ///
    /// The key makes a retried submit return the same job rather than
    /// starting a second export of the same data (shared doc §3). Without
    /// a key every submit is a new job, which is the correct default for
    /// a `GET`-triggered `$export`.
    ///
    /// # Errors
    ///
    /// When the lookup or insert fails.
    pub async fn submit<C: ConnectionTrait>(
        db: &C,
        entity: &str,
        job_kind: &str,
        format: &str,
        params: serde_json::Value,
        actor: Option<&str>,
        idempotency_key: Option<&str>,
    ) -> ModelResult<Self> {
        let (job, _reused) = Self::submit_with_ttl(
            db,
            entity,
            job_kind,
            format,
            params,
            actor,
            idempotency_key,
            JOB_TTL_SECS,
        )
        .await?;
        Ok(job)
    }

    /// Submit a job on a caller-chosen retention window, reporting whether
    /// an existing job for the same `idempotency_key` was returned instead
    /// of a new insert ([`NATIVE_BULK_TTL_SECS`] — the native bulk
    /// import/export API, §13 T-10).
    ///
    /// The `UNIQUE (entity, kind, idempotency_key)` constraint backstops
    /// the check-then-insert race: on an insert failure with a key
    /// present, the existing row is re-fetched and returned as the race
    /// winner rather than propagating a spurious error.
    ///
    /// # Errors
    ///
    /// When the lookup fails, or the insert fails for a reason other than
    /// a losing idempotency race.
    #[allow(clippy::too_many_arguments)]
    pub async fn submit_with_ttl<C: ConnectionTrait>(
        db: &C,
        entity: &str,
        job_kind: &str,
        format: &str,
        params: serde_json::Value,
        actor: Option<&str>,
        idempotency_key: Option<&str>,
        ttl_secs: i64,
    ) -> ModelResult<(Self, bool)> {
        if let Some(key) = idempotency_key
            && let Some(existing) = Self::find_by_idempotency_key(db, entity, job_kind, key).await?
        {
            return Ok((existing, true));
        }
        let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().trunc_subsecs(6).into();
        let inserted = bulk_jobs::ActiveModel {
            id: ActiveValue::set(Uuid::new_v4()),
            kind: ActiveValue::set(job_kind.to_string()),
            entity: ActiveValue::set(entity.to_string()),
            format: ActiveValue::set(format.to_string()),
            status: ActiveValue::set(status::QUEUED.to_string()),
            params: ActiveValue::set(params),
            rows_total: ActiveValue::set(None),
            rows_processed: ActiveValue::set(0),
            rows_created: ActiveValue::set(0),
            rows_upserted: ActiveValue::set(0),
            rows_to_review: ActiveValue::set(0),
            rows_errored: ActiveValue::set(0),
            actor: ActiveValue::set(actor.map(ToString::to_string)),
            idempotency_key: ActiveValue::set(idempotency_key.map(ToString::to_string)),
            input_url: ActiveValue::set(None),
            result_url: ActiveValue::set(None),
            error_report_url: ActiveValue::set(None),
            error: ActiveValue::set(None),
            created_at: ActiveValue::set(now),
            updated_at: ActiveValue::set(now),
            expires_at: ActiveValue::set(Some(now + chrono::Duration::seconds(ttl_secs))),
        }
        .insert(db)
        .await;
        match inserted {
            Ok(job) => Ok((job, false)),
            Err(insert_err) => {
                if let Some(key) = idempotency_key
                    && let Some(existing) =
                        Self::find_by_idempotency_key(db, entity, job_kind, key).await?
                {
                    return Ok((existing, true));
                }
                Err(insert_err.into())
            }
        }
    }

    /// Find an existing job for `entity` + `job_kind` bearing
    /// `idempotency_key`, or `None`. Matches the
    /// `UNIQUE (entity, kind, idempotency_key)` constraint so a retried
    /// submit resolves to the original job.
    ///
    /// # Errors
    ///
    /// When the query fails.
    async fn find_by_idempotency_key<C: ConnectionTrait>(
        db: &C,
        entity: &str,
        job_kind: &str,
        key: &str,
    ) -> ModelResult<Option<Self>> {
        Ok(bulk_jobs::Entity::find()
            .filter(bulk_jobs::Column::Entity.eq(entity))
            .filter(bulk_jobs::Column::Kind.eq(job_kind))
            .filter(bulk_jobs::Column::IdempotencyKey.eq(key))
            .one(db)
            .await?)
    }

    /// Attach the uploaded input artifact reference to an import job.
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn set_input_url<C: ConnectionTrait>(
        self,
        db: &C,
        input_url: String,
    ) -> ModelResult<Self> {
        let mut active = self.into_active_model();
        active.input_url = ActiveValue::set(Some(input_url));
        active.updated_at = ActiveValue::set(chrono::Utc::now().trunc_subsecs(6).into());
        Ok(active.update(db).await?)
    }

    /// Record the reconciled outcome of an **import** run (§7):
    /// `rows_total = rows_created + rows_upserted + rows_errored`
    /// (`rows_to_review` is a sub-count of `rows_created`, not a fourth
    /// exclusive bucket — see `crate::bulk::pipeline::ImportOutcome`).
    /// Status is [`status::COMPLETED`] when nothing errored, else
    /// [`status::COMPLETED_WITH_ERRORS`] — one bad row never aborts the
    /// rest of the load, so the job still finishes rather than failing.
    ///
    /// # Errors
    ///
    /// When the update fails.
    #[allow(clippy::too_many_arguments)]
    pub async fn finish_import<C: ConnectionTrait>(
        self,
        db: &C,
        rows_total: u64,
        rows_created: u64,
        rows_upserted: u64,
        rows_to_review: u64,
        rows_errored: u64,
        error_report_url: Option<String>,
    ) -> ModelResult<Self> {
        let final_status = if rows_errored > 0 {
            status::COMPLETED_WITH_ERRORS
        } else {
            status::COMPLETED
        };
        let mut active = self.into_active_model();
        active.status = ActiveValue::set(final_status.to_string());
        active.rows_total = ActiveValue::set(Some(i64_of(rows_total)));
        active.rows_processed = ActiveValue::set(i64_of(rows_total));
        active.rows_created = ActiveValue::set(i64_of(rows_created));
        active.rows_upserted = ActiveValue::set(i64_of(rows_upserted));
        active.rows_to_review = ActiveValue::set(i64_of(rows_to_review));
        active.rows_errored = ActiveValue::set(i64_of(rows_errored));
        active.error_report_url = ActiveValue::set(error_report_url);
        active.updated_at = ActiveValue::set(chrono::Utc::now().trunc_subsecs(6).into());
        Ok(active.update(db).await?)
    }

    /// Fetch a job by id.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_by_id<C: ConnectionTrait>(db: &C, id: Uuid) -> ModelResult<Option<Self>> {
        Ok(bulk_jobs::Entity::find_by_id(id).one(db).await?)
    }

    /// Claim a queued job for a worker, moving it to `running`.
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn start<C: ConnectionTrait>(self, db: &C) -> ModelResult<Self> {
        self.transition(db, status::RUNNING, None, None, None).await
    }

    /// Mark a job completed with its output artifact and row count.
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn complete<C: ConnectionTrait>(
        self,
        db: &C,
        result_url: &str,
        rows: i64,
    ) -> ModelResult<Self> {
        self.transition(db, status::COMPLETED, Some(result_url), None, Some(rows))
            .await
    }

    /// Mark a job failed, recording why.
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn fail<C: ConnectionTrait>(self, db: &C, reason: &str) -> ModelResult<Self> {
        self.transition(db, status::FAILED, None, Some(reason), None)
            .await
    }

    /// Cancel a job, clearing any output reference.
    ///
    /// The artifact itself is swept by retention; dropping the reference
    /// is what makes it unreachable, so a cancelled export stops serving
    /// clinical data immediately rather than at TTL.
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn cancel<C: ConnectionTrait>(self, db: &C) -> ModelResult<Self> {
        let mut active = self.into_active_model();
        active.status = ActiveValue::set(status::CANCELLED.to_string());
        active.result_url = ActiveValue::set(None);
        active.updated_at = ActiveValue::set(chrono::Utc::now().trunc_subsecs(6).into());
        Ok(active.update(db).await?)
    }

    /// The shared status transition.
    async fn transition<C: ConnectionTrait>(
        self,
        db: &C,
        to: &str,
        result_url: Option<&str>,
        error: Option<&str>,
        rows: Option<i64>,
    ) -> ModelResult<Self> {
        let mut active = self.into_active_model();
        active.status = ActiveValue::set(to.to_string());
        if let Some(url) = result_url {
            active.result_url = ActiveValue::set(Some(url.to_string()));
        }
        if let Some(reason) = error {
            active.error = ActiveValue::set(Some(reason.to_string()));
        }
        if let Some(n) = rows {
            active.rows_processed = ActiveValue::set(n);
            active.rows_total = ActiveValue::set(Some(n));
        }
        active.updated_at = ActiveValue::set(chrono::Utc::now().trunc_subsecs(6).into());
        Ok(active.update(db).await?)
    }

    /// Whether the job has outlived its retention window.
    #[must_use]
    pub fn is_expired(&self, now: chrono::DateTime<chrono::Utc>) -> bool {
        self.expires_at.is_some_and(|e| e.to_utc() < now)
    }

    /// Recent jobs for this entity, newest first, optionally filtered by
    /// `kind` (`import`/`export`) and/or `status` (shared doc §4: `GET
    /// /api/care-pathways/bulk-jobs` "list (filter by `kind`/`status`)").
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn recent<C: ConnectionTrait>(
        db: &C,
        entity: &str,
        kind: Option<&str>,
        status: Option<&str>,
        limit: u64,
    ) -> ModelResult<Vec<Self>> {
        let mut query = bulk_jobs::Entity::find().filter(bulk_jobs::Column::Entity.eq(entity));
        if let Some(k) = kind {
            query = query.filter(bulk_jobs::Column::Kind.eq(k));
        }
        if let Some(s) = status {
            query = query.filter(bulk_jobs::Column::Status.eq(s));
        }
        Ok(query
            .order_by_desc(bulk_jobs::Column::CreatedAt)
            .limit(limit)
            .all(db)
            .await?)
    }
}

#[cfg(test)]
mod tests {
    use super::{is_terminal, status};

    /// Only the four finished states are terminal; a terminal status is
    /// what lets the status endpoint be read without taking a lock.
    #[test]
    fn terminal_states_are_exactly_the_finished_ones() {
        assert!(is_terminal(status::COMPLETED));
        assert!(is_terminal(status::COMPLETED_WITH_ERRORS));
        assert!(is_terminal(status::FAILED));
        assert!(is_terminal(status::CANCELLED));
        assert!(!is_terminal(status::QUEUED));
        assert!(!is_terminal(status::RUNNING));
        assert!(!is_terminal("something-else"));
    }

    /// The status tokens are the wire contract, so pin their spelling.
    #[test]
    fn status_tokens_are_stable() {
        assert_eq!(
            [
                status::QUEUED,
                status::RUNNING,
                status::COMPLETED,
                status::COMPLETED_WITH_ERRORS,
                status::FAILED,
                status::CANCELLED
            ],
            [
                "queued",
                "running",
                "completed",
                "completed_with_errors",
                "failed",
                "cancelled"
            ]
        );
    }

    /// `i64_of` saturates rather than panicking/wrapping on an absurd
    /// count.
    #[test]
    fn i64_of_saturates() {
        assert_eq!(super::i64_of(5), 5);
        assert_eq!(super::i64_of(u64::MAX), i64::MAX);
    }
}
