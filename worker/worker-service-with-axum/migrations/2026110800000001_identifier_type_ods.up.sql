-- WK-D1: admit the `ODS` identifier type.
--
-- `IdentifierType::ODS` (an NHS ODS organisation code) is a variant the
-- API has always accepted, but the original CHECK constraint listed only
-- the other six types and `OTHER`, so an ODS identifier could never be
-- stored: the insert failed and the create answered 500.
ALTER TABLE worker_identifiers
    DROP CONSTRAINT worker_identifiers_identifier_type_check;
ALTER TABLE worker_identifiers
    ADD CONSTRAINT worker_identifiers_identifier_type_check
    CHECK (identifier_type IN ('MRN', 'SSN', 'DL', 'NPI', 'PPN', 'TAX', 'ODS', 'OTHER'));
