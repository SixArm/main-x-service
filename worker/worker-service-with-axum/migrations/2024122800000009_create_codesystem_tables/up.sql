-- ODS CodeSystem reference tables

-- ODSOrganisationRole reference data
CREATE TABLE ods_role_references (
    role_id VARCHAR(20) PRIMARY KEY,
    role_name VARCHAR(255) NOT NULL,
    is_primary_role_type BOOLEAN NOT NULL DEFAULT false,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ODSRelationship reference data
CREATE TABLE ods_relationship_references (
    relationship_id VARCHAR(20) PRIMARY KEY,
    relationship_name VARCHAR(255) NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ODSRecordClass reference data
CREATE TABLE ods_record_class_references (
    code VARCHAR(10) PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ODSRecordUseType reference data
CREATE TABLE ods_record_use_type_references (
    code VARCHAR(20) PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Practitioner role reference data
CREATE TABLE practitioner_role_references (
    role_code VARCHAR(20) PRIMARY KEY,
    role_name VARCHAR(255) NOT NULL,
    role_category VARCHAR(100),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Geography name reference data (ONS identifiers)
CREATE TABLE geography_name_references (
    ons_code VARCHAR(20) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    geography_type VARCHAR(100) NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Indexes
CREATE INDEX idx_ods_role_refs_primary ON ods_role_references(is_primary_role_type);
CREATE INDEX idx_geo_name_refs_type ON geography_name_references(geography_type);
