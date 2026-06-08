-- ============================================================================
-- person-service-schema.sql
--
-- Fully-normalized relational schema for the Person Service. Person child
-- collections that were JSONB (documents, emergency_contacts, photo) are
-- refactored into dedicated child tables. Small within-name word lists
-- (given / prefix / suffix) use native PostgreSQL TEXT[] arrays — these are
-- SQL, not JSONB.
--
-- JSONB retained ONLY for opaque snapshots (audit old/new values) and the
-- match-score component breakdown.
--
-- Target: PostgreSQL 15+. Soft-delete via deleted_at; never hard-deleted.
-- ============================================================================

-- ----------------------------------------------------------------------------
-- Organizations (managing / owning)
-- ----------------------------------------------------------------------------
CREATE TABLE organizations (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    active      BOOLEAN      NOT NULL DEFAULT TRUE,
    name        VARCHAR(500) NOT NULL,
    alias       TEXT[]       NOT NULL DEFAULT '{}',
    org_type    TEXT[]       NOT NULL DEFAULT '{}',
    part_of     UUID REFERENCES organizations (id) ON DELETE SET NULL,
    created_at  TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by  VARCHAR(256),
    updated_by  VARCHAR(256),
    deleted_at  TIMESTAMP WITH TIME ZONE,
    deleted_by  VARCHAR(256)
);
CREATE INDEX idx_organizations_name ON organizations (LOWER(name));

CREATE TABLE organization_identifiers (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID         NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    use_type        VARCHAR(32),
    identifier_type VARCHAR(32)  NOT NULL,
    system          VARCHAR(512) NOT NULL,
    value           VARCHAR(512) NOT NULL,
    assigner        VARCHAR(500),
    created_at      TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_org_identifiers_org ON organization_identifiers (organization_id);

CREATE TABLE organization_addresses (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    use_type        VARCHAR(32),
    line1           VARCHAR(500),
    line2           VARCHAR(500),
    city            VARCHAR(256),
    state           VARCHAR(256),
    postal_code     VARCHAR(32),
    country         VARCHAR(64),
    is_primary      BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_org_addresses_org ON organization_addresses (organization_id);

CREATE TABLE organization_contacts (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID         NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    system          VARCHAR(16)  NOT NULL,
    value           VARCHAR(512) NOT NULL,
    use_type        VARCHAR(16),
    is_primary      BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_org_contacts_org ON organization_contacts (organization_id);

-- ----------------------------------------------------------------------------
-- Core entity
-- ----------------------------------------------------------------------------
CREATE TABLE persons (
    id                          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    active                      BOOLEAN     NOT NULL DEFAULT TRUE,
    gender                      VARCHAR(16) NOT NULL,  -- male|female|other|unknown
    birth_date                  DATE,
    tax_id                      VARCHAR(64),
    deceased                    BOOLEAN     NOT NULL DEFAULT FALSE,
    deceased_datetime           TIMESTAMP WITH TIME ZONE,
    marital_status              VARCHAR(32),
    multiple_birth              BOOLEAN,
    managing_organization_id    UUID REFERENCES organizations (id) ON DELETE SET NULL,
    created_at                  TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at                  TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by                  VARCHAR(256),
    updated_by                  VARCHAR(256),
    deleted_at                  TIMESTAMP WITH TIME ZONE,
    deleted_by                  VARCHAR(256),
    CONSTRAINT chk_persons_gender CHECK (gender IN ('male','female','other','unknown'))
);
CREATE INDEX idx_persons_birth_date ON persons (birth_date);
CREATE INDEX idx_persons_tax_id     ON persons (tax_id);
CREATE INDEX idx_persons_deleted_at ON persons (deleted_at);

-- ----------------------------------------------------------------------------
-- Names (primary + additional). given/prefix/suffix are native TEXT[].
-- ----------------------------------------------------------------------------
CREATE TABLE person_names (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    person_id   UUID         NOT NULL REFERENCES persons (id) ON DELETE CASCADE,
    use_type    VARCHAR(16),
    family      VARCHAR(256) NOT NULL,
    given       TEXT[]       NOT NULL DEFAULT '{}',
    prefix      TEXT[]       NOT NULL DEFAULT '{}',
    suffix      TEXT[]       NOT NULL DEFAULT '{}',
    is_primary  BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_person_names_person ON person_names (person_id);
CREATE INDEX idx_person_names_family ON person_names (LOWER(family));

-- ----------------------------------------------------------------------------
-- Identifiers (MRN, SSN, national IDs, …)
-- ----------------------------------------------------------------------------
CREATE TABLE person_identifiers (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    person_id       UUID         NOT NULL REFERENCES persons (id) ON DELETE CASCADE,
    use_type        VARCHAR(32),
    identifier_type VARCHAR(32)  NOT NULL,  -- mrn|ssn|dl|npi|ppn|tax|other
    system          VARCHAR(512) NOT NULL,
    value           VARCHAR(512) NOT NULL,
    assigner        VARCHAR(500),
    created_at      TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_person_identifiers_person ON person_identifiers (person_id);
CREATE INDEX idx_person_identifiers_value  ON person_identifiers (identifier_type, system, value);

-- ----------------------------------------------------------------------------
-- Addresses
-- ----------------------------------------------------------------------------
CREATE TABLE person_addresses (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    person_id   UUID NOT NULL REFERENCES persons (id) ON DELETE CASCADE,
    use_type    VARCHAR(16),
    line1       VARCHAR(500),
    line2       VARCHAR(500),
    city        VARCHAR(256),
    state       VARCHAR(256),
    postal_code VARCHAR(32),
    country     VARCHAR(64),
    is_primary  BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_person_addresses_person ON person_addresses (person_id);

-- ----------------------------------------------------------------------------
-- Contact points (telecom)
-- ----------------------------------------------------------------------------
CREATE TABLE person_contacts (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    person_id   UUID         NOT NULL REFERENCES persons (id) ON DELETE CASCADE,
    system      VARCHAR(16)  NOT NULL,  -- phone|fax|email|pager|url|sms|other
    value       VARCHAR(512) NOT NULL,
    use_type    VARCHAR(16),
    is_primary  BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_person_contacts_person ON person_contacts (person_id);

-- ----------------------------------------------------------------------------
-- Identity documents  (was JSONB → now a child table)
-- ----------------------------------------------------------------------------
CREATE TABLE person_documents (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    person_id           UUID         NOT NULL REFERENCES persons (id) ON DELETE CASCADE,
    document_type       VARCHAR(32)  NOT NULL,  -- passport|birth_certificate|national_id|...
    number              VARCHAR(128) NOT NULL,
    issuing_country     VARCHAR(64),
    issuing_authority   VARCHAR(500),
    issue_date          DATE,
    expiry_date         DATE,
    verified            BOOLEAN      NOT NULL DEFAULT FALSE,
    position            INTEGER      NOT NULL DEFAULT 0
);
CREATE INDEX idx_person_documents_person ON person_documents (person_id);
CREATE INDEX idx_person_documents_number ON person_documents (document_type, number);

-- ----------------------------------------------------------------------------
-- Emergency contacts  (was JSONB → now a child table); address flattened,
-- telecom in its own grandchild table.
-- ----------------------------------------------------------------------------
CREATE TABLE person_emergency_contacts (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    person_id           UUID         NOT NULL REFERENCES persons (id) ON DELETE CASCADE,
    name                VARCHAR(500) NOT NULL,
    relationship        VARCHAR(128) NOT NULL,
    is_primary          BOOLEAN      NOT NULL DEFAULT FALSE,
    -- optional address, flattened
    address_use_type    VARCHAR(16),
    address_line1       VARCHAR(500),
    address_line2       VARCHAR(500),
    address_city        VARCHAR(256),
    address_state       VARCHAR(256),
    address_postal_code VARCHAR(32),
    address_country     VARCHAR(64),
    position            INTEGER      NOT NULL DEFAULT 0
);
CREATE INDEX idx_person_emergency_contacts_person ON person_emergency_contacts (person_id);

CREATE TABLE person_emergency_contact_telecom (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    emergency_contact_id    UUID         NOT NULL REFERENCES person_emergency_contacts (id) ON DELETE CASCADE,
    system                  VARCHAR(16)  NOT NULL,
    value                   VARCHAR(512) NOT NULL,
    use_type                VARCHAR(16),
    position                INTEGER      NOT NULL DEFAULT 0
);
CREATE INDEX idx_person_emergency_telecom_contact ON person_emergency_contact_telecom (emergency_contact_id);

-- ----------------------------------------------------------------------------
-- Photos  (was JSONB → now a child table)
-- ----------------------------------------------------------------------------
CREATE TABLE person_photos (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    person_id   UUID          NOT NULL REFERENCES persons (id) ON DELETE CASCADE,
    url         VARCHAR(2048) NOT NULL,
    position    INTEGER       NOT NULL DEFAULT 0
);
CREATE INDEX idx_person_photos_person ON person_photos (person_id);

-- ----------------------------------------------------------------------------
-- Person-to-person links
-- ----------------------------------------------------------------------------
CREATE TABLE person_links (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    person_id       UUID         NOT NULL REFERENCES persons (id) ON DELETE CASCADE,
    other_person_id UUID         NOT NULL,
    link_type       VARCHAR(16)  NOT NULL,  -- replaced_by|replaces|refer|see_also
    created_at      TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by      VARCHAR(256)
);
CREATE INDEX idx_person_links_person ON person_links (person_id);

-- ----------------------------------------------------------------------------
-- Consents
-- ----------------------------------------------------------------------------
CREATE TABLE person_consents (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    person_id       UUID        NOT NULL REFERENCES persons (id) ON DELETE CASCADE,
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
CREATE INDEX idx_person_consents_person ON person_consents (person_id);

-- ----------------------------------------------------------------------------
-- Match-score history (score_breakdown components as explicit columns)
-- ----------------------------------------------------------------------------
CREATE TABLE person_match_scores (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    person_id           UUID NOT NULL REFERENCES persons (id) ON DELETE CASCADE,
    candidate_id        UUID NOT NULL,
    total_score         NUMERIC(10,6) NOT NULL,
    name_score          NUMERIC(10,6),
    birth_date_score    NUMERIC(10,6),
    gender_score        NUMERIC(10,6),
    address_score       NUMERIC(10,6),
    identifier_score    NUMERIC(10,6),
    calculated_at       TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_person_match_scores_person ON person_match_scores (person_id);

-- ----------------------------------------------------------------------------
-- Audit log (old/new values are opaque snapshots → JSONB by design)
-- ----------------------------------------------------------------------------
CREATE TABLE audit_log (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    timestamp           TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    user_id             VARCHAR(256),
    action              VARCHAR(32)              NOT NULL,
    entity_type         VARCHAR(64)              NOT NULL,
    entity_id           UUID                     NOT NULL,
    old_values          JSONB,
    new_values          JSONB,
    ip_address          VARCHAR(64),
    user_agent          VARCHAR(512)
);
CREATE INDEX idx_audit_log_entity   ON audit_log (entity_type, entity_id);
CREATE INDEX idx_audit_log_timestamp ON audit_log (timestamp);
