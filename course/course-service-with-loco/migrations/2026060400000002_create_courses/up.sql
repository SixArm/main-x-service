-- Courses (schema.org/Course template; instances live in course_instances).
CREATE TABLE IF NOT EXISTS courses (
    id                                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name                                VARCHAR(500)                NOT NULL,
    alternate_names                     JSONB                       NOT NULL DEFAULT '[]'::jsonb,
    description                         TEXT,
    disambiguating_description          TEXT,
    url                                 VARCHAR(2048),
    image                               JSONB                       NOT NULL DEFAULT '[]'::jsonb,
    same_as                             JSONB                       NOT NULL DEFAULT '[]'::jsonb,
    keywords                            JSONB                       NOT NULL DEFAULT '[]'::jsonb,
    additional_type                     VARCHAR(500),
    about                               JSONB                       NOT NULL DEFAULT '[]'::jsonb,
    audience                            VARCHAR(200),
    in_language                         JSONB                       NOT NULL DEFAULT '[]'::jsonb,
    license                             VARCHAR(2048),
    typical_age_range                   VARCHAR(50),
    time_required                       VARCHAR(50),
    version                             VARCHAR(100),
    is_accessible_for_free              BOOLEAN,
    teaches                             JSONB                       NOT NULL DEFAULT '[]'::jsonb,
    assesses                            JSONB                       NOT NULL DEFAULT '[]'::jsonb,
    competency_required                 JSONB                       NOT NULL DEFAULT '[]'::jsonb,
    educational_level                   JSONB,
    educational_use                     VARCHAR(200),
    learning_resource_type              JSONB,
    interactivity_type                  VARCHAR(20),
    course_code                         VARCHAR(100),
    number_of_credits                   INTEGER,
    course_prerequisites                JSONB                       NOT NULL DEFAULT '[]'::jsonb,
    available_language                  JSONB                       NOT NULL DEFAULT '[]'::jsonb,
    financial_aid_eligible              JSONB                       NOT NULL DEFAULT '[]'::jsonb,
    educational_credential_awarded      JSONB,
    occupational_credential_awarded     JSONB,
    total_historical_enrollment         BIGINT,
    status                              VARCHAR(20)                 NOT NULL DEFAULT 'published'
                                          CHECK (status IN ('draft','published','archived','retired')),
    active                              BOOLEAN                     NOT NULL DEFAULT TRUE,
    provider_id                         UUID                        REFERENCES providers(id),
    created_at                          TIMESTAMP WITH TIME ZONE    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at                          TIMESTAMP WITH TIME ZONE    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at                          TIMESTAMP WITH TIME ZONE
);

CREATE INDEX IF NOT EXISTS idx_courses_name         ON courses (LOWER(name));
CREATE INDEX IF NOT EXISTS idx_courses_course_code  ON courses (course_code);
CREATE INDEX IF NOT EXISTS idx_courses_provider     ON courses (provider_id);
CREATE INDEX IF NOT EXISTS idx_courses_status       ON courses (status);
CREATE INDEX IF NOT EXISTS idx_courses_deleted_at   ON courses (deleted_at);

-- Identifiers attached to a course (DOI, Wikidata, LMS id, …).
CREATE TABLE IF NOT EXISTS course_identifiers (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    course_id       UUID                        NOT NULL
                      REFERENCES courses(id) ON DELETE CASCADE,
    property_id     JSONB                       NOT NULL,
    value           VARCHAR(500)                NOT NULL,
    name            VARCHAR(200),
    url             VARCHAR(2048),
    created_at      TIMESTAMP WITH TIME ZONE    NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_course_identifiers_course    ON course_identifiers (course_id);
CREATE INDEX IF NOT EXISTS idx_course_identifiers_value     ON course_identifiers (value);

-- Course-to-course links (replaces / prerequisite / successor / see-also).
CREATE TABLE IF NOT EXISTS course_links (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    course_id           UUID                    NOT NULL
                          REFERENCES courses(id) ON DELETE CASCADE,
    other_course_id     UUID                    NOT NULL
                          REFERENCES courses(id) ON DELETE CASCADE,
    link_type           VARCHAR(20)             NOT NULL,
    created_at          TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (course_id, other_course_id, link_type)
);
CREATE INDEX IF NOT EXISTS idx_course_links_course  ON course_links (course_id);
