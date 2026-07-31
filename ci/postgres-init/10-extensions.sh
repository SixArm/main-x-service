#!/bin/sh
#
# Enable the Postgres extensions the test suites need.
#
# Mounted read-only into every service's test-database container (see any
# `<service>/compose.test.yaml`), so there is one definition rather than
# seventeen copies drifting apart. The official image runs everything in
# /docker-entrypoint-initdb.d exactly once, on first initialisation of an
# empty data directory.
#
# `template1` is done first and deliberately: Postgres copies it for every
# subsequently created database, so the `ci_*` databases that
# scripts/ci-check.sh creates per crate inherit these extensions without
# needing superuser DDL of their own.
set -eu

for db in template1 "${POSTGRES_DB}"; do
  psql -v ON_ERROR_STOP=1 --username "${POSTGRES_USER}" --dbname "${db}" <<-'SQL'
	-- uuid-ossp / citext / unaccent: required by scripts/ci-check.sh, which
	-- creates them per database today; having them in template1 makes that
	-- a no-op rather than a dependency.
	CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
	CREATE EXTENSION IF NOT EXISTS citext;
	CREATE EXTENSION IF NOT EXISTS unaccent;
	-- pg_trgm and pgcrypto are created by the person / worker / event SQL
	-- migrations themselves. Creating them here too is harmless (IF NOT
	-- EXISTS) and means a suite that forgets is not the thing that
	-- discovers it.
	CREATE EXTENSION IF NOT EXISTS pg_trgm;
	CREATE EXTENSION IF NOT EXISTS pgcrypto;
SQL
done

echo "test-db: extensions ready in template1 and ${POSTGRES_DB}"
