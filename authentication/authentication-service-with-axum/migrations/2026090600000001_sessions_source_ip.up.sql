-- `sessions.source_ip` (T-14): the best-effort connecting-peer address
-- captured at issuance. Nullable, exactly like `user_agent`; neither is
-- backfillable for rows issued before this migration.
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS source_ip character varying;
