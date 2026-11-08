DROP TABLE IF EXISTS worker_photos;
DROP TABLE IF EXISTS worker_emergency_contact_telecom;
DROP TABLE IF EXISTS worker_emergency_contacts;
DROP TABLE IF EXISTS worker_documents;
ALTER TABLE workers DROP COLUMN IF EXISTS tax_id;
