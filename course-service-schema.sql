-- ============================================================================
-- course-service-schema.sql
--
-- Fully-normalized relational schema for the Course Service
-- (schema.org/Course + CourseInstance sub-resource). JSONB collection
-- columns are refactored into proper child tables.
--
-- Design notes:
--   * The Course aggregate has MANY parallel `Vec<String>` properties
--     (alternate_names, image, same_as, keywords, about, in_language,
--     teaches, assesses, competency_required, course_prerequisites,
--     available_language, financial_aid_eligible). Rather than a dozen
--     near-identical two-column tables, these are normalized into a single
--     tagged table `course_text_values(field, value, position)` with a
--     CHECK constraint enumerating the valid field names. Structured
--     sub-objects (identifiers, links, credentials, instances, syllabus)
--     get their own dedicated tables.
--   * JSONB is retained ONLY for opaque snapshots (audit old/new values,
--     merge transferred_data, review-queue score_breakdown).
--
-- Target: PostgreSQL 15+.
-- ============================================================================

-- ----------------------------------------------------------------------------
-- Providers (issuing organizations)
-- ----------------------------------------------------------------------------
CREATE TABLE providers (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        VARCHAR(500) NOT NULL,
    url         VARCHAR(2048),
    kind        VARCHAR(32),  -- college|university|school|company|nonprofit|government|platform|other
    created_at  TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_providers_name ON providers (LOWER(name));

CREATE TABLE provider_text_values (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider_id UUID         NOT NULL REFERENCES providers (id) ON DELETE CASCADE,
    field       VARCHAR(32)  NOT NULL,  -- alternate_name|same_as
    value       VARCHAR(2048) NOT NULL,
    position    INTEGER      NOT NULL DEFAULT 0,
    CONSTRAINT chk_provider_text_field CHECK (field IN ('alternate_name','same_as'))
);
CREATE INDEX idx_provider_text_values_provider ON provider_text_values (provider_id);

-- ----------------------------------------------------------------------------
-- Core entity
-- ----------------------------------------------------------------------------
CREATE TABLE courses (
    id                          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name                        VARCHAR(500)                NOT NULL,
    description                 TEXT,
    disambiguating_description  TEXT,
    url                         VARCHAR(2048),
    additional_type             VARCHAR(2048),
    active                      BOOLEAN                     NOT NULL DEFAULT TRUE,
    audience                    VARCHAR(500),
    license                     VARCHAR(2048),
    typical_age_range           VARCHAR(64),
    time_required               VARCHAR(64),   -- ISO 8601 duration
    version                     VARCHAR(64),
    is_accessible_for_free      BOOLEAN,
    educational_level           VARCHAR(32),   -- beginner|intermediate|advanced|expert|...
    educational_use             VARCHAR(128),
    learning_resource_type      VARCHAR(32),
    interactivity_type          VARCHAR(16),   -- active|expositive|mixed
    course_code                 VARCHAR(128),
    number_of_credits           INTEGER,
    total_historical_enrollment BIGINT,
    status                      VARCHAR(32)                 NOT NULL DEFAULT 'active',
    provider_id                 UUID REFERENCES providers (id) ON DELETE SET NULL,
    deleted_at                  TIMESTAMP WITH TIME ZONE,
    created_at                  TIMESTAMP WITH TIME ZONE    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at                  TIMESTAMP WITH TIME ZONE    NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_courses_name        ON courses (LOWER(name));
CREATE INDEX idx_courses_course_code ON courses (course_code);
CREATE INDEX idx_courses_provider    ON courses (provider_id);
CREATE INDEX idx_courses_deleted_at  ON courses (deleted_at);

-- ----------------------------------------------------------------------------
-- Course string-list properties (tagged single table)
-- ----------------------------------------------------------------------------
CREATE TABLE course_text_values (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    course_id   UUID          NOT NULL REFERENCES courses (id) ON DELETE CASCADE,
    field       VARCHAR(32)   NOT NULL,
    value       VARCHAR(2048) NOT NULL,
    position    INTEGER       NOT NULL DEFAULT 0,
    CONSTRAINT chk_course_text_field CHECK (field IN (
        'alternate_name','image','same_as','keyword','about','in_language',
        'teaches','assesses','competency_required','course_prerequisite',
        'available_language','financial_aid_eligible'
    ))
);
CREATE INDEX idx_course_text_values_course ON course_text_values (course_id, field);

-- ----------------------------------------------------------------------------
-- Course identifiers (PropertyValue shape)
-- ----------------------------------------------------------------------------
CREATE TABLE course_identifiers (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    course_id       UUID         NOT NULL REFERENCES courses (id) ON DELETE CASCADE,
    property_id     VARCHAR(32)  NOT NULL,
    custom_label    VARCHAR(128),
    value           VARCHAR(512) NOT NULL,
    name            VARCHAR(500),
    url             VARCHAR(2048),
    position        INTEGER      NOT NULL DEFAULT 0
);
CREATE INDEX idx_course_identifiers_course ON course_identifiers (course_id);
CREATE INDEX idx_course_identifiers_value  ON course_identifiers (property_id, value);

-- ----------------------------------------------------------------------------
-- Course links (course-to-course relationships)
-- ----------------------------------------------------------------------------
CREATE TABLE course_links (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    course_id       UUID         NOT NULL REFERENCES courses (id) ON DELETE CASCADE,
    other_course_id UUID         NOT NULL,
    link_type       VARCHAR(32)  NOT NULL,  -- replaced_by|replaces|refer|see_also
    position        INTEGER      NOT NULL DEFAULT 0
);
CREATE INDEX idx_course_links_course ON course_links (course_id);

-- ----------------------------------------------------------------------------
-- Course credentials (educational_credential_awarded / occupational_…)
--   role: educational|occupational  (0..1 of each per course)
-- ----------------------------------------------------------------------------
CREATE TABLE course_credentials (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    course_id           UUID         NOT NULL REFERENCES courses (id) ON DELETE CASCADE,
    role                VARCHAR(16)  NOT NULL,  -- educational|occupational
    name                VARCHAR(500) NOT NULL,
    category            VARCHAR(64),
    educational_level   VARCHAR(64),
    recognized_by       VARCHAR(500),
    url                 VARCHAR(2048),
    CONSTRAINT chk_course_credentials_role CHECK (role IN ('educational','occupational')),
    CONSTRAINT uq_course_credentials_role UNIQUE (course_id, role)
);
CREATE INDEX idx_course_credentials_course ON course_credentials (course_id);

-- ----------------------------------------------------------------------------
-- Syllabus sections (recursive: a section may have sub-sections)
-- ----------------------------------------------------------------------------
CREATE TABLE course_syllabus_sections (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    course_id           UUID         NOT NULL REFERENCES courses (id) ON DELETE CASCADE,
    parent_section_id   UUID REFERENCES course_syllabus_sections (id) ON DELETE CASCADE,
    name                VARCHAR(500) NOT NULL,
    description         TEXT,
    position            INTEGER      NOT NULL DEFAULT 0,
    time_required       VARCHAR(64)
);
CREATE INDEX idx_course_syllabus_course ON course_syllabus_sections (course_id);
CREATE INDEX idx_course_syllabus_parent ON course_syllabus_sections (parent_section_id);

CREATE TABLE course_syllabus_text_values (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    section_id  UUID          NOT NULL REFERENCES course_syllabus_sections (id) ON DELETE CASCADE,
    field       VARCHAR(16)   NOT NULL,  -- teaches|resource
    value       VARCHAR(2048) NOT NULL,
    position    INTEGER       NOT NULL DEFAULT 0,
    CONSTRAINT chk_course_syllabus_text_field CHECK (field IN ('teaches','resource'))
);
CREATE INDEX idx_course_syllabus_text_section ON course_syllabus_text_values (section_id);

-- ----------------------------------------------------------------------------
-- Course instances (sub-resource); Schedule flattened onto the row.
-- ----------------------------------------------------------------------------
CREATE TABLE course_instances (
    id                          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    course_id                   UUID NOT NULL REFERENCES courses (id) ON DELETE CASCADE,
    name                        VARCHAR(500),
    course_mode                 VARCHAR(16),   -- online|onsite|blended|hybrid
    status                      VARCHAR(32) NOT NULL,
    location                    VARCHAR(500),
    location_id                 UUID,
    maximum_attendee_capacity   INTEGER,
    enrolled_count              INTEGER,
    enrollment_opens            TIMESTAMP WITH TIME ZONE,
    enrollment_closes           TIMESTAMP WITH TIME ZONE,
    -- Schedule (Option<Schedule>) flattened:
    schedule_start_date         TIMESTAMP WITH TIME ZONE,
    schedule_end_date           TIMESTAMP WITH TIME ZONE,
    schedule_time_zone          VARCHAR(64),
    schedule_recurrence         VARCHAR(256),
    created_at                  TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at                  TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_course_instances_course ON course_instances (course_id);

CREATE TABLE course_instance_languages (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    instance_id UUID        NOT NULL REFERENCES course_instances (id) ON DELETE CASCADE,
    language    VARCHAR(16) NOT NULL,  -- ISO 639-1
    position    INTEGER     NOT NULL DEFAULT 0
);
CREATE INDEX idx_course_instance_languages_instance ON course_instance_languages (instance_id);

CREATE TABLE course_instance_instructors (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    instance_id     UUID         NOT NULL REFERENCES course_instances (id) ON DELETE CASCADE,
    instructor_id   UUID,
    instructor_name VARCHAR(500),
    position        INTEGER      NOT NULL DEFAULT 0
);
CREATE INDEX idx_course_instance_instructors_instance ON course_instance_instructors (instance_id);

CREATE TABLE course_instance_sessions (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    instance_id UUID NOT NULL REFERENCES course_instances (id) ON DELETE CASCADE,
    start_at    TIMESTAMP WITH TIME ZONE NOT NULL,
    end_at      TIMESTAMP WITH TIME ZONE,
    label       VARCHAR(256),
    position    INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_course_instance_sessions_instance ON course_instance_sessions (instance_id);

-- ----------------------------------------------------------------------------
-- Merge history (transferred_data is opaque → JSONB by design)
-- ----------------------------------------------------------------------------
CREATE TABLE course_merge_records (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    main_course_id      UUID         NOT NULL,
    duplicate_course_id UUID         NOT NULL,
    status              VARCHAR(16)  NOT NULL,  -- completed|reversed
    merged_by           VARCHAR(256),
    merge_reason        TEXT,
    match_score         DOUBLE PRECISION,
    transferred_data    JSONB,
    merged_at           TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_course_merge_main ON course_merge_records (main_course_id);

-- ----------------------------------------------------------------------------
-- Deduplication review queue (score_breakdown is opaque → JSONB by design)
-- ----------------------------------------------------------------------------
CREATE TABLE course_review_queue (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    course_id_a         UUID         NOT NULL,
    course_id_b         UUID         NOT NULL,
    match_score         DOUBLE PRECISION NOT NULL,
    match_quality       VARCHAR(32)  NOT NULL,
    detection_method    VARCHAR(64)  NOT NULL,
    score_breakdown     JSONB,
    status              VARCHAR(16)  NOT NULL,  -- pending|confirmed|rejected|auto_merged
    reviewed_by         VARCHAR(256),
    created_at          TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    reviewed_at         TIMESTAMP WITH TIME ZONE
);
CREATE INDEX idx_course_review_status ON course_review_queue (status);

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
