-- Restore the original constraint. Fails while any `ODS` identifier is
-- stored, rather than discarding or relabelling it.
ALTER TABLE worker_identifiers
    DROP CONSTRAINT worker_identifiers_identifier_type_check;
ALTER TABLE worker_identifiers
    ADD CONSTRAINT worker_identifiers_identifier_type_check
    CHECK (identifier_type IN ('MRN', 'SSN', 'DL', 'NPI', 'PPN', 'TAX', 'OTHER'));
