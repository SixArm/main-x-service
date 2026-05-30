-- Create worker-related tables

-- Worker names
CREATE TABLE worker_names (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    worker_id UUID NOT NULL REFERENCES workers(id) ON DELETE CASCADE,
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

-- Worker identifiers
CREATE TABLE worker_identifiers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    worker_id UUID NOT NULL REFERENCES workers(id) ON DELETE CASCADE,
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

-- Worker addresses
CREATE TABLE worker_addresses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    worker_id UUID NOT NULL REFERENCES workers(id) ON DELETE CASCADE,
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

-- Worker contacts
CREATE TABLE worker_contacts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    worker_id UUID NOT NULL REFERENCES workers(id) ON DELETE CASCADE,
    system VARCHAR(20) NOT NULL CHECK (system IN ('phone', 'fax', 'email', 'pager', 'url', 'sms', 'other')),
    value VARCHAR(255) NOT NULL,
    use_type VARCHAR(20) CHECK (use_type IN ('home', 'work', 'temp', 'old', 'mobile')),
    is_primary BOOLEAN NOT NULL DEFAULT false,

    -- Audit fields
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Worker links (for duplicate/merged records)
CREATE TABLE worker_links (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    worker_id UUID NOT NULL REFERENCES workers(id) ON DELETE CASCADE,
    other_worker_id UUID NOT NULL REFERENCES workers(id) ON DELETE CASCADE,
    link_type VARCHAR(20) NOT NULL CHECK (link_type IN ('replaced_by', 'replaces', 'refer', 'seealso')),

    -- Audit fields
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by VARCHAR(255),

    -- Prevent self-links
    CHECK (worker_id != other_worker_id),

    -- Prevent duplicate links
    UNIQUE(worker_id, other_worker_id, link_type)
);

-- Worker match scores
CREATE TABLE worker_match_scores (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    worker_id UUID NOT NULL REFERENCES workers(id) ON DELETE CASCADE,
    candidate_id UUID NOT NULL REFERENCES workers(id) ON DELETE CASCADE,
    total_score DECIMAL(5,4) NOT NULL,
    name_score DECIMAL(5,4),
    birth_date_score DECIMAL(5,4),
    gender_score DECIMAL(5,4),
    address_score DECIMAL(5,4),
    identifier_score DECIMAL(5,4),
    calculated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Prevent self-matching
    CHECK (worker_id != candidate_id),

    -- Unique constraint
    UNIQUE(worker_id, candidate_id)
);

-- Indexes for worker_names
CREATE INDEX idx_worker_names_worker_id ON worker_names(worker_id);
CREATE INDEX idx_worker_names_family ON worker_names(family);
CREATE INDEX idx_worker_names_is_primary ON worker_names(is_primary);

-- Indexes for worker_identifiers
CREATE INDEX idx_worker_identifiers_worker_id ON worker_identifiers(worker_id);
CREATE INDEX idx_worker_identifiers_type ON worker_identifiers(identifier_type);
CREATE INDEX idx_worker_identifiers_value ON worker_identifiers(value);
CREATE INDEX idx_worker_identifiers_system_value ON worker_identifiers(system, value);

-- Indexes for worker_addresses
CREATE INDEX idx_worker_addresses_worker_id ON worker_addresses(worker_id);
CREATE INDEX idx_worker_addresses_postal_code ON worker_addresses(postal_code);
CREATE INDEX idx_worker_addresses_city_state ON worker_addresses(city, state);
CREATE INDEX idx_worker_addresses_is_primary ON worker_addresses(is_primary);

-- Indexes for worker_contacts
CREATE INDEX idx_worker_contacts_worker_id ON worker_contacts(worker_id);
CREATE INDEX idx_worker_contacts_system ON worker_contacts(system);
CREATE INDEX idx_worker_contacts_value ON worker_contacts(value);
CREATE INDEX idx_worker_contacts_is_primary ON worker_contacts(is_primary);

-- Indexes for worker_links
CREATE INDEX idx_worker_links_worker_id ON worker_links(worker_id);
CREATE INDEX idx_worker_links_other_worker_id ON worker_links(other_worker_id);
CREATE INDEX idx_worker_links_link_type ON worker_links(link_type);

-- Indexes for worker_match_scores
CREATE INDEX idx_match_scores_worker_id ON worker_match_scores(worker_id);
CREATE INDEX idx_match_scores_total_score ON worker_match_scores(total_score DESC);
CREATE INDEX idx_match_scores_calculated_at ON worker_match_scores(calculated_at);
