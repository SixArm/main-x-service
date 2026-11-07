-- Things (schema.org/Thing), fully normalized. Repeating collections live
-- in dedicated child tables rather than JSONB columns.
CREATE TABLE IF NOT EXISTS things (
    id                          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name                        VARCHAR(500)                NOT NULL,
    description                 TEXT,
    disambiguating_description  TEXT,
    additional_type             VARCHAR(2048),
    url                         VARCHAR(2048),
    main_entity_of_page         VARCHAR(2048),
    owner                       VARCHAR(500),
    subject_of                  VARCHAR(2048),
    potential_action            VARCHAR(2048),
    is_deleted                  BOOLEAN                     NOT NULL DEFAULT FALSE,
    deleted_at                  TIMESTAMP WITH TIME ZONE,
    created_at                  TIMESTAMP WITH TIME ZONE    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at                  TIMESTAMP WITH TIME ZONE    NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_things_name        ON things (LOWER(name));
CREATE INDEX IF NOT EXISTS idx_things_is_deleted  ON things (is_deleted);
CREATE INDEX IF NOT EXISTS idx_things_created_at  ON things (created_at);

CREATE TABLE IF NOT EXISTS thing_alternate_names (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    thing_id    UUID         NOT NULL REFERENCES things (id) ON DELETE CASCADE,
    name        VARCHAR(500) NOT NULL,
    position    INTEGER      NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_thing_alternate_names_thing ON thing_alternate_names (thing_id);

CREATE TABLE IF NOT EXISTS thing_identifiers (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    thing_id        UUID         NOT NULL REFERENCES things (id) ON DELETE CASCADE,
    property_id     VARCHAR(32)  NOT NULL,
    custom_label    VARCHAR(128),
    value           VARCHAR(512) NOT NULL,
    name            VARCHAR(500),
    url             VARCHAR(2048),
    position        INTEGER      NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_thing_identifiers_thing ON thing_identifiers (thing_id);
CREATE INDEX IF NOT EXISTS idx_thing_identifiers_value ON thing_identifiers (property_id, value);

CREATE TABLE IF NOT EXISTS thing_images (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    thing_id    UUID          NOT NULL REFERENCES things (id) ON DELETE CASCADE,
    url         VARCHAR(2048) NOT NULL,
    position    INTEGER       NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_thing_images_thing ON thing_images (thing_id);

CREATE TABLE IF NOT EXISTS thing_same_as (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    thing_id    UUID          NOT NULL REFERENCES things (id) ON DELETE CASCADE,
    url         VARCHAR(2048) NOT NULL,
    position    INTEGER       NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_thing_same_as_thing ON thing_same_as (thing_id);
