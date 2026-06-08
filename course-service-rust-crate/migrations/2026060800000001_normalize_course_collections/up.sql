-- Normalize Course collections from JSONB columns to relational tables.

-- courses: drop the 12 string-list + 2 credential JSON columns; convert the
-- two enum columns from JSONB to bare VARCHAR.
ALTER TABLE courses
    DROP COLUMN IF EXISTS alternate_names,
    DROP COLUMN IF EXISTS image,
    DROP COLUMN IF EXISTS same_as,
    DROP COLUMN IF EXISTS keywords,
    DROP COLUMN IF EXISTS about,
    DROP COLUMN IF EXISTS in_language,
    DROP COLUMN IF EXISTS teaches,
    DROP COLUMN IF EXISTS assesses,
    DROP COLUMN IF EXISTS competency_required,
    DROP COLUMN IF EXISTS course_prerequisites,
    DROP COLUMN IF EXISTS available_language,
    DROP COLUMN IF EXISTS financial_aid_eligible,
    DROP COLUMN IF EXISTS educational_credential_awarded,
    DROP COLUMN IF EXISTS occupational_credential_awarded,
    DROP COLUMN IF EXISTS educational_level,
    DROP COLUMN IF EXISTS learning_resource_type;
ALTER TABLE courses ADD COLUMN educational_level      VARCHAR(32);
ALTER TABLE courses ADD COLUMN learning_resource_type VARCHAR(32);

-- course_identifiers: property_id JSONB → VARCHAR tag + custom_label + position.
ALTER TABLE course_identifiers DROP COLUMN IF EXISTS property_id;
ALTER TABLE course_identifiers ADD COLUMN property_id  VARCHAR(32) NOT NULL DEFAULT 'Custom';
ALTER TABLE course_identifiers ADD COLUMN custom_label VARCHAR(128);
ALTER TABLE course_identifiers ADD COLUMN position     INTEGER     NOT NULL DEFAULT 0;

-- course_instances: drop list/schedule JSON; flatten schedule onto the row.
ALTER TABLE course_instances
    DROP COLUMN IF EXISTS in_language,
    DROP COLUMN IF EXISTS instructor_ids,
    DROP COLUMN IF EXISTS instructor_names,
    DROP COLUMN IF EXISTS schedule;
ALTER TABLE course_instances ADD COLUMN schedule_start_date TIMESTAMP WITH TIME ZONE;
ALTER TABLE course_instances ADD COLUMN schedule_end_date   TIMESTAMP WITH TIME ZONE;
ALTER TABLE course_instances ADD COLUMN schedule_time_zone  VARCHAR(64);
ALTER TABLE course_instances ADD COLUMN schedule_recurrence VARCHAR(256);

-- syllabus_sections: drop teaches/resources JSON (moved to a child table).
ALTER TABLE syllabus_sections
    DROP COLUMN IF EXISTS teaches,
    DROP COLUMN IF EXISTS resources;

-- New normalized child tables.
CREATE TABLE course_text_values (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    course_id   UUID          NOT NULL REFERENCES courses (id) ON DELETE CASCADE,
    field       VARCHAR(32)   NOT NULL,
    value       VARCHAR(2048) NOT NULL,
    position    INTEGER       NOT NULL DEFAULT 0
);
CREATE INDEX idx_course_text_values_course ON course_text_values (course_id, field);

CREATE TABLE course_credentials (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    course_id           UUID         NOT NULL REFERENCES courses (id) ON DELETE CASCADE,
    role                VARCHAR(16)  NOT NULL,
    name                VARCHAR(500) NOT NULL,
    category            VARCHAR(64),
    educational_level   VARCHAR(64),
    recognized_by       VARCHAR(500),
    url                 VARCHAR(2048),
    CONSTRAINT uq_course_credentials_role UNIQUE (course_id, role)
);
CREATE INDEX idx_course_credentials_course ON course_credentials (course_id);

CREATE TABLE course_instance_languages (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    instance_id UUID        NOT NULL REFERENCES course_instances (id) ON DELETE CASCADE,
    language    VARCHAR(16) NOT NULL,
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

CREATE TABLE course_syllabus_text_values (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    section_id  UUID          NOT NULL REFERENCES syllabus_sections (id) ON DELETE CASCADE,
    field       VARCHAR(16)   NOT NULL,
    value       VARCHAR(2048) NOT NULL,
    position    INTEGER       NOT NULL DEFAULT 0
);
CREATE INDEX idx_course_syllabus_text_section ON course_syllabus_text_values (section_id);

-- providers: drop the two JSONB string-list columns (moved to a child table).
ALTER TABLE providers
    DROP COLUMN IF EXISTS alternate_names,
    DROP COLUMN IF EXISTS same_as;

CREATE TABLE provider_text_values (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider_id UUID          NOT NULL REFERENCES providers (id) ON DELETE CASCADE,
    field       VARCHAR(32)   NOT NULL,
    value       VARCHAR(2048) NOT NULL,
    position    INTEGER       NOT NULL DEFAULT 0
);
CREATE INDEX idx_provider_text_values_provider ON provider_text_values (provider_id);
