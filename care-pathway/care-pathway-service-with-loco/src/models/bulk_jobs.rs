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
//!    │          │
//!    │          └──────▶ failed
//!    └─────────────────▶ cancelled
//! ```
//!
//! `cancelled` is reachable from `queued` or `running` (the Bulk Data IG's
//! `DELETE` on the status endpoint). A terminal status never changes
//! again, which is what lets the status endpoint be read without a lock.

use chrono::SubsecRound as _;
use loco_rs::prelude::*;
use sea_orm::{ConnectionTrait, QueryOrder, QuerySelect};
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
    /// Finished with a hard failure; `error` says why.
    pub const FAILED: &str = "failed";
    /// Cancelled by the client.
    pub const CANCELLED: &str = "cancelled";
}

/// Whether a status is terminal — no worker will move it again.
#[must_use]
pub fn is_terminal(s: &str) -> bool {
    matches!(s, status::COMPLETED | status::FAILED | status::CANCELLED)
}

/// How long a completed job and its artifact stay retrievable.
///
/// The Bulk Data IG leaves retention to the server. Fifteen minutes
/// matches what the in-process registry offered, so adopting the durable
/// store does not silently extend how long clinical data sits in an
/// artifact.
pub const JOB_TTL_SECS: i64 = 900;

impl Model {
    /// Submit a job, or return the existing one for the same
    /// `idempotency_key`.
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
        if let Some(key) = idempotency_key
            && let Some(existing) = bulk_jobs::Entity::find()
                .filter(bulk_jobs::Column::Entity.eq(entity))
                .filter(bulk_jobs::Column::Kind.eq(job_kind))
                .filter(bulk_jobs::Column::IdempotencyKey.eq(key))
                .one(db)
                .await?
        {
            return Ok(existing);
        }
        let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().trunc_subsecs(6).into();
        let job = bulk_jobs::ActiveModel {
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
            expires_at: ActiveValue::set(Some(now + chrono::Duration::seconds(JOB_TTL_SECS))),
        }
        .insert(db)
        .await?;
        Ok(job)
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

    /// Recent jobs for this entity, newest first.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn recent<C: ConnectionTrait>(
        db: &C,
        entity: &str,
        limit: u64,
    ) -> ModelResult<Vec<Self>> {
        Ok(bulk_jobs::Entity::find()
            .filter(bulk_jobs::Column::Entity.eq(entity))
            .order_by_desc(bulk_jobs::Column::CreatedAt)
            .limit(limit)
            .all(db)
            .await?)
    }
}

#[cfg(test)]
mod tests {
    use super::{is_terminal, status};

    /// Only the three finished states are terminal; a terminal status is
    /// what lets the status endpoint be read without taking a lock.
    #[test]
    fn terminal_states_are_exactly_the_finished_ones() {
        assert!(is_terminal(status::COMPLETED));
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
                status::FAILED,
                status::CANCELLED
            ],
            ["queued", "running", "completed", "failed", "cancelled"]
        );
    }
}
