DROP TABLE IF EXISTS person_photos;
DROP TABLE IF EXISTS person_emergency_contact_telecom;
DROP TABLE IF EXISTS person_emergency_contacts;
DROP TABLE IF EXISTS person_documents;
ALTER TABLE persons DROP COLUMN IF EXISTS tax_id;
