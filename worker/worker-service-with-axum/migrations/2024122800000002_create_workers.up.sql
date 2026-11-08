-- Create workers table

CREATE TABLE workers (
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

-- Indexes for workers
CREATE INDEX idx_workers_birth_date ON workers(birth_date);
CREATE INDEX idx_workers_gender ON workers(gender);
CREATE INDEX idx_workers_active ON workers(active);
CREATE INDEX idx_workers_organization ON workers(managing_organization_id);
CREATE INDEX idx_workers_deleted_at ON workers(deleted_at);
CREATE INDEX idx_workers_deceased ON workers(deceased);
