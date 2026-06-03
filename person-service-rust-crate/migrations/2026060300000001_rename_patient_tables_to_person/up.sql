-- Rename `patient_*` tables to `person_*` to match the
-- SeaORM entity declarations in `src/db/models.rs` (`#[sea_orm(table_name = "persons")]`)
-- and the public REST contract (`/api/persons/*`). The original
-- migrations (`2024122800000002_create_patients` …) created the
-- tables under the legacy "patient" name; this migration brings the
-- schema in line with the spec.
BEGIN;
ALTER TABLE IF EXISTS patients              RENAME TO persons;
ALTER TABLE IF EXISTS patient_names         RENAME TO person_names;
ALTER TABLE IF EXISTS patient_identifiers   RENAME TO person_identifiers;
ALTER TABLE IF EXISTS patient_addresses     RENAME TO person_addresses;
ALTER TABLE IF EXISTS patient_contacts      RENAME TO person_contacts;
ALTER TABLE IF EXISTS patient_links         RENAME TO person_links;
ALTER TABLE IF EXISTS patient_match_scores  RENAME TO person_match_scores;
COMMIT;
