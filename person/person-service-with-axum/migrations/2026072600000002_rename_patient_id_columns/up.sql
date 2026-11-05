-- Rename the leftover `patient_id` foreign-key columns to `person_id`.
--
-- `2026060300000001_rename_patient_tables_to_person` renamed the *tables*
-- (`patient_names` → `person_names`, …) but not their FK columns, so the
-- schema kept `patient_id` while the SeaORM entities in `src/db/models.rs`
-- declare `person_id`. Every insert into a person child table therefore
-- failed with `column "person_id" of relation "person_names" does not
-- exist`.
--
-- A separate migration rather than an edit to the rename, because that one
-- *can* have applied successfully in a deployment — this moves such a
-- deployment forward instead of rewriting its history.
--
-- `IF EXISTS` on each table and a guard on each column keep this
-- idempotent, so it is a no-op where the columns were already correct.
BEGIN;

DO $$
DECLARE
    t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY[
        'person_names',
        'person_identifiers',
        'person_addresses',
        'person_contacts',
        'person_links',
        'person_match_scores'
    ] LOOP
        IF to_regclass(t) IS NOT NULL
           AND EXISTS (
               SELECT 1 FROM information_schema.columns
               WHERE table_name = t AND column_name = 'patient_id'
           )
           AND NOT EXISTS (
               SELECT 1 FROM information_schema.columns
               WHERE table_name = t AND column_name = 'person_id'
           )
        THEN
            EXECUTE format('ALTER TABLE %I RENAME COLUMN patient_id TO person_id', t);
        END IF;
    END LOOP;
END $$;

-- `person_links` also carries the far side of the link.
DO $$
BEGIN
    IF to_regclass('person_links') IS NOT NULL
       AND EXISTS (
           SELECT 1 FROM information_schema.columns
           WHERE table_name = 'person_links' AND column_name = 'other_patient_id'
       )
       AND NOT EXISTS (
           SELECT 1 FROM information_schema.columns
           WHERE table_name = 'person_links' AND column_name = 'other_person_id'
       )
    THEN
        ALTER TABLE person_links RENAME COLUMN other_patient_id TO other_person_id;
    END IF;
END $$;

COMMIT;
