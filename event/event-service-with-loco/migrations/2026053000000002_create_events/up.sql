-- The `events` table. Schema is aligned with https://schema.org/Event.
-- Repeated scalar arrays (alternate_names, image, same_as, keywords,
-- in_language) live in JSONB columns; relational children live in
-- separate tables (see 2026053000000003_create_event_related_tables).

CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TABLE events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    active BOOLEAN NOT NULL DEFAULT true,

    -- schema.org/Thing
    name VARCHAR(512) NOT NULL,
    description TEXT,
    disambiguating_description TEXT,
    url VARCHAR(2048),
    alternate_names JSONB NOT NULL DEFAULT '[]'::jsonb,
    image JSONB NOT NULL DEFAULT '[]'::jsonb,
    same_as JSONB NOT NULL DEFAULT '[]'::jsonb,
    keywords JSONB NOT NULL DEFAULT '[]'::jsonb,
    in_language JSONB NOT NULL DEFAULT '[]'::jsonb,

    -- schema.org/Event time window
    start_date TIMESTAMPTZ NOT NULL,
    end_date TIMESTAMPTZ,
    door_time TIMESTAMPTZ,
    duration VARCHAR(64),
    previous_start_date TIMESTAMPTZ,
    time_zone VARCHAR(64),
    all_day BOOLEAN NOT NULL DEFAULT false,

    -- Lifecycle / classification
    event_status VARCHAR(32) NOT NULL DEFAULT 'scheduled'
        CHECK (event_status IN ('scheduled','cancelled','moved_online','postponed','rescheduled','completed')),
    event_attendance_mode VARCHAR(16) NOT NULL DEFAULT 'offline'
        CHECK (event_attendance_mode IN ('offline','online','mixed')),
    event_type VARCHAR(32) NOT NULL DEFAULT 'generic',

    -- Audience / accessibility
    typical_age_range VARCHAR(32),
    is_accessible_for_free BOOLEAN,
    maximum_attendee_capacity INTEGER,
    maximum_physical_attendee_capacity INTEGER,
    maximum_virtual_attendee_capacity INTEGER,
    remaining_attendee_capacity INTEGER,

    -- Hierarchy
    super_event_id UUID REFERENCES events(id) ON DELETE SET NULL,

    -- Audit / soft-delete
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by VARCHAR(255),
    updated_by VARCHAR(255),
    deleted_at TIMESTAMPTZ,
    deleted_by VARCHAR(255),

    CONSTRAINT chk_end_after_start CHECK (end_date IS NULL OR end_date >= start_date),
    CONSTRAINT chk_door_before_start CHECK (door_time IS NULL OR door_time <= start_date),
    CONSTRAINT chk_capacities_nonneg CHECK (
        (maximum_attendee_capacity IS NULL OR maximum_attendee_capacity >= 0)
        AND (maximum_physical_attendee_capacity IS NULL OR maximum_physical_attendee_capacity >= 0)
        AND (maximum_virtual_attendee_capacity IS NULL OR maximum_virtual_attendee_capacity >= 0)
        AND (remaining_attendee_capacity IS NULL OR remaining_attendee_capacity >= 0)
    )
);

CREATE INDEX idx_events_start_date ON events(start_date);
CREATE INDEX idx_events_end_date ON events(end_date);
CREATE INDEX idx_events_status ON events(event_status);
CREATE INDEX idx_events_type ON events(event_type);
CREATE INDEX idx_events_attendance_mode ON events(event_attendance_mode);
CREATE INDEX idx_events_active ON events(active);
CREATE INDEX idx_events_super_event ON events(super_event_id);
CREATE INDEX idx_events_deleted_at ON events(deleted_at);
CREATE INDEX idx_events_name_lower ON events(LOWER(name));
