-- Audit log: the change trail for every CRUD mutation.
CREATE TABLE IF NOT EXISTS audit_log (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_type         VARCHAR(64)                 NOT NULL,
    entity_id           UUID                        NOT NULL,
    action              VARCHAR(16)                 NOT NULL,
    user_id             VARCHAR(256),
    user_ip_address     VARCHAR(64),
    user_agent          VARCHAR(512),
    old_values          JSONB,
    new_values          JSONB,
    created_at          TIMESTAMP WITH TIME ZONE    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_audit_log_entity_id   ON audit_log (entity_id);
CREATE INDEX IF NOT EXISTS idx_audit_log_created_at  ON audit_log (created_at);

-- Merge history: one row per fold of a duplicate thing into a main thing.
CREATE TABLE IF NOT EXISTS thing_merge_records (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    main_thing_id       UUID                        NOT NULL,
    duplicate_thing_id  UUID                        NOT NULL,
    merge_reason        TEXT,
    transferred_data    JSONB,
    merged_at           TIMESTAMP WITH TIME ZONE    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_thing_merge_main  ON thing_merge_records (main_thing_id);
