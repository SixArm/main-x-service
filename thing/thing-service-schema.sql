-- ============================================================================
-- thing-service-schema.sql
--
-- Fully-normalized relational schema for the Thing Service
-- (schema.org/Thing). This refactors the data model away from JSONB
-- collection columns toward proper child tables: every repeating domain
-- collection (alternate_names, identifiers, images, same_as) becomes its
-- own table with a foreign key back to `things` and an explicit ordering
-- column.
--
-- JSONB is retained ONLY for genuinely opaque payloads that are not
-- structured domain data: audit before/after snapshots and the merge
-- transferred-data snapshot.
--
-- Target: PostgreSQL 15+. UUID primary keys are application- or
-- server-generated (gen_random_uuid()). Soft-delete is via
-- `is_deleted` / `deleted_at`; rows are never hard-deleted.
-- ============================================================================

-- ----------------------------------------------------------------------------
-- Core entity
-- ----------------------------------------------------------------------------
CREATE TABLE things (
    id                          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name                        VARCHAR(500)                NOT NULL,
    description                 TEXT,
    disambiguating_description  TEXT,
    additional_type             VARCHAR(2048),  -- schema.org additionalType URL
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

CREATE INDEX idx_things_name       ON things (LOWER(name));
CREATE INDEX idx_things_is_deleted ON things (is_deleted);
CREATE INDEX idx_things_created_at ON things (created_at);

-- ----------------------------------------------------------------------------
-- alternate_names: Vec<String>  (schema.org alternateName)
-- ----------------------------------------------------------------------------
CREATE TABLE thing_alternate_names (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    thing_id    UUID         NOT NULL REFERENCES things (id) ON DELETE CASCADE,
    name        VARCHAR(500) NOT NULL,
    position    INTEGER      NOT NULL DEFAULT 0
);

CREATE INDEX idx_thing_alternate_names_thing ON thing_alternate_names (thing_id);

-- ----------------------------------------------------------------------------
-- identifiers: Vec<ThingIdentifier>  (schema.org PropertyValue shape)
--   property_id: IdentifierType enum
--     (doi|isbn|issn|gtin|sku|mpn|serial_number|uri|uuid|custom)
--   custom_label: holds the free-text label for IdentifierType::Custom(_)
-- ----------------------------------------------------------------------------
CREATE TABLE thing_identifiers (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    thing_id        UUID         NOT NULL REFERENCES things (id) ON DELETE CASCADE,
    property_id     VARCHAR(32)  NOT NULL,
    custom_label    VARCHAR(128),
    value           VARCHAR(512) NOT NULL,
    name            VARCHAR(500),
    url             VARCHAR(2048),
    position        INTEGER      NOT NULL DEFAULT 0,
    CONSTRAINT chk_thing_identifiers_property_id CHECK (
        property_id IN ('doi','isbn','issn','gtin','sku','mpn',
                        'serial_number','uri','uuid','custom')
    )
);

CREATE INDEX idx_thing_identifiers_thing     ON thing_identifiers (thing_id);
CREATE INDEX idx_thing_identifiers_value     ON thing_identifiers (property_id, value);

-- ----------------------------------------------------------------------------
-- images: Vec<String>  (schema.org image)
-- ----------------------------------------------------------------------------
CREATE TABLE thing_images (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    thing_id    UUID          NOT NULL REFERENCES things (id) ON DELETE CASCADE,
    url         VARCHAR(2048) NOT NULL,
    position    INTEGER       NOT NULL DEFAULT 0
);

CREATE INDEX idx_thing_images_thing ON thing_images (thing_id);

-- ----------------------------------------------------------------------------
-- same_as: Vec<String>  (schema.org sameAs — authoritative external URLs)
-- ----------------------------------------------------------------------------
CREATE TABLE thing_same_as (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    thing_id    UUID          NOT NULL REFERENCES things (id) ON DELETE CASCADE,
    url         VARCHAR(2048) NOT NULL,
    position    INTEGER       NOT NULL DEFAULT 0
);

CREATE INDEX idx_thing_same_as_thing ON thing_same_as (thing_id);

-- ----------------------------------------------------------------------------
-- consents: data-protection consent records
--   consent_type: data_processing|data_sharing|marketing|research
--   status:       active|revoked|expired
-- ----------------------------------------------------------------------------
CREATE TABLE thing_consents (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    thing_id        UUID        NOT NULL REFERENCES things (id) ON DELETE CASCADE,
    consent_type    VARCHAR(32) NOT NULL,
    status          VARCHAR(16) NOT NULL,
    granted_at      TIMESTAMP WITH TIME ZONE NOT NULL,
    expires_at      TIMESTAMP WITH TIME ZONE,
    CONSTRAINT chk_thing_consents_type CHECK (
        consent_type IN ('data_processing','data_sharing','marketing','research')
    ),
    CONSTRAINT chk_thing_consents_status CHECK (
        status IN ('active','revoked','expired')
    )
);

CREATE INDEX idx_thing_consents_thing ON thing_consents (thing_id);

-- ----------------------------------------------------------------------------
-- merge history (transferred_data is an opaque snapshot → JSONB by design)
-- ----------------------------------------------------------------------------
CREATE TABLE thing_merge_records (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    main_thing_id       UUID NOT NULL,
    duplicate_thing_id  UUID NOT NULL,
    merge_reason        TEXT,
    transferred_data    JSONB,
    merged_at           TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_thing_merge_main ON thing_merge_records (main_thing_id);

-- ----------------------------------------------------------------------------
-- audit log (old/new values are opaque snapshots → JSONB by design)
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
