-- Tamper-evident audit history for `audit_log` (HIPAA §164.312(c)),
-- plus the read/disclosure columns (§164.312(b), §164.528).
--
-- `seq` exists because this table's primary key is an application-assigned
-- UUID, which gives no insertion order — and a hash chain needs a total
-- order to bind "the row before this one". `timestamp` alone is not enough
-- (two rows can share a microsecond, and ties would make verification
-- ambiguous), so a BIGSERIAL supplies the real append order.
--
-- Every column is nullable or defaulted, so rows written before this
-- migration stay valid; verification reports them as `unchained` rather
-- than as breaks. Backfilled `seq` values are assigned in arbitrary order,
-- which is harmless precisely because those rows are unchained anyway.

ALTER TABLE audit_log ADD COLUMN IF NOT EXISTS seq BIGSERIAL;
ALTER TABLE audit_log ADD COLUMN IF NOT EXISTS prev_hash TEXT;
ALTER TABLE audit_log ADD COLUMN IF NOT EXISTS hash TEXT;
ALTER TABLE audit_log ADD COLUMN IF NOT EXISTS context JSONB;
ALTER TABLE audit_log ADD COLUMN IF NOT EXISTS disclosure BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE audit_log ADD COLUMN IF NOT EXISTS redacted_at TIMESTAMPTZ;

-- The chain is read tail-first on every append and walked in order on
-- verification; both are `seq`-ordered.
CREATE INDEX IF NOT EXISTS audit_log_seq_idx ON audit_log (seq);
