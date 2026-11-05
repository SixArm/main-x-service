-- Create patients table

CREATE TABLE patients (
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

-- Indexes for patients
CREATE INDEX idx_patients_birth_date ON patients(birth_date);
CREATE INDEX idx_patients_gender ON patients(gender);
CREATE INDEX idx_patients_active ON patients(active);
CREATE INDEX idx_patients_organization ON patients(managing_organization_id);
CREATE INDEX idx_patients_deleted_at ON patients(deleted_at);
CREATE INDEX idx_patients_deceased ON patients(deceased);
