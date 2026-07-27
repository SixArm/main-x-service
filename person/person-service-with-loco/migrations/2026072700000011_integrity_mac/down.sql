-- Drop the keyed-integrity columns.
ALTER TABLE audit_log DROP COLUMN IF EXISTS mac;
ALTER TABLE persons DROP COLUMN IF EXISTS content_mac;
