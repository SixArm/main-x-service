#!/bin/sh
#
# Creates one database per Main X Index service in the shared Postgres
# container the full-family / enforced compose stacks use.
#
# Mounted read-only into /docker-entrypoint-initdb.d/ alongside
# ci/postgres-init/10-extensions.sh (which this file's numbering
# deliberately follows — extensions in template1 first, so every
# database created below inherits them via Postgres's own
# copy-template1-on-CREATE-DATABASE behaviour). Runs exactly once, on
# first initialisation of an empty data directory.
set -eu

for db in person worker place thing event course \
          organization care_pathway case portfolio \
          authentication link_graph; do
  # Double-quoted: "case" is a reserved SQL keyword and an unquoted
  # `CREATE DATABASE case;` is a syntax error. Quoting every name here
  # is a defensive no-op for the rest, but the one that matters.
  psql -v ON_ERROR_STOP=1 --username "${POSTGRES_USER}" --dbname postgres \
    -c "CREATE DATABASE \"${db}\";"
done

echo "full-family: 12 service databases created"
