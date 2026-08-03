#!/bin/sh
#
# Enable the Postgres extensions the service migrations need. Copied
# from ci/postgres-init/10-extensions.sh (the CI / compose.test.yaml
# original) rather than shared by reference, since this stack's
# `POSTGRES_DB` is just the default `postgres` administrative database
# (the 12 real service databases are created afterward by
# 10-databases.sh in this directory), not one particular service's
# test database.
#
# `template1` is done first and deliberately: Postgres copies it for
# every subsequently created database, so 10-databases.sh's
# `CREATE DATABASE` calls inherit these extensions without needing
# superuser DDL of their own. Runs exactly once, on first
# initialisation of an empty data directory, before 10-databases.sh.
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
