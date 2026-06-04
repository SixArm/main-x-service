-- Audit log + review-queue tables shared by the deduplication workflow.
CREATE TABLE IF NOT EXISTS audit_log (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_type     VARCHAR(50)                 NOT NULL,
    entity_id       UUID                        NOT NULL,
    action          VARCHAR(50)                 NOT NULL,
    user_id         VARCHAR(255),
    user_ip_address VARCHAR(45),
    user_agent      VARCHAR(500),
    old_values      JSONB,
    new_values      JSONB,
    created_at      TIMESTAMP WITH TIME ZONE    NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_audit_log_entity ON audit_log (entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_audit_log_created ON audit_log (created_at DESC);

CREATE TABLE IF NOT EXISTS course_match_scores (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    course_id       UUID                        NOT NULL
                      REFERENCES courses(id) ON DELETE CASCADE,
    candidate_id    UUID                        NOT NULL
                      REFERENCES courses(id) ON DELETE CASCADE,
    match_score     DOUBLE PRECISION            NOT NULL,
    match_quality   VARCHAR(50)                 NOT NULL,
    detection_method VARCHAR(50)                NOT NULL,
    score_breakdown JSONB,
    status          VARCHAR(20)                 NOT NULL DEFAULT 'Pending',
    reviewed_by     VARCHAR(255),
    created_at      TIMESTAMP WITH TIME ZONE    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    reviewed_at     TIMESTAMP WITH TIME ZONE,
    UNIQUE (course_id, candidate_id)
);
CREATE INDEX IF NOT EXISTS idx_course_match_scores_status ON course_match_scores (status);

CREATE TABLE IF NOT EXISTS course_merge_records (
    id                          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    main_course_id              UUID                    NOT NULL
                                  REFERENCES courses(id) ON DELETE CASCADE,
    duplicate_course_id         UUID                    NOT NULL,
    status                      VARCHAR(20)             NOT NULL,
    merged_by                   VARCHAR(255),
    merge_reason                TEXT,
    match_score                 DOUBLE PRECISION,
    transferred_data            JSONB,
    merged_at                   TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_course_merge_records_main ON course_merge_records (main_course_id);
