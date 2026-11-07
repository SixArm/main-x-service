-- Drop the never-used `organizations` table cluster (`organizations`,
-- `organization_addresses`, `organization_contacts`,
-- `organization_identifiers`), created by
-- 2024122800000001_create_organizations.
--
-- Spec §13 T-14: `organizations` was only ever a foreign-key target of
-- its own three child tables; nothing in the service inserted a row, so
-- the table was migrated and permanently empty in every deployment.
--
-- A new migration rather than an edit of the original, so a database
-- that ran it and one that never did agree on what "the schema" is. The
-- down migration recreates the four tables verbatim from the original
-- up migration, indexes included.
DROP TABLE IF EXISTS organization_contacts;
DROP TABLE IF EXISTS organization_addresses;
DROP TABLE IF EXISTS organization_identifiers;
DROP TABLE IF EXISTS organizations CASCADE;
