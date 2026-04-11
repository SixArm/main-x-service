-- Expand organizations table with NHS ODS fields

ALTER TABLE organizations ADD COLUMN ods_code VARCHAR(12) UNIQUE;
ALTER TABLE organizations ADD COLUMN ods_status VARCHAR(20)
    CHECK (ods_status IN ('active', 'inactive'));
ALTER TABLE organizations ADD COLUMN record_class VARCHAR(20)
    CHECK (record_class IN ('organisation', 'site'));
ALTER TABLE organizations ADD COLUMN record_use_type VARCHAR(20)
    CHECK (record_use_type IN ('full', 'ref_only'));
ALTER TABLE organizations ADD COLUMN assigning_authority VARCHAR(255);
ALTER TABLE organizations ADD COLUMN last_change_date DATE;

-- Indexes
CREATE INDEX idx_organizations_ods_code ON organizations(ods_code);
CREATE INDEX idx_organizations_record_class ON organizations(record_class);
CREATE INDEX idx_organizations_ods_status ON organizations(ods_status);
