-- Drop ODS instance tables

DROP TRIGGER IF EXISTS update_organization_relationships_updated_at ON organization_relationships;
DROP TRIGGER IF EXISTS update_organization_roles_updated_at ON organization_roles;

DROP INDEX IF EXISTS idx_postcode_geo_lsoa;
DROP INDEX IF EXISTS idx_postcode_geo_icb;
DROP INDEX IF EXISTS idx_postcode_geo_la;
DROP INDEX IF EXISTS idx_org_periods_org_id;
DROP INDEX IF EXISTS idx_org_succs_type;
DROP INDEX IF EXISTS idx_org_succs_target;
DROP INDEX IF EXISTS idx_org_succs_org_id;
DROP INDEX IF EXISTS idx_org_rels_type;
DROP INDEX IF EXISTS idx_org_rels_target;
DROP INDEX IF EXISTS idx_org_rels_org_id;
DROP INDEX IF EXISTS idx_org_roles_primary;
DROP INDEX IF EXISTS idx_org_roles_code;
DROP INDEX IF EXISTS idx_org_roles_org_id;

DROP TABLE IF EXISTS postcode_geography;
DROP TABLE IF EXISTS organization_periods;
DROP TABLE IF EXISTS organization_successions;
DROP TABLE IF EXISTS organization_relationship_periods;
DROP TABLE IF EXISTS organization_relationships;
DROP TABLE IF EXISTS organization_role_periods;
DROP TABLE IF EXISTS organization_roles;
