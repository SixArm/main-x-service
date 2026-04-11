-- Add triggers and additional functions

-- Function to automatically update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Apply updated_at trigger to all tables with updated_at column
CREATE TRIGGER update_events_updated_at
    BEFORE UPDATE ON events
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_organizations_updated_at
    BEFORE UPDATE ON organizations
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_event_names_updated_at
    BEFORE UPDATE ON event_names
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_event_identifiers_updated_at
    BEFORE UPDATE ON event_identifiers
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_event_addresses_updated_at
    BEFORE UPDATE ON event_addresses
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_event_contacts_updated_at
    BEFORE UPDATE ON event_contacts
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_organization_identifiers_updated_at
    BEFORE UPDATE ON organization_identifiers
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_organization_addresses_updated_at
    BEFORE UPDATE ON organization_addresses
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_organization_contacts_updated_at
    BEFORE UPDATE ON organization_contacts
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Function to audit event changes
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

-- Apply audit trigger to events table
CREATE TRIGGER audit_events_changes
    AFTER INSERT OR UPDATE OR DELETE ON events
    FOR EACH ROW
    EXECUTE FUNCTION audit_event_changes();

-- Function to audit organization changes
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

-- Apply audit trigger to organizations table
CREATE TRIGGER audit_organizations_changes
    AFTER INSERT OR UPDATE OR DELETE ON organizations
    FOR EACH ROW
    EXECUTE FUNCTION audit_organization_changes();

-- Full-text search support (using PostgreSQL built-in)
CREATE INDEX idx_event_names_family_trgm ON event_names USING gin(family gin_trgm_ops);
CREATE INDEX idx_event_names_given_trgm ON event_names USING gin(given gin_trgm_ops);

-- Enable pg_trgm extension for fuzzy matching
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- Composite indexes for common queries
CREATE INDEX idx_events_active_gender ON events(active, gender) WHERE deleted_at IS NULL;
CREATE INDEX idx_events_birth_date_gender ON events(birth_date, gender) WHERE deleted_at IS NULL;
