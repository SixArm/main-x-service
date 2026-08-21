### PostgreSQL

PostgresSQL version 18

- SQL schema with tables, indexes, audits, soft deletes.

### PostgreSQL extensions

| Name               | Purpose                                         |
| ------------------ | ----------------------------------------------- |
| pg_stat_statements | record execution statistics                     |
| uuid-ossp          | generate random UUID v4 identifiers             |
| pg_vector          | for similarity search and RAG use case          |
| pgcrypto           | cryptography securty, encryption, hashing       |
| pg_trgm            | trigram search for autocomplete fuzzy matching  |
| postgis            | geographic information system for map locations |
| citext             | case-insenstive text field for matching         |
| unaccent           | helps text search by removing diacritics        |

### The local test database (Podman)

Every service crate carries a **`compose.test.yaml`**: one
`postgres:18-alpine` container providing exactly what that crate's
DB-gated suite needs, and matching what CI provides
(`.github/workflows/ci.yml` `test-db`) — superuser `loco`/`loco`, port
5432, the database its `config/test.yaml` names. Podman, not Docker
(see [rust-loco-stack.md](rust-loco-stack.md)).

```sh
scripts/test-db.sh up   <crate>          # start; waits for the healthcheck
scripts/ci-check.sh test-db <crate>      # run the suite as CI runs it
scripts/test-db.sh down <crate>          # psql · logs · url · status · down-all
TEST_DB_PORT=5433 scripts/test-db.sh up <crate>   # a second one alongside
```

Three properties worth knowing:

- **PGDATA is on tmpfs**, so every `up` is a fresh `initdb` and nothing
  survives a `down`. A test database that accumulates state is the
  difference between a real failure and a stale one. (In the 18 image
  PGDATA is `/var/lib/postgresql/18/docker` — *not* the
  `/var/lib/postgresql/data` of earlier majors, which is why a tmpfs
  mount at the old path silently did nothing.)
- **Extensions come from one shared init script**
  ([`ci/postgres-init/`](../../ci/postgres-init/), mounted read-only into
  every container rather than copied per service). It enables `uuid-ossp`,
  `citext`, `unaccent`, `pg_trgm`, and `pgcrypto` in **`template1`**, so
  every database created afterwards — including the `ci_*` databases
  `scripts/ci-check.sh` creates per crate — inherits them.
- **`up` checks the published port from outside**, not just the
  container's healthcheck. On macOS podman publishes on IPv6 `*`, so a
  Postgres already holding IPv4 `127.0.0.1:<port>` wins the `localhost`
  lookup and answers instead — the container is healthy, the connection
  succeeds, and the error you get is "database does not exist", which
  reads like a broken container. The script detects that and tells you to
  pick a free `TEST_DB_PORT`.
- **Only those five extensions are enabled.** They are the ones the
  migrations and CI actually use (verified by grep, not assumption).
  `pg_stat_statements` needs `shared_preload_libraries` and buys a test
  run nothing; `pg_vector` and `postgis` are not in the stock image and
  no migration references them. A service that starts needing either
  changes its own `compose.test.yaml` image — which is the point of the
  file being per-service.
