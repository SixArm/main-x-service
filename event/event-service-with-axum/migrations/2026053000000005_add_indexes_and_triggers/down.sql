DROP INDEX IF EXISTS idx_events_type_start;
DROP INDEX IF EXISTS idx_events_active_status_start;
DROP INDEX IF EXISTS idx_event_locations_name_trgm;
DROP INDEX IF EXISTS idx_event_parties_name_trgm;
DROP INDEX IF EXISTS idx_events_description_trgm;
DROP INDEX IF EXISTS idx_events_name_trgm;

DROP TRIGGER IF EXISTS audit_organizations_changes ON organizations;
DROP FUNCTION IF EXISTS audit_organization_changes();
DROP TRIGGER IF EXISTS audit_events_changes ON events;
DROP FUNCTION IF EXISTS audit_event_changes();

DROP TRIGGER IF EXISTS update_organization_contacts_updated_at ON organization_contacts;
DROP TRIGGER IF EXISTS update_organization_addresses_updated_at ON organization_addresses;
DROP TRIGGER IF EXISTS update_organization_identifiers_updated_at ON organization_identifiers;
DROP TRIGGER IF EXISTS update_organizations_updated_at ON organizations;
DROP TRIGGER IF EXISTS update_event_offers_updated_at ON event_offers;
DROP TRIGGER IF EXISTS update_event_parties_updated_at ON event_parties;
DROP TRIGGER IF EXISTS update_event_locations_updated_at ON event_locations;
DROP TRIGGER IF EXISTS update_event_identifiers_updated_at ON event_identifiers;
DROP TRIGGER IF EXISTS update_events_updated_at ON events;
DROP FUNCTION IF EXISTS update_updated_at_column();
