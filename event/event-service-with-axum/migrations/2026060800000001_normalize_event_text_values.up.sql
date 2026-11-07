-- Normalize the Event string-list columns (alternate_names, image,
-- same_as, keywords, in_language) from JSONB into a tagged child table.
ALTER TABLE events
    DROP COLUMN IF EXISTS alternate_names,
    DROP COLUMN IF EXISTS image,
    DROP COLUMN IF EXISTS same_as,
    DROP COLUMN IF EXISTS keywords,
    DROP COLUMN IF EXISTS in_language;

CREATE TABLE event_text_values (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id    UUID          NOT NULL REFERENCES events (id) ON DELETE CASCADE,
    field       VARCHAR(16)   NOT NULL,
    value       VARCHAR(2048) NOT NULL,
    position    INTEGER       NOT NULL DEFAULT 0
);
CREATE INDEX idx_event_text_values_event ON event_text_values (event_id, field);
