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
CREATE TRIGGER update_workers_updated_at
    BEFORE UPDATE ON workers
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_organizations_updated_at
    BEFORE UPDATE ON organizations
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_worker_names_updated_at
    BEFORE UPDATE ON worker_names
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_worker_identifiers_updated_at
    BEFORE UPDATE ON worker_identifiers
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_worker_addresses_updated_at
    BEFORE UPDATE ON worker_addresses
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_worker_contacts_updated_at
    BEFORE UPDATE ON worker_contacts
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

-- Function to audit worker changes
CREATE OR REPLACE FUNCTION audit_worker_changes()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        INSERT INTO audit_log (action, entity_type, entity_id, new_values, user_id)
        VALUES ('CREATE', 'worker', NEW.id, to_jsonb(NEW), NEW.created_by);
        RETURN NEW;
    ELSIF TG_OP = 'UPDATE' THEN
        INSERT INTO audit_log (action, entity_type, entity_id, old_values, new_values, user_id)
        VALUES ('UPDATE', 'worker', NEW.id, to_jsonb(OLD), to_jsonb(NEW), NEW.updated_by);
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        INSERT INTO audit_log (action, entity_type, entity_id, old_values, user_id)
        VALUES ('DELETE', 'worker', OLD.id, to_jsonb(OLD), OLD.deleted_by);
        RETURN OLD;
    END IF;
END;
$$ language 'plpgsql';

-- Apply audit trigger to workers table
CREATE TRIGGER audit_workers_changes
    AFTER INSERT OR UPDATE OR DELETE ON workers
    FOR EACH ROW
    EXECUTE FUNCTION audit_worker_changes();

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
--
-- Two defects fixed here (2026-07-26). Both made this block fail on a
-- fresh database, which halted the migration chain — so `audit_log.seq`,
-- `workers.worker_type`, and everything else added later never existed,
-- and the crate's DB-gated suite could not run at all.
--
--   1. `CREATE EXTENSION pg_trgm` came *after* the indexes that use
--      `gin_trgm_ops`, so the operator class was not yet defined.
--   2. `given` is `TEXT[]`, and `gin_trgm_ops` does not accept an array
--      type. The right GIN index for an array is the default `array_ops`,
--      which serves containment and overlap (`given @> ARRAY['jo']`) —
--      the questions you can actually ask of a set of given names in SQL.
--      An expression index over `array_to_string(given, ' ')` was tried
--      and rejected: Postgres requires index expressions to be IMMUTABLE
--      and `array_to_string` is not. Fuzzy given-name matching is not
--      lost — it happens in the `worker-matcher` crate (Jaro-Winkler,
--      Soundex) and in Tantivy, neither of which would have used a SQL
--      trigram index.
--
-- Editing this file in place is safe precisely because it could never
-- have applied: no deployment can have run past it.
CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE INDEX idx_worker_names_family_trgm ON worker_names USING gin(family gin_trgm_ops);
CREATE INDEX idx_worker_names_given_arr ON worker_names USING gin(given);

-- Composite indexes for common queries
CREATE INDEX idx_workers_active_gender ON workers(active, gender) WHERE deleted_at IS NULL;
CREATE INDEX idx_workers_birth_date_gender ON workers(birth_date, gender) WHERE deleted_at IS NULL;
