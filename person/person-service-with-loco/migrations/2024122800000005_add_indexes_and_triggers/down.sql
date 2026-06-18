-- Drop triggers and functions

-- Drop audit triggers
DROP TRIGGER IF EXISTS audit_organizations_changes ON organizations;
DROP TRIGGER IF EXISTS audit_patients_changes ON patients;

-- Drop update triggers
DROP TRIGGER IF EXISTS update_organization_contacts_updated_at ON organization_contacts;
DROP TRIGGER IF EXISTS update_organization_addresses_updated_at ON organization_addresses;
DROP TRIGGER IF EXISTS update_organization_identifiers_updated_at ON organization_identifiers;
DROP TRIGGER IF EXISTS update_patient_contacts_updated_at ON patient_contacts;
DROP TRIGGER IF EXISTS update_patient_addresses_updated_at ON patient_addresses;
DROP TRIGGER IF EXISTS update_patient_identifiers_updated_at ON patient_identifiers;
DROP TRIGGER IF EXISTS update_patient_names_updated_at ON patient_names;
DROP TRIGGER IF EXISTS update_organizations_updated_at ON organizations;
DROP TRIGGER IF EXISTS update_patients_updated_at ON patients;

-- Drop functions
DROP FUNCTION IF EXISTS audit_organization_changes();
DROP FUNCTION IF EXISTS audit_patient_changes();
DROP FUNCTION IF EXISTS update_updated_at_column();

-- Drop full-text search indexes
DROP INDEX IF EXISTS idx_patient_names_family_trgm;
DROP INDEX IF EXISTS idx_patient_names_given_trgm;

-- Drop composite indexes
DROP INDEX IF EXISTS idx_patients_active_gender;
DROP INDEX IF EXISTS idx_patients_birth_date_gender;

-- Drop extensions
DROP EXTENSION IF EXISTS pg_trgm;
