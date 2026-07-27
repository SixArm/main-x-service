-- Drop the keyed-integrity columns.
ALTER TABLE audit_log DROP COLUMN IF EXISTS mac;
ALTER TABLE workers DROP COLUMN IF EXISTS content_mac;
ALTER TABLE worker_assessments DROP COLUMN IF EXISTS content_mac;
