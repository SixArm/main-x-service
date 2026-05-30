-- Drop triggers and functions

-- Drop audit triggers
DROP TRIGGER IF EXISTS audit_organizations_changes ON organizations;
DROP TRIGGER IF EXISTS audit_events_changes ON events;

-- Drop update triggers
DROP TRIGGER IF EXISTS update_organization_contacts_updated_at ON organization_contacts;
DROP TRIGGER IF EXISTS update_organization_addresses_updated_at ON organization_addresses;
DROP TRIGGER IF EXISTS update_organization_identifiers_updated_at ON organization_identifiers;
DROP TRIGGER IF EXISTS update_event_contacts_updated_at ON event_contacts;
DROP TRIGGER IF EXISTS update_event_addresses_updated_at ON event_addresses;
DROP TRIGGER IF EXISTS update_event_identifiers_updated_at ON event_identifiers;
DROP TRIGGER IF EXISTS update_event_names_updated_at ON event_names;
DROP TRIGGER IF EXISTS update_organizations_updated_at ON organizations;
DROP TRIGGER IF EXISTS update_events_updated_at ON events;

-- Drop functions
DROP FUNCTION IF EXISTS audit_organization_changes();
DROP FUNCTION IF EXISTS audit_event_changes();
DROP FUNCTION IF EXISTS update_updated_at_column();

-- Drop full-text search indexes
DROP INDEX IF EXISTS idx_event_names_family_trgm;
DROP INDEX IF EXISTS idx_event_names_given_trgm;

-- Drop composite indexes
DROP INDEX IF EXISTS idx_events_active_gender;
DROP INDEX IF EXISTS idx_events_birth_date_gender;

-- Drop extensions
DROP EXTENSION IF EXISTS pg_trgm;
