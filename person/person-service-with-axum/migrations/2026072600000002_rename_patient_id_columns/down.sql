BEGIN;
DO $$
DECLARE
    t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY[
        'person_names', 'person_identifiers', 'person_addresses',
        'person_contacts', 'person_links', 'person_match_scores'
    ] LOOP
        IF to_regclass(t) IS NOT NULL
           AND EXISTS (SELECT 1 FROM information_schema.columns
                       WHERE table_name = t AND column_name = 'person_id')
        THEN
            EXECUTE format('ALTER TABLE %I RENAME COLUMN person_id TO patient_id', t);
        END IF;
    END LOOP;
END $$;
DO $$
BEGIN
    IF to_regclass('person_links') IS NOT NULL
       AND EXISTS (SELECT 1 FROM information_schema.columns
                   WHERE table_name = 'person_links' AND column_name = 'other_person_id')
    THEN
        ALTER TABLE person_links RENAME COLUMN other_person_id TO other_patient_id;
    END IF;
END $$;
COMMIT;
