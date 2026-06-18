BEGIN;
ALTER TABLE IF EXISTS person_match_scores  RENAME TO patient_match_scores;
ALTER TABLE IF EXISTS person_links         RENAME TO patient_links;
ALTER TABLE IF EXISTS person_contacts      RENAME TO patient_contacts;
ALTER TABLE IF EXISTS person_addresses     RENAME TO patient_addresses;
ALTER TABLE IF EXISTS person_identifiers   RENAME TO patient_identifiers;
ALTER TABLE IF EXISTS person_names         RENAME TO patient_names;
ALTER TABLE IF EXISTS persons              RENAME TO patients;
COMMIT;
