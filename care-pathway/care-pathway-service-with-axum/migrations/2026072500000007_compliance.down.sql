DROP INDEX audit_logs_entity_pid_idx;

ALTER TABLE audit_logs
    DROP COLUMN prev_hash,
    DROP COLUMN hash,
    DROP COLUMN context,
    DROP COLUMN disclosure,
    DROP COLUMN redacted_at;
