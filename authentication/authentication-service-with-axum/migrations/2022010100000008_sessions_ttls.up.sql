-- Reshape `sessions` toward the shared-§3 schema
-- (`agents/share/authentication-sessions.md` §3): the **idle** and
-- **absolute** TTL columns and a `last_seen_at` sliding-window marker,
-- plus the partial index over active sessions.
--
-- The opaque session id stays the `jid` column. The new columns are
-- nullable so this migrates a populated table without a backfill; new
-- sessions set all three, and `is_active` treats a NULL bound as
-- "unbounded" (legacy grace).
ALTER TABLE sessions
    ADD COLUMN last_seen_at timestamp with time zone,
    ADD COLUMN idle_expires_at timestamp with time zone,
    ADD COLUMN absolute_expires_at timestamp with time zone;

-- The common lookup filters `revoked_at IS NULL`, so index only those rows.
CREATE INDEX IF NOT EXISTS sessions_active_user
    ON sessions (user_pid) WHERE revoked_at IS NULL;
