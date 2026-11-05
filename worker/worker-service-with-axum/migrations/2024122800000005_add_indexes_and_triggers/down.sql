-- Drop triggers and functions

-- Drop audit triggers
DROP TRIGGER IF EXISTS audit_organizations_changes ON organizations;
DROP TRIGGER IF EXISTS audit_workers_changes ON workers;

-- Drop update triggers
DROP TRIGGER IF EXISTS update_organization_contacts_updated_at ON organization_contacts;
DROP TRIGGER IF EXISTS update_organization_addresses_updated_at ON organization_addresses;
DROP TRIGGER IF EXISTS update_organization_identifiers_updated_at ON organization_identifiers;
DROP TRIGGER IF EXISTS update_worker_contacts_updated_at ON worker_contacts;
DROP TRIGGER IF EXISTS update_worker_addresses_updated_at ON worker_addresses;
DROP TRIGGER IF EXISTS update_worker_identifiers_updated_at ON worker_identifiers;
DROP TRIGGER IF EXISTS update_worker_names_updated_at ON worker_names;
DROP TRIGGER IF EXISTS update_organizations_updated_at ON organizations;
DROP TRIGGER IF EXISTS update_workers_updated_at ON workers;

-- Drop functions
DROP FUNCTION IF EXISTS audit_organization_changes();
DROP FUNCTION IF EXISTS audit_worker_changes();
DROP FUNCTION IF EXISTS update_updated_at_column();

-- Drop full-text search indexes
DROP INDEX IF EXISTS idx_worker_names_family_trgm;
DROP INDEX IF EXISTS idx_worker_names_given_trgm;

-- Drop composite indexes
DROP INDEX IF EXISTS idx_workers_active_gender;
DROP INDEX IF EXISTS idx_workers_birth_date_gender;

-- Drop extensions
DROP EXTENSION IF EXISTS pg_trgm;
