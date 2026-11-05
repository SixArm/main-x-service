DROP TABLE IF EXISTS event_text_values;
ALTER TABLE events ADD COLUMN alternate_names JSONB NOT NULL DEFAULT '[]'::jsonb;
ALTER TABLE events ADD COLUMN image           JSONB NOT NULL DEFAULT '[]'::jsonb;
ALTER TABLE events ADD COLUMN same_as         JSONB NOT NULL DEFAULT '[]'::jsonb;
ALTER TABLE events ADD COLUMN keywords        JSONB NOT NULL DEFAULT '[]'::jsonb;
ALTER TABLE events ADD COLUMN in_language     JSONB NOT NULL DEFAULT '[]'::jsonb;
