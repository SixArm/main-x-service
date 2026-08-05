-- SEC-B4 follow-up: physical artifact deletion (object-store TTL sweep).
-- `expires_at` already stops an expired job's artifact *reference* being
-- handed out (the status-handler 404 gate). It does not delete the
-- underlying bytes. `artifact_deleted_at` records that the periodic
-- sweep (`bulk_artifact_sweep` task, src/bulk/sweep.rs) has already
-- physically removed this job's artifacts from the store, so a swept
-- row is never re-processed by a later pass.
ALTER TABLE bulk_jobs ADD COLUMN IF NOT EXISTS artifact_deleted_at TIMESTAMPTZ;

-- The sweep's row source: jobs past their deadline, not yet swept,
-- oldest deadline first.
CREATE INDEX IF NOT EXISTS bulk_jobs_artifact_sweep
    ON bulk_jobs (expires_at)
    WHERE artifact_deleted_at IS NULL;
