-- Remove ODS columns from organizations table

DROP INDEX IF EXISTS idx_organizations_ods_status;
DROP INDEX IF EXISTS idx_organizations_record_class;
DROP INDEX IF EXISTS idx_organizations_ods_code;

ALTER TABLE organizations DROP COLUMN IF EXISTS last_change_date;
ALTER TABLE organizations DROP COLUMN IF EXISTS assigning_authority;
ALTER TABLE organizations DROP COLUMN IF EXISTS record_use_type;
ALTER TABLE organizations DROP COLUMN IF EXISTS record_class;
ALTER TABLE organizations DROP COLUMN IF EXISTS ods_status;
ALTER TABLE organizations DROP COLUMN IF EXISTS ods_code;
