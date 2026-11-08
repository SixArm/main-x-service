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

-- Trigram indexes for fuzzy name matching.
--
-- Fixed 2026-07-26. As written this block could never apply to a fresh
-- database, so no deployment can have run past it and editing it in place
-- is safe:
--
--   1. The extension was created *after* the indexes that use its
--      operator class, so the first CREATE INDEX failed with
--      "operator class gin_trgm_ops does not exist for access method gin".
--   2. `given` is `TEXT[]`, and `gin_trgm_ops` accepts only text/varchar,
--      so it failed with "does not accept data type text[]".
--
-- The extension now comes first. `family` is `VARCHAR`, so it keeps a
-- real trigram index. `given` is an array, and the right GIN index for an
-- array is the default `array_ops` — it serves containment and overlap
-- (`given @> ARRAY['john']`), which is what you can actually ask of a set
-- of given names in SQL.
--
-- An expression index over `array_to_string(given, ' ')` was tried and
-- rejected: Postgres requires index expressions to be IMMUTABLE and
-- `array_to_string` is not. Fuzzy given-name matching is not lost — it
-- happens in the `person-matcher` crate (Jaro-Winkler, Soundex) and in
-- Tantivy, neither of which would have used a SQL trigram index.
CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE INDEX idx_patient_names_family_trgm
    ON patient_names USING gin (family gin_trgm_ops);
CREATE INDEX idx_patient_names_given_arr
    ON patient_names USING gin (given);

-- Composite indexes for common queries
CREATE INDEX idx_patients_active_gender ON patients(active, gender) WHERE deleted_at IS NULL;
CREATE INDEX idx_patients_birth_date_gender ON patients(birth_date, gender) WHERE deleted_at IS NULL;
