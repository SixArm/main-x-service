-- Organization roles (primary + non-primary)
CREATE TABLE organization_roles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    unique_role_id BIGINT NOT NULL,
    role_code VARCHAR(20) NOT NULL,
    role_name VARCHAR(255),
    is_primary BOOLEAN NOT NULL DEFAULT false,
    status VARCHAR(20) NOT NULL CHECK (status IN ('active', 'inactive')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(organization_id, unique_role_id)
);

-- Role date periods
CREATE TABLE organization_role_periods (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_role_id UUID NOT NULL REFERENCES organization_roles(id) ON DELETE CASCADE,
    period_type VARCHAR(20) NOT NULL CHECK (period_type IN ('legal', 'operational')),
    start_date DATE,
    end_date DATE
);

-- Organization relationships
CREATE TABLE organization_relationships (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    unique_rel_id BIGINT NOT NULL,
    relationship_type_code VARCHAR(20) NOT NULL,
    relationship_type_name VARCHAR(255),
    status VARCHAR(20) NOT NULL CHECK (status IN ('active', 'inactive')),
    target_ods_code VARCHAR(12) NOT NULL,
    target_primary_role_id VARCHAR(20),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(organization_id, unique_rel_id)
);

-- Relationship date periods
CREATE TABLE organization_relationship_periods (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_relationship_id UUID NOT NULL REFERENCES organization_relationships(id) ON DELETE CASCADE,
    period_type VARCHAR(20) NOT NULL CHECK (period_type IN ('legal', 'operational')),
    start_date DATE,
    end_date DATE
);

-- Organization successions (mergers, acquisitions)
CREATE TABLE organization_successions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    unique_succ_id BIGINT NOT NULL,
    succession_type VARCHAR(20) NOT NULL CHECK (succession_type IN ('predecessor', 'successor')),
    target_ods_code VARCHAR(12) NOT NULL,
    target_primary_role_id VARCHAR(20),
    legal_start_date DATE,
    has_forward_succession BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(organization_id, unique_succ_id)
);

-- Organization date periods (legal/operational existence)
CREATE TABLE organization_periods (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    period_type VARCHAR(20) NOT NULL CHECK (period_type IN ('legal', 'operational')),
    start_date DATE,
    end_date DATE
);

-- Postcode geography boundary mappings
CREATE TABLE postcode_geography (
    postcode VARCHAR(10) PRIMARY KEY,
    lsoa11 VARCHAR(20),
    local_authority VARCHAR(20),
    local_authority_name VARCHAR(255),
    icb VARCHAR(20),
    icb_name VARCHAR(255),
    nhs_england_region VARCHAR(20),
    nhs_england_region_name VARCHAR(255),
    parliamentary_constituency VARCHAR(20),
    parliamentary_constituency_name VARCHAR(255),
    government_office_region VARCHAR(20),
    cancer_alliance VARCHAR(20),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Indexes
CREATE INDEX idx_org_roles_org_id ON organization_roles(organization_id);
CREATE INDEX idx_org_roles_code ON organization_roles(role_code);
CREATE INDEX idx_org_roles_primary ON organization_roles(is_primary);
CREATE INDEX idx_org_rels_org_id ON organization_relationships(organization_id);
CREATE INDEX idx_org_rels_target ON organization_relationships(target_ods_code);
CREATE INDEX idx_org_rels_type ON organization_relationships(relationship_type_code);
CREATE INDEX idx_org_succs_org_id ON organization_successions(organization_id);
CREATE INDEX idx_org_succs_target ON organization_successions(target_ods_code);
CREATE INDEX idx_org_succs_type ON organization_successions(succession_type);
CREATE INDEX idx_org_periods_org_id ON organization_periods(organization_id);
CREATE INDEX idx_postcode_geo_la ON postcode_geography(local_authority);
CREATE INDEX idx_postcode_geo_icb ON postcode_geography(icb);
CREATE INDEX idx_postcode_geo_lsoa ON postcode_geography(lsoa11);

-- Auto-update triggers for updated_at columns
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_organization_roles_updated_at
    BEFORE UPDATE ON organization_roles
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_organization_relationships_updated_at
    BEFORE UPDATE ON organization_relationships
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
