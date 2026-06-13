-- ============================================================================
-- worker-service-schema.sql
--
-- Fully-normalized relational schema for the Worker Service (workforce /
-- professional identity registry). Mirrors the Person schema and adds a
-- `worker_type` plus the UK ODS-extended Organization model (roles,
-- relationships, successions, periods). Worker child collections that were
-- JSONB (documents, emergency_contacts, photo) become dedicated tables;
-- within-name word lists (given/prefix/suffix) use native TEXT[] arrays.
--
-- JSONB retained ONLY for opaque snapshots (audit) and the match-score
-- component breakdown.
--
-- Target: PostgreSQL 15+.
-- ============================================================================

-- ----------------------------------------------------------------------------
-- Organizations (ODS-extended)
-- ----------------------------------------------------------------------------
CREATE TABLE organizations (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    active              BOOLEAN      NOT NULL DEFAULT TRUE,
    name                VARCHAR(500) NOT NULL,
    alias               TEXT[]       NOT NULL DEFAULT '{}',
    org_type            TEXT[]       NOT NULL DEFAULT '{}',
    part_of             UUID REFERENCES organizations (id) ON DELETE SET NULL,
    -- ODS extensions
    ods_code            VARCHAR(32),
    ods_status          VARCHAR(16),   -- active|inactive
    record_class        VARCHAR(32),
    record_use_type     VARCHAR(32),
    assigning_authority VARCHAR(256),
    last_change_date    DATE,
    created_at          TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at          TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_organizations_name     ON organizations (LOWER(name));
CREATE INDEX idx_organizations_ods_code ON organizations (ods_code);

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

-- ODS operational periods directly on the organization
CREATE TABLE organization_periods (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID        NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    period_type     VARCHAR(16) NOT NULL,  -- operational|legal
    start_date      DATE,
    end_date        DATE,
    position        INTEGER     NOT NULL DEFAULT 0
);
CREATE INDEX idx_org_periods_org ON organization_periods (organization_id);

-- ODS roles (each with its own periods)
CREATE TABLE organization_roles (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID         NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    unique_role_id  BIGINT       NOT NULL,
    role_code       VARCHAR(32)  NOT NULL,
    role_name       VARCHAR(256),
    is_primary      BOOLEAN      NOT NULL DEFAULT FALSE,
    status          VARCHAR(16)  NOT NULL,  -- active|inactive
    position        INTEGER      NOT NULL DEFAULT 0
);
CREATE INDEX idx_org_roles_org ON organization_roles (organization_id);

CREATE TABLE organization_role_periods (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    role_id     UUID        NOT NULL REFERENCES organization_roles (id) ON DELETE CASCADE,
    period_type VARCHAR(16) NOT NULL,
    start_date  DATE,
    end_date    DATE,
    position    INTEGER     NOT NULL DEFAULT 0
);
CREATE INDEX idx_org_role_periods_role ON organization_role_periods (role_id);

-- ODS relationships (each with its own periods)
CREATE TABLE organization_relationships (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id         UUID         NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    unique_rel_id           BIGINT       NOT NULL,
    relationship_type_code  VARCHAR(32)  NOT NULL,
    relationship_type_name  VARCHAR(256),
    status                  VARCHAR(16)  NOT NULL,
    target_ods_code         VARCHAR(32)  NOT NULL,
    target_primary_role_id  VARCHAR(64),
    position                INTEGER      NOT NULL DEFAULT 0
);
CREATE INDEX idx_org_relationships_org ON organization_relationships (organization_id);

CREATE TABLE organization_relationship_periods (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    relationship_id UUID        NOT NULL REFERENCES organization_relationships (id) ON DELETE CASCADE,
    period_type     VARCHAR(16) NOT NULL,
    start_date      DATE,
    end_date        DATE,
    position        INTEGER     NOT NULL DEFAULT 0
);
CREATE INDEX idx_org_rel_periods_rel ON organization_relationship_periods (relationship_id);

-- ODS successions
CREATE TABLE organization_successions (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id         UUID         NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    unique_succ_id          BIGINT       NOT NULL,
    succession_type         VARCHAR(16)  NOT NULL,  -- predecessor|successor
    target_ods_code         VARCHAR(32)  NOT NULL,
    target_primary_role_id  VARCHAR(64),
    legal_start_date        DATE,
    has_forward_succession  BOOLEAN      NOT NULL DEFAULT FALSE,
    position                INTEGER      NOT NULL DEFAULT 0
);
CREATE INDEX idx_org_successions_org ON organization_successions (organization_id);

-- ----------------------------------------------------------------------------
-- Core entity
-- ----------------------------------------------------------------------------
CREATE TABLE workers (
    id                          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    active                      BOOLEAN     NOT NULL DEFAULT TRUE,
    gender                      VARCHAR(16) NOT NULL,
    worker_type                 VARCHAR(32),
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
    CONSTRAINT chk_workers_gender CHECK (gender IN ('male','female','other','unknown'))
);
CREATE INDEX idx_workers_birth_date ON workers (birth_date);
CREATE INDEX idx_workers_tax_id     ON workers (tax_id);
CREATE INDEX idx_workers_deleted_at ON workers (deleted_at);

-- ----------------------------------------------------------------------------
-- Names (given/prefix/suffix native TEXT[])
-- ----------------------------------------------------------------------------
CREATE TABLE worker_names (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    worker_id   UUID         NOT NULL REFERENCES workers (id) ON DELETE CASCADE,
    use_type    VARCHAR(16),
    family      VARCHAR(256) NOT NULL,
    given       TEXT[]       NOT NULL DEFAULT '{}',
    prefix      TEXT[]       NOT NULL DEFAULT '{}',
    suffix      TEXT[]       NOT NULL DEFAULT '{}',
    is_primary  BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_worker_names_worker ON worker_names (worker_id);
CREATE INDEX idx_worker_names_family ON worker_names (LOWER(family));

-- ----------------------------------------------------------------------------
-- Identifiers
-- ----------------------------------------------------------------------------
CREATE TABLE worker_identifiers (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    worker_id       UUID         NOT NULL REFERENCES workers (id) ON DELETE CASCADE,
    use_type        VARCHAR(32),
    identifier_type VARCHAR(32)  NOT NULL,
    system          VARCHAR(512) NOT NULL,
    value           VARCHAR(512) NOT NULL,
    assigner        VARCHAR(500),
    created_at      TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_worker_identifiers_worker ON worker_identifiers (worker_id);
CREATE INDEX idx_worker_identifiers_value  ON worker_identifiers (identifier_type, system, value);

-- ----------------------------------------------------------------------------
-- Addresses / contacts
-- ----------------------------------------------------------------------------
CREATE TABLE worker_addresses (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    worker_id   UUID NOT NULL REFERENCES workers (id) ON DELETE CASCADE,
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
CREATE INDEX idx_worker_addresses_worker ON worker_addresses (worker_id);

CREATE TABLE worker_contacts (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    worker_id   UUID         NOT NULL REFERENCES workers (id) ON DELETE CASCADE,
    system      VARCHAR(16)  NOT NULL,
    value       VARCHAR(512) NOT NULL,
    use_type    VARCHAR(16),
    is_primary  BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_worker_contacts_worker ON worker_contacts (worker_id);

-- ----------------------------------------------------------------------------
-- Identity documents  (was JSONB → child table)
-- ----------------------------------------------------------------------------
CREATE TABLE worker_documents (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    worker_id           UUID         NOT NULL REFERENCES workers (id) ON DELETE CASCADE,
    document_type       VARCHAR(32)  NOT NULL,
    number              VARCHAR(128) NOT NULL,
    issuing_country     VARCHAR(64),
    issuing_authority   VARCHAR(500),
    issue_date          DATE,
    expiry_date         DATE,
    verified            BOOLEAN      NOT NULL DEFAULT FALSE,
    position            INTEGER      NOT NULL DEFAULT 0
);
CREATE INDEX idx_worker_documents_worker ON worker_documents (worker_id);
CREATE INDEX idx_worker_documents_number ON worker_documents (document_type, number);

-- ----------------------------------------------------------------------------
-- Emergency contacts  (was JSONB → child table) + telecom grandchild
-- ----------------------------------------------------------------------------
CREATE TABLE worker_emergency_contacts (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    worker_id           UUID         NOT NULL REFERENCES workers (id) ON DELETE CASCADE,
    name                VARCHAR(500) NOT NULL,
    relationship        VARCHAR(128) NOT NULL,
    is_primary          BOOLEAN      NOT NULL DEFAULT FALSE,
    address_use_type    VARCHAR(16),
    address_line1       VARCHAR(500),
    address_line2       VARCHAR(500),
    address_city        VARCHAR(256),
    address_state       VARCHAR(256),
    address_postal_code VARCHAR(32),
    address_country     VARCHAR(64),
    position            INTEGER      NOT NULL DEFAULT 0
);
CREATE INDEX idx_worker_emergency_contacts_worker ON worker_emergency_contacts (worker_id);

CREATE TABLE worker_emergency_contact_telecom (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    emergency_contact_id    UUID         NOT NULL REFERENCES worker_emergency_contacts (id) ON DELETE CASCADE,
    system                  VARCHAR(16)  NOT NULL,
    value                   VARCHAR(512) NOT NULL,
    use_type                VARCHAR(16),
    position                INTEGER      NOT NULL DEFAULT 0
);
CREATE INDEX idx_worker_emergency_telecom_contact ON worker_emergency_contact_telecom (emergency_contact_id);

-- ----------------------------------------------------------------------------
-- Photos  (was JSONB → child table)
-- ----------------------------------------------------------------------------
CREATE TABLE worker_photos (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    worker_id   UUID          NOT NULL REFERENCES workers (id) ON DELETE CASCADE,
    url         VARCHAR(2048) NOT NULL,
    position    INTEGER       NOT NULL DEFAULT 0
);
CREATE INDEX idx_worker_photos_worker ON worker_photos (worker_id);

-- ----------------------------------------------------------------------------
-- Worker-to-worker links
-- ----------------------------------------------------------------------------
CREATE TABLE worker_links (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    worker_id       UUID         NOT NULL REFERENCES workers (id) ON DELETE CASCADE,
    other_worker_id UUID         NOT NULL,
    link_type       VARCHAR(16)  NOT NULL,
    created_at      TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by      VARCHAR(256)
);
CREATE INDEX idx_worker_links_worker ON worker_links (worker_id);

-- ----------------------------------------------------------------------------
-- Consents
-- ----------------------------------------------------------------------------
CREATE TABLE worker_consents (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    worker_id       UUID        NOT NULL REFERENCES workers (id) ON DELETE CASCADE,
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
CREATE INDEX idx_worker_consents_worker ON worker_consents (worker_id);

-- ----------------------------------------------------------------------------
-- Match-score history
-- ----------------------------------------------------------------------------
CREATE TABLE worker_match_scores (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    worker_id           UUID NOT NULL REFERENCES workers (id) ON DELETE CASCADE,
    candidate_id        UUID NOT NULL,
    total_score         NUMERIC(10,6) NOT NULL,
    name_score          NUMERIC(10,6),
    birth_date_score    NUMERIC(10,6),
    gender_score        NUMERIC(10,6),
    address_score       NUMERIC(10,6),
    identifier_score    NUMERIC(10,6),
    calculated_at       TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_worker_match_scores_worker ON worker_match_scores (worker_id);

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
CREATE INDEX idx_audit_log_entity    ON audit_log (entity_type, entity_id);
CREATE INDEX idx_audit_log_timestamp ON audit_log (timestamp);
