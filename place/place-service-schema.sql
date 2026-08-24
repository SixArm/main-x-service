-- ============================================================================
-- place-service-schema.sql
--
-- Fully-normalized relational schema for the Place Service
-- (schema.org/Place). Refactors the JSONB column model toward proper SQL:
--   * 0..1 nested objects (PostalAddress, GeoCoordinates) are FLATTENED onto
--     the `places` row with prefixed columns.
--   * repeating collections (keywords, identifiers, amenity_features,
--     opening_hours) become dedicated child tables with FK + ordering.
--   * the PlaceType enum's free-text `Other(String)` variant is captured via
--     a `place_type_custom` column; likewise PlaceIdentifier's Custom scheme.
--
-- JSONB retained ONLY for opaque snapshots (audit old/new values, merge
-- transferred_data).
--
-- Target: PostgreSQL 15+. Soft-delete via is_deleted / deleted_at.
-- ============================================================================

-- ----------------------------------------------------------------------------
-- Core entity (PostalAddress + GeoCoordinates flattened onto the row)
-- ----------------------------------------------------------------------------
CREATE TABLE places (
    id                          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name                        VARCHAR(500) NOT NULL,
    alternate_name              VARCHAR(500),
    description                 TEXT,
    -- PlaceType enum (local_business|civic_structure|administrative_area|
    -- landform|park|airport|hospital|school|library|museum|restaurant|
    -- hotel|other); place_type_custom holds the label for Other(_).
    place_type                  VARCHAR(32),
    place_type_custom           VARCHAR(128),
    -- PostalAddress (Option) flattened
    address_street_address      VARCHAR(500),
    address_locality            VARCHAR(256),
    address_region              VARCHAR(256),
    address_country             VARCHAR(64),
    address_postal_code         VARCHAR(32),
    -- GeoCoordinates (Option) flattened (WGS 84)
    geo_latitude_as_decimal_degrees                DOUBLE PRECISION,
    geo_longitude_as_decimal_degrees               DOUBLE PRECISION,
    geo_elevation_as_decimal_metres               DOUBLE PRECISION,
    -- Contact
    telephone                   VARCHAR(64),
    fax_number                  VARCHAR(64),
    url                         VARCHAR(2048),
    -- Identifiers / hierarchy
    global_location_number      VARCHAR(13),
    branch_code                 VARCHAR(128),
    contained_in_place          UUID REFERENCES places (id) ON DELETE SET NULL,
    -- Access / capacity flags
    is_accessible_for_free      BOOLEAN,
    public_access               BOOLEAN,
    smoking_allowed             BOOLEAN,
    maximum_attendee_capacity   INTEGER,
    -- Registry bookkeeping
    is_deleted                  BOOLEAN     NOT NULL DEFAULT FALSE,
    deleted_at                  TIMESTAMP WITH TIME ZONE,
    created_at                  TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at                  TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT chk_places_latitude  CHECK (geo_latitude_as_decimal_degrees  IS NULL OR (geo_latitude_as_decimal_degrees  BETWEEN -90  AND 90)),
    CONSTRAINT chk_places_longitude CHECK (geo_longitude_as_decimal_degrees IS NULL OR (geo_longitude_as_decimal_degrees BETWEEN -180 AND 180))
);
CREATE INDEX idx_places_name               ON places (LOWER(name));
CREATE INDEX idx_places_gln                ON places (global_location_number);
CREATE INDEX idx_places_contained_in_place ON places (contained_in_place);
CREATE INDEX idx_places_is_deleted         ON places (is_deleted);
CREATE INDEX idx_places_created_at         ON places (created_at);
CREATE INDEX idx_places_locality           ON places (LOWER(address_locality));
CREATE INDEX idx_places_geo                ON places (geo_latitude_as_decimal_degrees, geo_longitude_as_decimal_degrees);

-- ----------------------------------------------------------------------------
-- Keywords: Vec<String>
-- ----------------------------------------------------------------------------
CREATE TABLE place_keywords (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    place_id    UUID         NOT NULL REFERENCES places (id) ON DELETE CASCADE,
    keyword     VARCHAR(256) NOT NULL,
    position    INTEGER      NOT NULL DEFAULT 0
);
CREATE INDEX idx_place_keywords_place ON place_keywords (place_id);

-- ----------------------------------------------------------------------------
-- Identifiers: Vec<PlaceIdentifier>
--   identifier_type: global_location_number|branch_code|fips|gnis|
--                    open_street_map|custom
--   custom_label: free-text label for IdentifierType::Custom(_)
-- ----------------------------------------------------------------------------
CREATE TABLE place_identifiers (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    place_id        UUID         NOT NULL REFERENCES places (id) ON DELETE CASCADE,
    identifier_type VARCHAR(32)  NOT NULL,
    custom_label    VARCHAR(128),
    value           VARCHAR(512) NOT NULL,
    position        INTEGER      NOT NULL DEFAULT 0
);
CREATE INDEX idx_place_identifiers_place ON place_identifiers (place_id);
CREATE INDEX idx_place_identifiers_value ON place_identifiers (identifier_type, value);

-- ----------------------------------------------------------------------------
-- Amenity features: Vec<AmenityFeature>
-- ----------------------------------------------------------------------------
CREATE TABLE place_amenity_features (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    place_id    UUID         NOT NULL REFERENCES places (id) ON DELETE CASCADE,
    name        VARCHAR(256) NOT NULL,
    value       VARCHAR(256),
    position    INTEGER      NOT NULL DEFAULT 0
);
CREATE INDEX idx_place_amenity_features_place ON place_amenity_features (place_id);

-- ----------------------------------------------------------------------------
-- Opening hours: Vec<OpeningHoursSpecification>
--   day_of_week: monday|tuesday|wednesday|thursday|friday|saturday|sunday
--   opens / closes: "HH:MM"
-- ----------------------------------------------------------------------------
CREATE TABLE place_opening_hours (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    place_id    UUID        NOT NULL REFERENCES places (id) ON DELETE CASCADE,
    day_of_week VARCHAR(16) NOT NULL,
    opens       VARCHAR(8)  NOT NULL,
    closes      VARCHAR(8)  NOT NULL,
    position    INTEGER     NOT NULL DEFAULT 0,
    CONSTRAINT chk_place_opening_hours_day CHECK (day_of_week IN (
        'monday','tuesday','wednesday','thursday','friday','saturday','sunday'
    ))
);
CREATE INDEX idx_place_opening_hours_place ON place_opening_hours (place_id);

-- ----------------------------------------------------------------------------
-- Consents
-- ----------------------------------------------------------------------------
CREATE TABLE place_consents (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    place_id        UUID        NOT NULL REFERENCES places (id) ON DELETE CASCADE,
    consent_type    VARCHAR(32) NOT NULL,  -- data_processing|data_sharing|marketing|research
    status          VARCHAR(16) NOT NULL,  -- active|revoked|expired
    granted_at      TIMESTAMP WITH TIME ZONE NOT NULL,
    expires_at      TIMESTAMP WITH TIME ZONE,
    CONSTRAINT chk_place_consents_type CHECK (
        consent_type IN ('data_processing','data_sharing','marketing','research')
    ),
    CONSTRAINT chk_place_consents_status CHECK (status IN ('active','revoked','expired'))
);
CREATE INDEX idx_place_consents_place ON place_consents (place_id);

-- ----------------------------------------------------------------------------
-- Merge history (transferred_data is an opaque snapshot → JSONB by design)
-- ----------------------------------------------------------------------------
CREATE TABLE place_merge_records (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    main_place_id       UUID NOT NULL,
    duplicate_place_id  UUID NOT NULL,
    merge_reason        TEXT,
    transferred_data    JSONB,
    merged_at           TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_place_merge_main ON place_merge_records (main_place_id);

-- ----------------------------------------------------------------------------
-- Audit log (old/new values are opaque snapshots → JSONB by design)
-- ----------------------------------------------------------------------------
CREATE TABLE audit_log (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_type         VARCHAR(64)              NOT NULL,
    entity_id           UUID                     NOT NULL,
    action              VARCHAR(16)              NOT NULL,
    user_id             VARCHAR(256),
    user_ip_address     VARCHAR(64),
    user_agent          VARCHAR(512),
    old_values          JSONB,
    new_values          JSONB,
    created_at          TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_audit_log_entity_id  ON audit_log (entity_id);
CREATE INDEX idx_audit_log_created_at ON audit_log (created_at);
