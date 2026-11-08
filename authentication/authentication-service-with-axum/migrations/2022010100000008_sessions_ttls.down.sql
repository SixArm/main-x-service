DROP INDEX IF EXISTS sessions_active_user;
ALTER TABLE sessions
    DROP COLUMN absolute_expires_at,
    DROP COLUMN idle_expires_at,
    DROP COLUMN last_seen_at;
