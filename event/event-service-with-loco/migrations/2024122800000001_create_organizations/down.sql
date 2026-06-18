-- Drop organization-related tables in reverse order

DROP TABLE IF EXISTS organization_contacts;
DROP TABLE IF EXISTS organization_addresses;
DROP TABLE IF EXISTS organization_identifiers;
DROP TABLE IF EXISTS organizations CASCADE;
