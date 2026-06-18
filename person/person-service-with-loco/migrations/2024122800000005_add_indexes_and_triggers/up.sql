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
CREATE TRIGGER update_patients_updated_at
    BEFORE UPDATE ON patients
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_organizations_updated_at
    BEFORE UPDATE ON organizations
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_patient_names_updated_at
    BEFORE UPDATE ON patient_names
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_patient_identifiers_updated_at
    BEFORE UPDATE ON patient_identifiers
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_patient_addresses_updated_at
    BEFORE UPDATE ON patient_addresses
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_patient_contacts_updated_at
    BEFORE UPDATE ON patient_contacts
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

-- Function to audit patient changes
CREATE OR REPLACE FUNCTION audit_patient_changes()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        INSERT INTO audit_log (action, entity_type, entity_id, new_values, user_id)
        VALUES ('CREATE', 'patient', NEW.id, to_jsonb(NEW), NEW.created_by);
        RETURN NEW;
    ELSIF TG_OP = 'UPDATE' THEN
        INSERT INTO audit_log (action, entity_type, entity_id, old_values, new_values, user_id)
        VALUES ('UPDATE', 'patient', NEW.id, to_jsonb(OLD), to_jsonb(NEW), NEW.updated_by);
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        INSERT INTO audit_log (action, entity_type, entity_id, old_values, user_id)
        VALUES ('DELETE', 'patient', OLD.id, to_jsonb(OLD), OLD.deleted_by);
        RETURN OLD;
    END IF;
END;
$$ language 'plpgsql';

-- Apply audit trigger to patients table
CREATE TRIGGER audit_patients_changes
    AFTER INSERT OR UPDATE OR DELETE ON patients
    FOR EACH ROW
    EXECUTE FUNCTION audit_patient_changes();

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
CREATE INDEX idx_patient_names_family_trgm ON patient_names USING gin(family gin_trgm_ops);
CREATE INDEX idx_patient_names_given_trgm ON patient_names USING gin(given gin_trgm_ops);

-- Enable pg_trgm extension for fuzzy matching
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- Composite indexes for common queries
CREATE INDEX idx_patients_active_gender ON patients(active, gender) WHERE deleted_at IS NULL;
CREATE INDEX idx_patients_birth_date_gender ON patients(birth_date, gender) WHERE deleted_at IS NULL;
