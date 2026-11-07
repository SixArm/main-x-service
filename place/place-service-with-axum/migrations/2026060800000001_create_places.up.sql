-- Places (schema.org/Place), fully normalized. The 0..1 address/geo objects
-- are flattened onto the row; repeating collections live in child tables.
CREATE TABLE IF NOT EXISTS places (
    id                          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name                        VARCHAR(500)                NOT NULL,
    alternate_name              VARCHAR(500),
    description                 TEXT,
    place_type                  VARCHAR(32),
    place_type_custom           VARCHAR(128),
    address_street_address      VARCHAR(500),
    address_locality            VARCHAR(256),
    address_region              VARCHAR(256),
    address_country             VARCHAR(64),
    address_postal_code         VARCHAR(32),
    geo_latitude                DOUBLE PRECISION,
    geo_longitude               DOUBLE PRECISION,
    geo_elevation               DOUBLE PRECISION,
    telephone                   VARCHAR(64),
    fax_number                  VARCHAR(64),
    url                         VARCHAR(2048),
    global_location_number      VARCHAR(13),
    branch_code                 VARCHAR(128),
    contained_in_place          UUID REFERENCES places (id) ON DELETE SET NULL,
    is_accessible_for_free      BOOLEAN,
    public_access               BOOLEAN,
    smoking_allowed             BOOLEAN,
    maximum_attendee_capacity   INTEGER,
    is_deleted                  BOOLEAN                     NOT NULL DEFAULT FALSE,
    deleted_at                  TIMESTAMP WITH TIME ZONE,
    created_at                  TIMESTAMP WITH TIME ZONE    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at                  TIMESTAMP WITH TIME ZONE    NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_places_name               ON places (LOWER(name));
CREATE INDEX IF NOT EXISTS idx_places_gln                ON places (global_location_number);
CREATE INDEX IF NOT EXISTS idx_places_contained_in_place ON places (contained_in_place);
CREATE INDEX IF NOT EXISTS idx_places_is_deleted         ON places (is_deleted);
CREATE INDEX IF NOT EXISTS idx_places_created_at         ON places (created_at);
CREATE INDEX IF NOT EXISTS idx_places_geo                ON places (geo_latitude, geo_longitude);

CREATE TABLE IF NOT EXISTS place_keywords (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    place_id    UUID         NOT NULL REFERENCES places (id) ON DELETE CASCADE,
    keyword     VARCHAR(256) NOT NULL,
    position    INTEGER      NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_place_keywords_place ON place_keywords (place_id);

CREATE TABLE IF NOT EXISTS place_identifiers (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    place_id        UUID         NOT NULL REFERENCES places (id) ON DELETE CASCADE,
    identifier_type VARCHAR(32)  NOT NULL,
    custom_label    VARCHAR(128),
    value           VARCHAR(512) NOT NULL,
    position        INTEGER      NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_place_identifiers_place ON place_identifiers (place_id);
CREATE INDEX IF NOT EXISTS idx_place_identifiers_value ON place_identifiers (identifier_type, value);

CREATE TABLE IF NOT EXISTS place_amenity_features (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    place_id    UUID         NOT NULL REFERENCES places (id) ON DELETE CASCADE,
    name        VARCHAR(256) NOT NULL,
    value       VARCHAR(256),
    position    INTEGER      NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_place_amenity_features_place ON place_amenity_features (place_id);

CREATE TABLE IF NOT EXISTS place_opening_hours (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    place_id    UUID        NOT NULL REFERENCES places (id) ON DELETE CASCADE,
    day_of_week VARCHAR(16) NOT NULL,
    opens       VARCHAR(8)  NOT NULL,
    closes      VARCHAR(8)  NOT NULL,
    position    INTEGER     NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_place_opening_hours_place ON place_opening_hours (place_id);
