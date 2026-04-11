-- Drop ODS CodeSystem reference tables

DROP INDEX IF EXISTS idx_geo_name_refs_type;
DROP INDEX IF EXISTS idx_ods_role_refs_primary;

DROP TABLE IF EXISTS geography_name_references;
DROP TABLE IF EXISTS practitioner_role_references;
DROP TABLE IF EXISTS ods_record_use_type_references;
DROP TABLE IF EXISTS ods_record_class_references;
DROP TABLE IF EXISTS ods_relationship_references;
DROP TABLE IF EXISTS ods_role_references;
