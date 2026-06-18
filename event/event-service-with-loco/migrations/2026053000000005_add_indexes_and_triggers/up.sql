-- Trigram + fuzzy-search support, updated_at triggers, audit triggers
-- for events and organizations.

CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- ---------------------------------------------------------------------------
-- updated_at trigger function (idempotent)
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_events_updated_at
    BEFORE UPDATE ON events
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_event_identifiers_updated_at
    BEFORE UPDATE ON event_identifiers
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_event_locations_updated_at
    BEFORE UPDATE ON event_locations
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_event_parties_updated_at
    BEFORE UPDATE ON event_parties
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_event_offers_updated_at
    BEFORE UPDATE ON event_offers
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_organizations_updated_at
    BEFORE UPDATE ON organizations
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_organization_identifiers_updated_at
    BEFORE UPDATE ON organization_identifiers
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_organization_addresses_updated_at
    BEFORE UPDATE ON organization_addresses
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_organization_contacts_updated_at
    BEFORE UPDATE ON organization_contacts
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ---------------------------------------------------------------------------
-- Audit triggers (write to audit_log on INSERT/UPDATE/DELETE)
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION audit_event_changes()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        INSERT INTO audit_log (action, entity_type, entity_id, new_values, user_id)
        VALUES ('CREATE', 'event', NEW.id, to_jsonb(NEW), NEW.created_by);
        RETURN NEW;
    ELSIF TG_OP = 'UPDATE' THEN
        INSERT INTO audit_log (action, entity_type, entity_id, old_values, new_values, user_id)
        VALUES ('UPDATE', 'event', NEW.id, to_jsonb(OLD), to_jsonb(NEW), NEW.updated_by);
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        INSERT INTO audit_log (action, entity_type, entity_id, old_values, user_id)
        VALUES ('DELETE', 'event', OLD.id, to_jsonb(OLD), OLD.deleted_by);
        RETURN OLD;
    END IF;
END;
$$ language 'plpgsql';

CREATE TRIGGER audit_events_changes
    AFTER INSERT OR UPDATE OR DELETE ON events
    FOR EACH ROW EXECUTE FUNCTION audit_event_changes();

CREATE OR REPLACE FUNCTION audit_organization_changes()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        INSERT INTO audit_log (action, entity_type, entity_id, new_values, user_id)
        VALUES ('CREATE', 'organization', NEW.id, to_jsonb(NEW), NEW.created_by);
        RETURN NEW;
    ELSIF TG_OP = 'UPDATE' THEN
        INSERT INTO audit_log (action, entity_type, entity_id, old_values, new_values, user_id)
        VALUES ('UPDATE', 'organization', NEW.id, to_jsonb(OLD), to_jsonb(NEW), NEW.updated_by);
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        INSERT INTO audit_log (action, entity_type, entity_id, old_values, user_id)
        VALUES ('DELETE', 'organization', OLD.id, to_jsonb(OLD), OLD.deleted_by);
        RETURN OLD;
    END IF;
END;
$$ language 'plpgsql';

CREATE TRIGGER audit_organizations_changes
    AFTER INSERT OR UPDATE OR DELETE ON organizations
    FOR EACH ROW EXECUTE FUNCTION audit_organization_changes();

-- ---------------------------------------------------------------------------
-- Trigram indexes for fuzzy text search
-- ---------------------------------------------------------------------------

CREATE INDEX idx_events_name_trgm ON events USING gin(name gin_trgm_ops);
CREATE INDEX idx_events_description_trgm ON events USING gin(description gin_trgm_ops);
CREATE INDEX idx_event_parties_name_trgm ON event_parties USING gin(name gin_trgm_ops);
CREATE INDEX idx_event_locations_name_trgm ON event_locations USING gin(name gin_trgm_ops);

-- ---------------------------------------------------------------------------
-- Composite indexes for common queries
-- ---------------------------------------------------------------------------

CREATE INDEX idx_events_active_status_start
    ON events(active, event_status, start_date)
    WHERE deleted_at IS NULL;

CREATE INDEX idx_events_type_start
    ON events(event_type, start_date)
    WHERE deleted_at IS NULL;
