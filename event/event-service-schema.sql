-- ============================================================================
-- event-service-schema.sql
--
-- Fully-normalized relational schema for the Event Service
-- (schema.org/Event). JSONB collection columns are refactored into child
-- tables. The polymorphic `location` (Place | PostalAddress | Virtual |
-- Text) is modeled as a single table with a `kind` discriminator and
-- nullable per-variant columns; the multi-role party lists
-- (organizers/performers/attendees/sponsors/funders/contributors) are a
-- single table tagged by `role`.
--
-- JSONB retained ONLY for opaque snapshots (audit, merge, review-queue).
--
-- Target: PostgreSQL 15+.
-- ============================================================================

-- ----------------------------------------------------------------------------
-- Organizations
-- ----------------------------------------------------------------------------
CREATE TABLE organizations (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    active      BOOLEAN      NOT NULL DEFAULT TRUE,
    name        VARCHAR(500) NOT NULL,
    part_of     UUID,
    created_at  TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_organizations_name ON organizations (LOWER(name));

CREATE TABLE organization_text_values (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID         NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    field           VARCHAR(16)  NOT NULL,  -- org_type|alias
    value           VARCHAR(500) NOT NULL,
    position        INTEGER      NOT NULL DEFAULT 0,
    CONSTRAINT chk_org_text_field CHECK (field IN ('org_type','alias'))
);
CREATE INDEX idx_org_text_values_org ON organization_text_values (organization_id);

CREATE TABLE organization_identifiers (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID         NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    use_type        VARCHAR(32),
    identifier_type VARCHAR(32)  NOT NULL,
    system          VARCHAR(512) NOT NULL,
    value           VARCHAR(512) NOT NULL,
    assigner        VARCHAR(500),
    position        INTEGER      NOT NULL DEFAULT 0
);
CREATE INDEX idx_org_identifiers_org ON organization_identifiers (organization_id);

CREATE TABLE organization_addresses (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    use_type        VARCHAR(32),
    city            VARCHAR(256),
    state           VARCHAR(256),
    postal_code     VARCHAR(32),
    country         VARCHAR(64),
    position        INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_org_addresses_org ON organization_addresses (organization_id);

CREATE TABLE organization_contacts (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID         NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    system          VARCHAR(16)  NOT NULL,  -- phone|fax|email|pager|url|sms|other
    value           VARCHAR(512) NOT NULL,
    use_type        VARCHAR(16),
    position        INTEGER      NOT NULL DEFAULT 0
);
CREATE INDEX idx_org_contacts_org ON organization_contacts (organization_id);

-- ----------------------------------------------------------------------------
-- Core entity
-- ----------------------------------------------------------------------------
CREATE TABLE events (
    id                                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    active                              BOOLEAN     NOT NULL DEFAULT TRUE,
    name                                VARCHAR(500) NOT NULL,
    description                         TEXT,
    disambiguating_description          TEXT,
    url                                 VARCHAR(2048),
    -- Time window
    start_date                          TIMESTAMP WITH TIME ZONE NOT NULL,
    end_date                            TIMESTAMP WITH TIME ZONE,
    door_time                           TIMESTAMP WITH TIME ZONE,
    duration                            VARCHAR(64),  -- ISO 8601 duration
    previous_start_date                 TIMESTAMP WITH TIME ZONE,
    time_zone                           VARCHAR(64),  -- IANA tz (storage is UTC)
    all_day                             BOOLEAN     NOT NULL DEFAULT FALSE,
    -- Status / classification
    event_status                        VARCHAR(32) NOT NULL,
    event_attendance_mode               VARCHAR(16) NOT NULL,
    event_type                          VARCHAR(32) NOT NULL,
    -- Audience / capacity
    typical_age_range                   VARCHAR(64),
    is_accessible_for_free              BOOLEAN,
    maximum_attendee_capacity           INTEGER,
    maximum_physical_attendee_capacity  INTEGER,
    maximum_virtual_attendee_capacity   INTEGER,
    remaining_attendee_capacity         INTEGER,
    -- Hierarchy
    super_event                         UUID,
    -- Registry bookkeeping
    deleted_at                          TIMESTAMP WITH TIME ZONE,
    created_at                          TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at                          TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_events_name       ON events (LOWER(name));
CREATE INDEX idx_events_start_date ON events (start_date);
CREATE INDEX idx_events_status     ON events (event_status);
CREATE INDEX idx_events_deleted_at ON events (deleted_at);

-- ----------------------------------------------------------------------------
-- Event string-list properties (tagged single table)
-- ----------------------------------------------------------------------------
CREATE TABLE event_text_values (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id    UUID          NOT NULL REFERENCES events (id) ON DELETE CASCADE,
    field       VARCHAR(16)   NOT NULL,
    value       VARCHAR(2048) NOT NULL,
    position    INTEGER       NOT NULL DEFAULT 0,
    CONSTRAINT chk_event_text_field CHECK (field IN (
        'alternate_name','image','same_as','keyword','in_language'
    ))
);
CREATE INDEX idx_event_text_values_event ON event_text_values (event_id, field);

-- ----------------------------------------------------------------------------
-- Event identifiers
-- ----------------------------------------------------------------------------
CREATE TABLE event_identifiers (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id        UUID         NOT NULL REFERENCES events (id) ON DELETE CASCADE,
    use_type        VARCHAR(32),
    identifier_type VARCHAR(32)  NOT NULL,  -- booking_number|confirmation_code|ticket_number|...
    system          VARCHAR(512) NOT NULL,
    value           VARCHAR(512) NOT NULL,
    assigner        VARCHAR(500),
    position        INTEGER      NOT NULL DEFAULT 0
);
CREATE INDEX idx_event_identifiers_event ON event_identifiers (event_id);
CREATE INDEX idx_event_identifiers_value ON event_identifiers (identifier_type, system, value);

-- ----------------------------------------------------------------------------
-- Event locations (polymorphic: Place | PostalAddress | Virtual | Text)
-- ----------------------------------------------------------------------------
CREATE TABLE event_locations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id        UUID        NOT NULL REFERENCES events (id) ON DELETE CASCADE,
    kind            VARCHAR(16) NOT NULL,  -- place|postal_address|virtual|text
    position        INTEGER     NOT NULL DEFAULT 0,
    -- Place: external place-service reference + venue details
    place_ref_id    UUID,
    name            VARCHAR(500),
    latitude        DOUBLE PRECISION,
    longitude       DOUBLE PRECISION,
    url             VARCHAR(2048),
    -- Place / PostalAddress: address fields
    address_line1   VARCHAR(500),
    address_line2   VARCHAR(500),
    city            VARCHAR(256),
    state           VARCHAR(256),
    postal_code     VARCHAR(32),
    country         VARCHAR(64),
    -- Text: free-text fallback value
    text_value      VARCHAR(1000),
    CONSTRAINT chk_event_locations_kind CHECK (
        kind IN ('place','postal_address','virtual','text')
    )
);
CREATE INDEX idx_event_locations_event ON event_locations (event_id);

-- ----------------------------------------------------------------------------
-- Event parties (organizer/performer/attendee/sponsor/funder/contributor)
-- ----------------------------------------------------------------------------
CREATE TABLE event_parties (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id        UUID         NOT NULL REFERENCES events (id) ON DELETE CASCADE,
    role            VARCHAR(16)  NOT NULL,  -- organizer|performer|attendee|sponsor|funder|contributor
    party_kind      VARCHAR(16)  NOT NULL,  -- person|organization
    party_ref_id    UUID,
    name            VARCHAR(500) NOT NULL,
    email           VARCHAR(320),
    url             VARCHAR(2048),
    position        INTEGER      NOT NULL DEFAULT 0,
    CONSTRAINT chk_event_parties_role CHECK (
        role IN ('organizer','performer','attendee','sponsor','funder','contributor')
    ),
    CONSTRAINT chk_event_parties_kind CHECK (party_kind IN ('person','organization'))
);
CREATE INDEX idx_event_parties_event ON event_parties (event_id, role);

-- ----------------------------------------------------------------------------
-- Event references (about / workFeatured)
-- ----------------------------------------------------------------------------
CREATE TABLE event_references (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id    UUID         NOT NULL REFERENCES events (id) ON DELETE CASCADE,
    role        VARCHAR(16)  NOT NULL,  -- about|works
    ref_id      UUID,
    name        VARCHAR(500) NOT NULL,
    url         VARCHAR(2048),
    kind        VARCHAR(64),
    position    INTEGER      NOT NULL DEFAULT 0,
    CONSTRAINT chk_event_references_role CHECK (role IN ('about','works'))
);
CREATE INDEX idx_event_references_event ON event_references (event_id);

-- ----------------------------------------------------------------------------
-- Event sub-events (Vec<Uuid>)
-- ----------------------------------------------------------------------------
CREATE TABLE event_sub_events (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id        UUID    NOT NULL REFERENCES events (id) ON DELETE CASCADE,
    sub_event_id    UUID    NOT NULL,
    position        INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_event_sub_events_event ON event_sub_events (event_id);

-- ----------------------------------------------------------------------------
-- Event offers (ticket / pricing tiers)
-- ----------------------------------------------------------------------------
CREATE TABLE event_offers (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id        UUID NOT NULL REFERENCES events (id) ON DELETE CASCADE,
    name            VARCHAR(500),
    price           VARCHAR(64),   -- decimal as string
    price_currency  VARCHAR(3),    -- ISO 4217
    url             VARCHAR(2048),
    availability    VARCHAR(16),   -- in_stock|sold_out|pre_order|out_of_stock|discontinued
    valid_from      TIMESTAMP WITH TIME ZONE,
    valid_through   TIMESTAMP WITH TIME ZONE,
    position        INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_event_offers_event ON event_offers (event_id);

-- ----------------------------------------------------------------------------
-- Event links (event-to-event relationships)
-- ----------------------------------------------------------------------------
CREATE TABLE event_links (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id        UUID        NOT NULL REFERENCES events (id) ON DELETE CASCADE,
    other_event_id  UUID        NOT NULL,
    link_type       VARCHAR(16) NOT NULL,  -- replaced_by|replaces|refer|see_also
    position        INTEGER     NOT NULL DEFAULT 0
);
CREATE INDEX idx_event_links_event ON event_links (event_id);

-- ----------------------------------------------------------------------------
-- Consents (dates are calendar DATEs)
-- ----------------------------------------------------------------------------
CREATE TABLE event_consents (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id        UUID        NOT NULL REFERENCES events (id) ON DELETE CASCADE,
    consent_type    VARCHAR(32) NOT NULL,
    status          VARCHAR(16) NOT NULL,
    granted_date    DATE        NOT NULL,
    expiry_date     DATE,
    revoked_date    DATE,
    purpose         VARCHAR(500),
    method          VARCHAR(64),
    created_at      TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_event_consents_event ON event_consents (event_id);

-- ----------------------------------------------------------------------------
-- Merge history / review queue / audit (opaque snapshots → JSONB by design)
-- ----------------------------------------------------------------------------
CREATE TABLE event_merge_records (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    main_event_id       UUID         NOT NULL,
    duplicate_event_id  UUID         NOT NULL,
    status              VARCHAR(16)  NOT NULL,
    merged_by           VARCHAR(256),
    merge_reason        TEXT,
    match_score         DOUBLE PRECISION,
    transferred_data    JSONB,
    merged_at           TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_event_merge_main ON event_merge_records (main_event_id);

CREATE TABLE event_review_queue (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id_a          UUID         NOT NULL,
    event_id_b          UUID         NOT NULL,
    match_score         DOUBLE PRECISION NOT NULL,
    match_quality       VARCHAR(32)  NOT NULL,
    detection_method    VARCHAR(64)  NOT NULL,
    score_breakdown     JSONB,
    status              VARCHAR(16)  NOT NULL,
    reviewed_by         VARCHAR(256),
    created_at          TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    reviewed_at         TIMESTAMP WITH TIME ZONE
);
CREATE INDEX idx_event_review_status ON event_review_queue (status);

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
