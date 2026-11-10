//! Reconcile `bulk_jobs` with the axa job queue
//! (`agents/share/bulk-import-export.md` §3; recovery in
//! `agents/share/runbooks/bulk-job-recovery.md`).
//!
//! The two tables are separate bookkeeping. When axa gives up on a job, its
//! `axa_queue` row says `failed`, but the worker that would have updated
//! `bulk_jobs` is the thing that died, so the `bulk_jobs` row would stay
//! `queued` or `running` for ever. [`reconcile_lost`] closes that gap.
//!
//! **The link.** There is no foreign key. A queue row belongs to a bulk job
//! when its `name` is the worker's class name and `args->>'job_id'` is the
//! job's id, which is what `perform_later` writes.
//!
//! **What counts as lost.** A job in `queued` or `running` is lost when no
//! queue row for it is still alive, and either:
//!
//! - a queue row for it has `failed`. axa has no way out of `failed`, and it
//!   only gets there once the attempts are used up, so a job axa has requeued
//!   is never touched; or
//! - it has no queue row at all and has not changed for
//!   [`ENQUEUE_GRACE_MINUTES`]. The row is committed before its job is queued,
//!   so a young job with no queue row may simply be mid-request. An old one was
//!   never queued (`perform_later` failed) or its failed row was purged.
//!
//! A missing queue row cannot be a job that finished and was purged: axa
//! records `completed` only after the worker returns, by which time the job's
//! own row is terminal, and terminal rows are never selected here.
//!
//! The update is one statement, so it needs no lock, and a second worker
//! process running it at the same moment does no harm.

use std::time::Duration;

use axa::AppContext;
use axa::worker::BackgroundWorker;
use sqlx::PgPool;
use uuid::Uuid;

use crate::bulk::worker::{BulkJobArgs, BulkJobWorker};

/// How often a worker process reconciles, matching axa's own default
/// `maintenance_interval`.
const RECONCILE_INTERVAL: Duration = Duration::from_mins(5);

/// How long a job may sit with no queue row before it is called lost. Longer
/// than the longest request that can be creating it (`IMPORT_TIMEOUT_SECONDS`,
/// ten minutes, in `src/app.rs`).
pub const ENQUEUE_GRACE_MINUTES: i32 = 15;

/// The class name `perform_later` records on a bulk job's queue row.
fn job_worker_name() -> String {
    <BulkJobWorker as BackgroundWorker<BulkJobArgs>>::class_name()
}

/// Move every lost job to `failed`, returning the ids moved.
///
/// # Errors
///
/// When the statement fails, including when `axa_queue` does not exist. It
/// always does in a process that connected the queue, the only one that runs
/// this.
pub async fn reconcile_lost(db: &PgPool) -> crate::Result<Vec<Uuid>> {
    // The statement is `db::bulk_jobs::mark_lost`: one `UPDATE … RETURNING`,
    // so a job cannot finish between the check and the write.
    let moved =
        crate::db::bulk_jobs::mark_lost(db, &job_worker_name(), ENQUEUE_GRACE_MINUTES).await?;
    for id in &moved {
        tracing::error!(job_id = %id, "bulk job lost by the job queue; marked failed");
    }
    Ok(moved)
}

/// Reconcile straight away, to recover anything a previous process left
/// behind, and then on a timer.
async fn run(db: PgPool) {
    loop {
        if let Err(error) = reconcile_lost(&db).await {
            tracing::error!(%error, "could not reconcile bulk jobs");
        }
        tokio::time::sleep(RECONCILE_INTERVAL).await;
    }
}

/// Spawn the reconcile loop in a process that serves the queue. A no-op
/// otherwise, because a process that runs no jobs has no business failing them.
pub fn spawn(ctx: &AppContext, serving_queue: bool) {
    if serving_queue && ctx.queue_provider.is_some() {
        tokio::spawn(run(ctx.db.clone()));
    }
}

#[cfg(test)]
mod db_tests {
    use super::*;
    use axa::queue::{Queue, QueueOptions};
    use serial_test::serial;

    use crate::bulk::BulkFormat;
    use crate::db::bulk_jobs::{self, NewBulkJob};

    fn url() -> String {
        std::env::var("DB_URL").expect("DB_URL must be set for DB tests")
    }

    /// The real queue, so these tests pin the table axa actually writes.
    async fn connect() -> (PgPool, Queue) {
        let db = PgPool::connect(&url()).await.expect("connect to DB_URL");
        axa::sqlx::migrate(&db, &crate::db::MIGRATOR)
            .await
            .expect("run migrations");
        let queue = Queue::connect(&url(), QueueOptions::default())
            .await
            .expect("connect the queue");
        (db, queue)
    }

    /// A fresh `queued` job.
    async fn new_job(db: &PgPool) -> Uuid {
        let job = NewBulkJob::export(BulkFormat::Jsonl, serde_json::json!({}), None);
        bulk_jobs::create(db, job).await.expect("create job").id
    }

    /// Put a job in `status`, last changed `minutes` ago.
    async fn age_job(db: &PgPool, id: Uuid, status: &str, minutes: i32) {
        sqlx::query!(
            "UPDATE bulk_jobs SET status = $1, updated_at = now() - make_interval(mins => $2)
             WHERE id = $3",
            status,
            minutes,
            id,
        )
        .execute(db)
        .await
        .expect("statement runs");
    }

    async fn status_of(db: &PgPool, id: Uuid) -> String {
        sqlx::query_scalar!("SELECT status FROM bulk_jobs WHERE id = $1", id)
            .fetch_optional(db)
            .await
            .expect("query runs")
            .expect("job exists")
    }

    /// Queue `id` as `perform_later` would, with `max_attempts`.
    async fn enqueue(queue: &Queue, id: Uuid, max_attempts: Option<i32>) {
        let args = serde_json::to_value(BulkJobArgs { job_id: id }).expect("args serialize");
        queue
            .enqueue_limited(&job_worker_name(), args, max_attempts)
            .await
            .expect("enqueue");
    }

    /// Claim the job as a worker would, then lose that worker: the row is left
    /// `running`, with no heartbeat for two hours.
    async fn lose_worker(db: &PgPool, id: Uuid) {
        sqlx::query!(
            "UPDATE axa_queue SET status = 'running', attempts = attempts + 1,
                 started_at = now(), updated_at = now() - interval '2 hours'
             WHERE args->>'job_id' = $1",
            id.to_string(),
        )
        .execute(db)
        .await
        .expect("statement runs");
    }

    /// The bug: axa gives up on a job, and `bulk_jobs` must follow it.
    #[tokio::test]
    #[serial]
    #[ignore = "requires a running PostgreSQL via DB_URL"]
    async fn a_job_axa_gave_up_on_moves_to_failed() {
        let (db, queue) = connect().await;
        let id = new_job(&db).await;
        enqueue(&queue, id, None).await;
        age_job(&db, id, "running", 120).await;
        lose_worker(&db, id).await;

        // Before axa has given up, the queue row is alive.
        assert!(!reconcile_lost(&db).await.unwrap().contains(&id));
        assert_eq!(status_of(&db, id).await, "running");

        queue.requeue_stale().await.expect("axa sweeps");
        assert!(reconcile_lost(&db).await.unwrap().contains(&id));
        assert_eq!(status_of(&db, id).await, "failed");
    }

    /// A job axa queued again still has attempts, so it stays as it is until
    /// the last one is used up.
    #[tokio::test]
    #[serial]
    #[ignore = "requires a running PostgreSQL via DB_URL"]
    async fn a_requeued_job_is_not_failed_while_it_has_attempts() {
        let (db, queue) = connect().await;
        let id = new_job(&db).await;
        enqueue(&queue, id, Some(2)).await;
        age_job(&db, id, "running", 120).await;
        lose_worker(&db, id).await;
        assert!(queue.requeue_stale().await.expect("axa sweeps") >= 1);

        assert!(!reconcile_lost(&db).await.unwrap().contains(&id));
        assert_eq!(status_of(&db, id).await, "running");

        lose_worker(&db, id).await;
        queue.requeue_stale().await.expect("axa sweeps");
        assert!(reconcile_lost(&db).await.unwrap().contains(&id));
        assert_eq!(status_of(&db, id).await, "failed");
    }

    /// A row is committed before its job is queued, so a young job with no
    /// queue row is mid-request. An old one will never run.
    #[tokio::test]
    #[serial]
    #[ignore = "requires a running PostgreSQL via DB_URL"]
    async fn a_job_with_no_queue_row_is_lost_only_after_the_grace() {
        let (db, _queue) = connect().await;
        let young = new_job(&db).await;
        age_job(&db, young, "queued", ENQUEUE_GRACE_MINUTES - 1).await;
        let old = new_job(&db).await;
        age_job(&db, old, "running", ENQUEUE_GRACE_MINUTES + 1).await;

        let moved = reconcile_lost(&db).await.unwrap();
        assert!(!moved.contains(&young));
        assert!(moved.contains(&old));
        assert_eq!(status_of(&db, young).await, "queued");
        assert_eq!(status_of(&db, old).await, "failed");
    }

    /// However old it looks, a job with a live queue row is still in axa's
    /// hands: heartbeats touch `axa_queue`, not `bulk_jobs`.
    #[tokio::test]
    #[serial]
    #[ignore = "requires a running PostgreSQL via DB_URL"]
    async fn a_long_job_with_a_live_queue_row_is_left_alone() {
        let (db, queue) = connect().await;
        let waiting = new_job(&db).await;
        enqueue(&queue, waiting, None).await;
        age_job(&db, waiting, "queued", 600).await;
        let long = new_job(&db).await;
        enqueue(&queue, long, None).await;
        sqlx::query!(
            "UPDATE axa_queue SET status = 'running', attempts = 1 WHERE args->>'job_id' = $1",
            long.to_string(),
        )
        .execute(&db)
        .await
        .expect("statement runs");
        age_job(&db, long, "running", 600).await;

        let moved = reconcile_lost(&db).await.unwrap();
        assert!(!moved.contains(&waiting));
        assert!(!moved.contains(&long));
    }

    /// A finished job keeps its outcome, even if its queue row failed.
    #[tokio::test]
    #[serial]
    #[ignore = "requires a running PostgreSQL via DB_URL"]
    async fn a_terminal_job_is_never_moved() {
        let (db, queue) = connect().await;
        let id = new_job(&db).await;
        enqueue(&queue, id, None).await;
        sqlx::query!(
            "UPDATE axa_queue SET status = 'failed' WHERE args->>'job_id' = $1",
            id.to_string(),
        )
        .execute(&db)
        .await
        .expect("statement runs");
        age_job(&db, id, "completed", 120).await;

        assert!(!reconcile_lost(&db).await.unwrap().contains(&id));
        assert_eq!(status_of(&db, id).await, "completed");
    }

    /// Other workers carry a `job_id` too, so the link includes the name.
    #[tokio::test]
    #[serial]
    #[ignore = "requires a running PostgreSQL via DB_URL"]
    async fn a_failed_row_from_another_worker_is_no_link() {
        let (db, queue) = connect().await;
        let id = new_job(&db).await;
        queue
            .enqueue("SomeOtherWorker", serde_json::json!({ "job_id": id }))
            .await
            .expect("enqueue");
        sqlx::query!("UPDATE axa_queue SET status = 'failed' WHERE name = 'SomeOtherWorker'")
            .execute(&db)
            .await
            .expect("statement runs");

        assert!(!reconcile_lost(&db).await.unwrap().contains(&id));
        assert_eq!(status_of(&db, id).await, "queued");
    }

    /// The loop's first pass runs straight away, so a restarted worker
    /// recovers what its predecessor lost without waiting for the timer.
    #[tokio::test]
    #[serial]
    #[ignore = "requires a running PostgreSQL via DB_URL"]
    async fn the_loop_reconciles_on_its_first_pass() {
        let (db, _queue) = connect().await;
        let id = new_job(&db).await;
        age_job(&db, id, "running", ENQUEUE_GRACE_MINUTES + 1).await;

        let task = tokio::spawn(run(db.clone()));
        let mut status = String::new();
        for _ in 0..50 {
            status = status_of(&db, id).await;
            if status == "failed" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        task.abort();
        assert_eq!(status, "failed");
    }
}
