-- Create organizations table and related tables

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Organizations table
CREATE TABLE organizations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    active BOOLEAN NOT NULL DEFAULT true,
    name VARCHAR(255) NOT NULL,
    alias TEXT[] NOT NULL DEFAULT '{}',
    org_type TEXT[] NOT NULL DEFAULT '{}',
    part_of UUID REFERENCES organizations(id),

    -- Audit fields
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by VARCHAR(255),
    updated_by VARCHAR(255),

    -- Soft delete
    deleted_at TIMESTAMPTZ,
    deleted_by VARCHAR(255)
);

-- Organization identifiers
CREATE TABLE organization_identifiers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    use_type VARCHAR(20) CHECK (use_type IN ('usual', 'official', 'temp', 'secondary', 'old')),
    identifier_type VARCHAR(10) NOT NULL,
    system VARCHAR(255) NOT NULL,
    value VARCHAR(255) NOT NULL,
    assigner VARCHAR(255),

    -- Audit fields
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Unique constraint
    UNIQUE(system, value)
);

-- Organization addresses
CREATE TABLE organization_addresses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
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

-- Organization contacts
CREATE TABLE organization_contacts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    system VARCHAR(20) NOT NULL CHECK (system IN ('phone', 'fax', 'email', 'pager', 'url', 'sms', 'other')),
    value VARCHAR(255) NOT NULL,
    use_type VARCHAR(20) CHECK (use_type IN ('home', 'work', 'temp', 'old', 'mobile')),
    is_primary BOOLEAN NOT NULL DEFAULT false,

    -- Audit fields
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for organizations
CREATE INDEX idx_organizations_name ON organizations(name);
CREATE INDEX idx_organizations_active ON organizations(active);
CREATE INDEX idx_organizations_part_of ON organizations(part_of);
CREATE INDEX idx_organizations_deleted_at ON organizations(deleted_at);

-- Indexes for organization_identifiers
CREATE INDEX idx_org_identifiers_organization_id ON organization_identifiers(organization_id);
CREATE INDEX idx_org_identifiers_type ON organization_identifiers(identifier_type);
CREATE INDEX idx_org_identifiers_value ON organization_identifiers(value);

-- Indexes for organization_addresses
CREATE INDEX idx_org_addresses_organization_id ON organization_addresses(organization_id);
CREATE INDEX idx_org_addresses_postal_code ON organization_addresses(postal_code);

-- Indexes for organization_contacts
CREATE INDEX idx_org_contacts_organization_id ON organization_contacts(organization_id);
CREATE INDEX idx_org_contacts_system ON organization_contacts(system);
