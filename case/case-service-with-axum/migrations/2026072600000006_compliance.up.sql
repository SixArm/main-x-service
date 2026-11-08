-- Extend `audit_logs` with the compliance columns: the tamper-evident
-- hash chain (`prev_hash` / `hash`), the per-access `context`
-- (purpose-of-use, residency, lawful basis), the `disclosure` flag that
-- separates an internal access from an outward disclosure (HIPAA §164.528
-- accounting), and `redacted_at`, the GDPR Art. 17 marker for a row whose
-- content has been destroyed while its chain linkage survives.
--
-- All added columns are nullable (or defaulted), so rows written before
-- this migration remain valid: chain verification reports them as
-- `unchained` rather than as a break.
ALTER TABLE audit_logs
    ADD COLUMN prev_hash character varying NULL,
    ADD COLUMN hash character varying NULL,
    ADD COLUMN context jsonb NULL,
    ADD COLUMN disclosure boolean NOT NULL DEFAULT false,
    ADD COLUMN redacted_at timestamp with time zone NULL;

-- Per-record audit + disclosure queries (`/{pid}/audit`,
-- `/{pid}/disclosures`, and the Art. 17 redaction sweep) all filter on
-- `entity_pid`; without this they are sequential scans.
CREATE INDEX audit_logs_entity_pid_idx ON audit_logs (entity_pid);
