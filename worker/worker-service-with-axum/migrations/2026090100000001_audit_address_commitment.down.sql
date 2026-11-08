DROP INDEX IF EXISTS audit_log_address_held;
ALTER TABLE audit_log DROP COLUMN IF EXISTS ip_commitment;
ALTER TABLE audit_log DROP COLUMN IF EXISTS ip_salt;
