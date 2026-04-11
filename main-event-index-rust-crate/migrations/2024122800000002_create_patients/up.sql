-- Create events table

CREATE TABLE events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    active BOOLEAN NOT NULL DEFAULT true,
    gender VARCHAR(20) NOT NULL CHECK (gender IN ('male', 'female', 'other', 'unknown')),
    birth_date DATE,
    deceased BOOLEAN NOT NULL DEFAULT false,
    deceased_datetime TIMESTAMPTZ,
    marital_status VARCHAR(50),
    multiple_birth BOOLEAN,
    managing_organization_id UUID REFERENCES organizations(id),

    -- Audit fields
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by VARCHAR(255),
    updated_by VARCHAR(255),

    -- Soft delete
    deleted_at TIMESTAMPTZ,
    deleted_by VARCHAR(255)
);

-- Indexes for events
CREATE INDEX idx_events_birth_date ON events(birth_date);
CREATE INDEX idx_events_gender ON events(gender);
CREATE INDEX idx_events_active ON events(active);
CREATE INDEX idx_events_organization ON events(managing_organization_id);
CREATE INDEX idx_events_deleted_at ON events(deleted_at);
CREATE INDEX idx_events_deceased ON events(deceased);
