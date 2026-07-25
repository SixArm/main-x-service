-- Workforce assessments: one row per administration of one instrument
-- (a named test) to one worker. `category` is the family of test
-- (aptitude / personality / psychometric / selection) and `status` the
-- lifecycle token (scheduled / in_progress / completed / expired /
-- cancelled) — both stored as the family's lowercase wire tokens, with
-- the closed vocabulary owned by the domain model so it can grow by
-- data migration rather than DDL.
--
-- The per-scale outcomes ride in the `results` JSONB array (one object
-- per measured dimension: scale, raw_score, max_score, percentile,
-- band, narrative). They are read and written as a whole assessment,
-- never queried field-by-field, so a child table would buy nothing.
--
-- Assessment results are sensitive personal data (they profile
-- cognition and behaviour), so reads are authorized and audited at the
-- handler and masked under the ABAC `mask` obligation. Deletion is
-- soft (`deleted_at`) so the audit trail stays intact.
CREATE TABLE IF NOT EXISTS worker_assessments (
    id UUID PRIMARY KEY,
    worker_id UUID NOT NULL,
    category VARCHAR NOT NULL,
    instrument VARCHAR NOT NULL,
    provider VARCHAR NULL,
    status VARCHAR NOT NULL DEFAULT 'scheduled',
    administered_on DATE NULL,
    expires_on DATE NULL,
    administered_by VARCHAR NULL,
    notes VARCHAR NULL,
    results JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ NULL
);

-- The dominant query: one worker's live assessments, newest first.
CREATE INDEX IF NOT EXISTS worker_assessments_worker_idx
    ON worker_assessments (worker_id, administered_on DESC)
    WHERE deleted_at IS NULL;

-- Category filtering on the profile view.
CREATE INDEX IF NOT EXISTS worker_assessments_category_idx
    ON worker_assessments (category)
    WHERE deleted_at IS NULL;
