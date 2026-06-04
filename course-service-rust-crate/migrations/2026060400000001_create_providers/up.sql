-- Providers (issuing organisations).
CREATE TABLE IF NOT EXISTS providers (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name                VARCHAR(500)                NOT NULL,
    alternate_names     JSONB                       NOT NULL DEFAULT '[]'::jsonb,
    url                 VARCHAR(2048),
    same_as             JSONB                       NOT NULL DEFAULT '[]'::jsonb,
    kind                VARCHAR(50),
    created_at          TIMESTAMP WITH TIME ZONE    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at          TIMESTAMP WITH TIME ZONE    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at          TIMESTAMP WITH TIME ZONE
);

CREATE INDEX IF NOT EXISTS idx_providers_name        ON providers (LOWER(name));
CREATE INDEX IF NOT EXISTS idx_providers_deleted_at  ON providers (deleted_at);
