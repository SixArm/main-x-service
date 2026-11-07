-- Durable per-pass audit trail for the periodic cross-service
-- `same_identity` suggestion job (T-33, design §16 OQ-9(d) "audit ...
-- every run's counts"). One row per completed pass; a pass that fails at
-- the fetch step records nothing (matches the job's existing
-- log-and-retry posture for that case — there are no counts worth
-- keeping from a run that never fetched anything).
CREATE TABLE suggestion_runs (
    id                  UUID PRIMARY KEY,
    started_at          TIMESTAMPTZ NOT NULL,
    completed_at        TIMESTAMPTZ NOT NULL,
    persons_fetched     BIGINT NOT NULL,
    workers_fetched     BIGINT NOT NULL,
    candidates          BIGINT NOT NULL,
    posted              BIGINT NOT NULL,
    failed              BIGINT NOT NULL,
    dropped             BIGINT NOT NULL,
    max_candidates      BIGINT NOT NULL,
    max_edges_per_run   BIGINT NOT NULL
);
CREATE INDEX suggestion_runs_completed_at ON suggestion_runs (completed_at);
