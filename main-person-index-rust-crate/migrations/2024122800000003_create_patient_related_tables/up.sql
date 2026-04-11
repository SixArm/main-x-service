-- Create patient-related tables

-- Patient names
CREATE TABLE patient_names (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    patient_id UUID NOT NULL REFERENCES patients(id) ON DELETE CASCADE,
    use_type VARCHAR(20) CHECK (use_type IN ('usual', 'official', 'temp', 'nickname', 'anonymous', 'old', 'maiden')),
    family VARCHAR(255) NOT NULL,
    given TEXT[] NOT NULL DEFAULT '{}',
    prefix TEXT[] NOT NULL DEFAULT '{}',
    suffix TEXT[] NOT NULL DEFAULT '{}',
    is_primary BOOLEAN NOT NULL DEFAULT false,

    -- Audit fields
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Patient identifiers
CREATE TABLE patient_identifiers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    patient_id UUID NOT NULL REFERENCES patients(id) ON DELETE CASCADE,
    use_type VARCHAR(20) CHECK (use_type IN ('usual', 'official', 'temp', 'secondary', 'old')),
    identifier_type VARCHAR(10) NOT NULL CHECK (identifier_type IN ('MRN', 'SSN', 'DL', 'NPI', 'PPN', 'TAX', 'OTHER')),
    system VARCHAR(255) NOT NULL,
    value VARCHAR(255) NOT NULL,
    assigner VARCHAR(255),

    -- Audit fields
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Unique constraint: one identifier per system
    UNIQUE(system, value)
);

-- Patient addresses
CREATE TABLE patient_addresses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    patient_id UUID NOT NULL REFERENCES patients(id) ON DELETE CASCADE,
    use_type VARCHAR(20) CHECK (use_type IN ('home', 'work', 'temp', 'old', 'billing')),
    line1 VARCHAR(255),
    line2 VARCHAR(255),
    city VARCHAR(100),
    state VARCHAR(50),
    postal_code VARCHAR(20),
    country VARCHAR(100) DEFAULT 'USA',
    is_primary BOOLEAN NOT NULL DEFAULT false,

    -- Audit fields
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Patient contacts
CREATE TABLE patient_contacts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    patient_id UUID NOT NULL REFERENCES patients(id) ON DELETE CASCADE,
    system VARCHAR(20) NOT NULL CHECK (system IN ('phone', 'fax', 'email', 'pager', 'url', 'sms', 'other')),
    value VARCHAR(255) NOT NULL,
    use_type VARCHAR(20) CHECK (use_type IN ('home', 'work', 'temp', 'old', 'mobile')),
    is_primary BOOLEAN NOT NULL DEFAULT false,

    -- Audit fields
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Patient links (for duplicate/merged records)
CREATE TABLE patient_links (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    patient_id UUID NOT NULL REFERENCES patients(id) ON DELETE CASCADE,
    other_patient_id UUID NOT NULL REFERENCES patients(id) ON DELETE CASCADE,
    link_type VARCHAR(20) NOT NULL CHECK (link_type IN ('replaced_by', 'replaces', 'refer', 'seealso')),

    -- Audit fields
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by VARCHAR(255),

    -- Prevent self-links
    CHECK (patient_id != other_patient_id),

    -- Prevent duplicate links
    UNIQUE(patient_id, other_patient_id, link_type)
);

-- Patient match scores
CREATE TABLE patient_match_scores (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    patient_id UUID NOT NULL REFERENCES patients(id) ON DELETE CASCADE,
    candidate_id UUID NOT NULL REFERENCES patients(id) ON DELETE CASCADE,
    total_score DECIMAL(5,4) NOT NULL,
    name_score DECIMAL(5,4),
    birth_date_score DECIMAL(5,4),
    gender_score DECIMAL(5,4),
    address_score DECIMAL(5,4),
    identifier_score DECIMAL(5,4),
    calculated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Prevent self-matching
    CHECK (patient_id != candidate_id),

    -- Unique constraint
    UNIQUE(patient_id, candidate_id)
);

-- Indexes for patient_names
CREATE INDEX idx_patient_names_patient_id ON patient_names(patient_id);
CREATE INDEX idx_patient_names_family ON patient_names(family);
CREATE INDEX idx_patient_names_is_primary ON patient_names(is_primary);

-- Indexes for patient_identifiers
CREATE INDEX idx_patient_identifiers_patient_id ON patient_identifiers(patient_id);
CREATE INDEX idx_patient_identifiers_type ON patient_identifiers(identifier_type);
CREATE INDEX idx_patient_identifiers_value ON patient_identifiers(value);
CREATE INDEX idx_patient_identifiers_system_value ON patient_identifiers(system, value);

-- Indexes for patient_addresses
CREATE INDEX idx_patient_addresses_patient_id ON patient_addresses(patient_id);
CREATE INDEX idx_patient_addresses_postal_code ON patient_addresses(postal_code);
CREATE INDEX idx_patient_addresses_city_state ON patient_addresses(city, state);
CREATE INDEX idx_patient_addresses_is_primary ON patient_addresses(is_primary);

-- Indexes for patient_contacts
CREATE INDEX idx_patient_contacts_patient_id ON patient_contacts(patient_id);
CREATE INDEX idx_patient_contacts_system ON patient_contacts(system);
CREATE INDEX idx_patient_contacts_value ON patient_contacts(value);
CREATE INDEX idx_patient_contacts_is_primary ON patient_contacts(is_primary);

-- Indexes for patient_links
CREATE INDEX idx_patient_links_patient_id ON patient_links(patient_id);
CREATE INDEX idx_patient_links_other_patient_id ON patient_links(other_patient_id);
CREATE INDEX idx_patient_links_link_type ON patient_links(link_type);

-- Indexes for patient_match_scores
CREATE INDEX idx_match_scores_patient_id ON patient_match_scores(patient_id);
CREATE INDEX idx_match_scores_total_score ON patient_match_scores(total_score DESC);
CREATE INDEX idx_match_scores_calculated_at ON patient_match_scores(calculated_at);
