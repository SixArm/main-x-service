-- Specific offerings of a course at a particular time / place / mode.
CREATE TABLE IF NOT EXISTS course_instances (
    id                              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    course_id                       UUID                        NOT NULL
                                      REFERENCES courses(id) ON DELETE CASCADE,
    name                            VARCHAR(500),
    course_mode                     VARCHAR(20),
    status                          VARCHAR(30)                 NOT NULL DEFAULT 'scheduled'
                                      CHECK (status IN ('scheduled','enrollment_open','enrollment_closed','in_progress','completed','cancelled')),
    in_language                     JSONB                       NOT NULL DEFAULT '[]'::jsonb,
    location                        TEXT,
    location_id                     UUID,
    instructor_ids                  JSONB                       NOT NULL DEFAULT '[]'::jsonb,
    instructor_names                JSONB                       NOT NULL DEFAULT '[]'::jsonb,
    maximum_attendee_capacity       INTEGER,
    enrolled_count                  INTEGER,
    enrollment_opens                TIMESTAMP WITH TIME ZONE,
    enrollment_closes               TIMESTAMP WITH TIME ZONE,
    schedule                        JSONB,
    created_at                      TIMESTAMP WITH TIME ZONE    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at                      TIMESTAMP WITH TIME ZONE    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at                      TIMESTAMP WITH TIME ZONE
);

CREATE INDEX IF NOT EXISTS idx_course_instances_course  ON course_instances (course_id);
CREATE INDEX IF NOT EXISTS idx_course_instances_status  ON course_instances (status);

-- Syllabus sections, hierarchical (parent_id self-FK).
CREATE TABLE IF NOT EXISTS syllabus_sections (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    course_id       UUID                        NOT NULL
                      REFERENCES courses(id) ON DELETE CASCADE,
    parent_id       UUID
                      REFERENCES syllabus_sections(id) ON DELETE CASCADE,
    name            VARCHAR(500)                NOT NULL,
    description     TEXT,
    position        INTEGER,
    teaches         JSONB                       NOT NULL DEFAULT '[]'::jsonb,
    time_required   VARCHAR(50),
    resources       JSONB                       NOT NULL DEFAULT '[]'::jsonb,
    created_at      TIMESTAMP WITH TIME ZONE    NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_syllabus_sections_course ON syllabus_sections (course_id);
CREATE INDEX IF NOT EXISTS idx_syllabus_sections_parent ON syllabus_sections (parent_id);
