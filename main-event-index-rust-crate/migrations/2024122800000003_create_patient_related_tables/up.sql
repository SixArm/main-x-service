-- Create event-related tables

-- Event names
CREATE TABLE event_names (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
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

-- Event identifiers
CREATE TABLE event_identifiers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
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

-- Event addresses
CREATE TABLE event_addresses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
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

-- Event contacts
CREATE TABLE event_contacts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    system VARCHAR(20) NOT NULL CHECK (system IN ('phone', 'fax', 'email', 'pager', 'url', 'sms', 'other')),
    value VARCHAR(255) NOT NULL,
    use_type VARCHAR(20) CHECK (use_type IN ('home', 'work', 'temp', 'old', 'mobile')),
    is_primary BOOLEAN NOT NULL DEFAULT false,

    -- Audit fields
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Event links (for duplicate/merged records)
CREATE TABLE event_links (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    other_event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    link_type VARCHAR(20) NOT NULL CHECK (link_type IN ('replaced_by', 'replaces', 'refer', 'seealso')),

    -- Audit fields
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by VARCHAR(255),

    -- Prevent self-links
    CHECK (event_id != other_event_id),

    -- Prevent duplicate links
    UNIQUE(event_id, other_event_id, link_type)
);

-- Event match scores
CREATE TABLE event_match_scores (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    candidate_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    total_score DECIMAL(5,4) NOT NULL,
    name_score DECIMAL(5,4),
    birth_date_score DECIMAL(5,4),
    gender_score DECIMAL(5,4),
    address_score DECIMAL(5,4),
    identifier_score DECIMAL(5,4),
    calculated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Prevent self-matching
    CHECK (event_id != candidate_id),

    -- Unique constraint
    UNIQUE(event_id, candidate_id)
);

-- Indexes for event_names
CREATE INDEX idx_event_names_event_id ON event_names(event_id);
CREATE INDEX idx_event_names_family ON event_names(family);
CREATE INDEX idx_event_names_is_primary ON event_names(is_primary);

-- Indexes for event_identifiers
CREATE INDEX idx_event_identifiers_event_id ON event_identifiers(event_id);
CREATE INDEX idx_event_identifiers_type ON event_identifiers(identifier_type);
CREATE INDEX idx_event_identifiers_value ON event_identifiers(value);
CREATE INDEX idx_event_identifiers_system_value ON event_identifiers(system, value);

-- Indexes for event_addresses
CREATE INDEX idx_event_addresses_event_id ON event_addresses(event_id);
CREATE INDEX idx_event_addresses_postal_code ON event_addresses(postal_code);
CREATE INDEX idx_event_addresses_city_state ON event_addresses(city, state);
CREATE INDEX idx_event_addresses_is_primary ON event_addresses(is_primary);

-- Indexes for event_contacts
CREATE INDEX idx_event_contacts_event_id ON event_contacts(event_id);
CREATE INDEX idx_event_contacts_system ON event_contacts(system);
CREATE INDEX idx_event_contacts_value ON event_contacts(value);
CREATE INDEX idx_event_contacts_is_primary ON event_contacts(is_primary);

-- Indexes for event_links
CREATE INDEX idx_event_links_event_id ON event_links(event_id);
CREATE INDEX idx_event_links_other_event_id ON event_links(other_event_id);
CREATE INDEX idx_event_links_link_type ON event_links(link_type);

-- Indexes for event_match_scores
CREATE INDEX idx_match_scores_event_id ON event_match_scores(event_id);
CREATE INDEX idx_match_scores_total_score ON event_match_scores(total_score DESC);
CREATE INDEX idx_match_scores_calculated_at ON event_match_scores(calculated_at);
