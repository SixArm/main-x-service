# Availability

Monorepo-wide reference for how the **Main X Index** family achieves
availability, horizontal scaling, and resilient deployment. This is the
comprehensive spec; the short version lives at
[`agents/share/availability.md`](../../agents/share/availability.md).

Every entity ships as an independent service (its own binary, its own
database, its own container) — there is no shared runtime and no shared
database. Availability is therefore a *per-service* property that the
whole family achieves the same way: stateless processes behind a load
balancer, all durable state in PostgreSQL, health-checked containers.

> Related: [postgresql](../postgresql/index.md) (pool sizing, backups),
> [authentication](../authentication/index.md) (Postgres-backed rate
> limiter), [architecture](../architecture/index.md),
> [restful](../restful/index.md),
> [search](../search/index.md) (Tantivy index — the one node-local
> volume), event streaming
> ([`agents/share/event-bus.md`](../../agents/share/event-bus.md)),
> observability
> ([`agents/share/observability.md`](../../agents/share/observability.md)),
> and the stack
> ([`agents/share/rust-loco-stack.md`](../../agents/share/rust-loco-stack.md)).

## 1. Stateless design enables horizontal scaling

The services are designed to be **stateless processes**: a request is
served entirely from durable shared state (PostgreSQL) plus the request
body, so any replica can serve any request and replicas can be added or
removed freely. Nothing the orchestrator needs to keep a session alive
lives in process memory.

Two in-process exceptions existed historically; both are resolved or on
a path to resolution:

| Per-instance state | Status | Coordination |
|---|---|---|
| **Rate limiter** (magic-link throttle) | **Postgres-backed** — `auth_rate_limits` table + `pg_advisory_xact_lock(hashtext(email_key))`. Correct across replicas. | Shared DB row; transaction-scoped advisory lock. See [authentication](../authentication/index.md) and [postgresql §9](../postgresql/index.md). |
| **Event stream** (CRUD fan-out) | **In-memory ring buffer today** — `OnceLock` ring (cap 1000, per-process `seq`) in the loco services; `EventProducer` trait in the legacy Axum services. Process-local and volatile. | Not yet cross-replica correct. Durable bus is the roadmap (§8). See [`agents/share/event-bus.md`](../../agents/share/event-bus.md). |

Everything else — entity records, audit log, merge history, review
queue, sessions, background-job queue — is in PostgreSQL. The only
node-local persistent artifact is the **Tantivy search index** volume
(§5, §6), which is a per-replica cache rebuildable from the database,
not a source of truth.

The practical rule: **a request must never depend on which replica
served the previous request.** Sticky sessions are not required and
should not be configured.

## 2. Database connection pooling

Each service holds a SeaORM/`sqlx` connection pool to its own database.
Pooling plus statelessness is what makes horizontal scaling safe — see
[postgresql §3](../postgresql/index.md) for the authoritative table.

**loco services** configure the pool in `config/*.yaml` under
`database:`:

| Key | Dev default | Production guidance |
|---|---|---|
| `connect_timeout` | `500` ms | acquire-connection timeout |
| `idle_timeout` | `500` ms | idle duration before a pooled connection closes |
| `min_connections` | `1` (dev) | raise to keep warm connections ready |
| `max_connections` | `1` (dev) | **raise for production** — primary scaling knob |
| `auto_migrate` | `true` (dev) | **`false` in production**; migrate as a deploy step |

**Legacy Axum services** read the same values from the environment
(`DATABASE_MAX_CONNECTIONS`, `DATABASE_MIN_CONNECTIONS`), wired through
`docker-compose.yml` — e.g. person-service defaults to
`DATABASE_MAX_CONNECTIONS=10`, `DATABASE_MIN_CONNECTIONS=2`. The
person-service `config/production.yaml` ships `min_connections: 2`,
`max_connections: 20`.

### Sizing rule

The total connections opened across all replicas must stay below the
PostgreSQL server's `max_connections`, leaving headroom for migrations,
admin tooling (pgAdmin), and background-job workers:

```
replicas × max_connections_per_replica  <  server max_connections − headroom
```

Worked example: a Postgres server with `max_connections = 100`, six
replicas, and ~10 connections of headroom → at most
`(100 − 10) / 6 ≈ 15` connections per replica. Oversizing the pool
exhausts the server and makes *adding* a replica reduce availability —
the opposite of the goal. Use a server-side pooler (PgBouncer in
transaction mode) when replica counts grow.

## 3. Health checks

Health endpoints exist for orchestrator **liveness** (is the process
up?) and **readiness** (can it serve traffic, including DB
connectivity?) probes, and for container `HEALTHCHECK` directives.

| Endpoint | Provided by | Purpose |
|---|---|---|
| `GET /_health` | loco services (organization, care-pathway, case, authentication) | loco's built-in readiness check (verifies DB/queue) |
| `GET /_ping` | loco services | loco's lightweight liveness ping |
| `GET /api/health` | legacy Axum services (Dockerfile `HEALTHCHECK`) | service health check |
| `GET /api/health` | event-service and some compose health probes | versioned health check |

This split is **accepted drift**: the loco services use loco's
conventional `/_health` + `/_ping`; the older Axum services predate the
conversion and keep `/api/health` (the event-service mounts under
`/api`). Orchestrator probes and load-balancer checks must target
the endpoint the specific service actually exposes.

The container `HEALTHCHECK` runs `curl --fail` against the health
endpoint on an interval (person-service: `--interval=30s --timeout=3s
--start-period=10s --retries=3`); the compose service health probe uses
a longer `start_period` (40s) to allow first-boot warm-up before the
container is marked unhealthy.

## 4. Graceful shutdown & non-root execution

- **Graceful shutdown.** Services drain in-flight requests on `SIGTERM`
  / `SIGINT` (Axum/loco shutdown signal) rather than dropping
  connections, so rolling deploys and replica scale-down do not return
  errors to in-flight callers. In-flight DB transactions either commit
  or roll back cleanly; pool connections close on drain.
- **Non-root container execution.** Containers run as an unprivileged
  user, never root. The person-service image creates
  `useradd --uid 1000 person`, `chown`s `/app`, and switches with
  `USER person` before copying the binary — so a container escape does
  not yield root, and rootless Podman maps host UID 1000 → container UID
  1000 cleanly for bind-mounted volumes.

## 5. Containerization

The family standardizes on **Podman, not Docker** (the `Dockerfile` /
`docker-compose.yml` filenames stay Docker-compatible, so Docker works
if a contributor has it, but Podman is the supported runtime — see
[`rust-loco-stack.md`](../../agents/share/rust-loco-stack.md)).

| Aspect | Choice | Rationale |
|---|---|---|
| Runtime | **Podman** (rootless) | no daemon, rootless by default, drop-in CLI |
| Build | **multi-stage** | `rust:1.93-slim` builder stage → `debian:13-slim` runtime stage; the toolchain never ships in the runtime image |
| Base image | **Debian 13 slim** (Trixie) | small, current stable, `libpq5`/`libssl3` available |
| Allocator | **MiMalloc** for MUSL static builds | faster allocator under `cfg(target_env = "musl")`; see the global-allocator snippet in [`rust-loco-stack.md`](../../agents/share/rust-loco-stack.md) |
| Health | container `HEALTHCHECK` | `curl --fail` the health endpoint (§3) |
| Security | non-root `USER` (§4) | unprivileged runtime user |

The runtime image installs only the runtime shared libraries
(`libpq5`, `libssl3`, `ca-certificates`, `curl`); build-only packages
(`pkg-config`, `libssl-dev`, `libpq-dev`, `gcc`, `make`, `perl`) stay in
the builder stage and are discarded.

### Compose for dev / test / prod

**Test (every service crate, uniform).** `compose.test.yaml` brings up
one container — `postgres:18-alpine`, superuser `loco`/`loco` on port
5432, the database that crate's `config/test.yaml` names, the shared
extension init from `ci/postgres-init/`, and PGDATA on **tmpfs** so each
`up` starts from a clean initdb. It is deliberately the same shape CI
provides (`.github/workflows/ci.yml` `test-db`), so a suite that passes
locally passes there for the same reasons. Driven by
`scripts/test-db.sh {up|down|psql|logs|url|status|down-all}`; the tests
themselves run on the host, not in a container. There is no `restart`
policy and no named volume, because nothing about a test database should
survive it.

**Dev (four older crates only: person, worker, event, course).**
`docker-compose.yml` brings up:

- **`postgres`** — `postgres:18-alpine`, `restart: unless-stopped`, with
  its own `pg_isready` healthcheck, on a named volume.
- **the service** — built from the local `Dockerfile`, with
  `depends_on: postgres (condition: service_healthy)` so it only starts
  once the database is accepting connections, plus its own HTTP
  healthcheck.
- **pgAdmin** (optional, `--profile tools`) — DB administration UI, off
  by default.

These four are the only crates with a dev compose file, and they still
carry the Docker-era filename; the family-wide dev/prod compose is
[`tasks.md`](../../tasks.md) DEP-1.

Production uses a built-and-tagged image
(`podman build -t <service>:vX.Y.Z .` then `podman run`), an externally
managed PostgreSQL (or a managed cloud database), and
`auto_migrate: false` with migrations run as an explicit deploy step.

## 6. Horizontal scaling specifics

The deployment shape for scale is **N stateless service replicas behind
a load balancer**, all pointed at one logical (per-service) PostgreSQL:

```
            +-------------------+
   clients →|  Load Balancer    |   (health-check probes the §3 endpoint)
            +---+-----+-----+---+
                |     |     |
            +---v-+ +-v---+ +-v---+
            | svc | | svc | | svc |   N stateless replicas
            +--+--+ +--+--+ +--+--+
               \      |      /
                \     |     /
              +--------v---------+
              |  PostgreSQL      |   one DB per service (not shared)
              |  (per service)   |   system of record for all shared state
              +------------------+
```

What scales cleanly and what needs coordination:

| Concern | Cross-replica behaviour |
|---|---|
| **Entity CRUD / read** | Clean — all state in Postgres; any replica serves any request; pool-size the connections (§2). |
| **Database** | One database per service (no cross-DB access). Scale reads with Postgres read replicas if needed; writes go to the primary. |
| **Rate limiter** | **Correct across replicas today** — `auth_rate_limits` row + `pg_advisory_xact_lock` make the throttle exact regardless of which replica handles the request. See [authentication](../authentication/index.md), [postgresql §9](../postgresql/index.md). |
| **Event stream** | **Not yet cross-replica correct** — the in-memory ring is per-process, so `recent(limit)` only reflects events that replica emitted, and a subscriber bound to one replica misses events from the others. This is the one feature that constrains multi-instance correctness until the durable bus lands (§8). |
| **Search index** | Each replica keeps its own node-local Tantivy volume (a cache, not source of truth); rebuildable from the database. Today's loco services use Postgres `ILIKE` instead, which is stateless and scales freely. See [search](../search/index.md). |
| **Background jobs** | Postgres-backed queue (loco `bg_pg` / `queue.kind: Postgres`) in the service's own DB — workers across replicas coordinate through the shared queue tables. |

The load balancer should use the service's health endpoint (§3) for
member health and may round-robin or least-connections; **no sticky
sessions** are required because the design is stateless (§1).

## 7. Backups & disaster recovery

Backup and restore are per-database, since each service owns its own DB:
`pg_dump` per database, restore into the matching per-service name. See
[postgresql §13](../postgresql/index.md) for the operational detail
(pooling/health/backups/observability). Standard Postgres DR applies —
WAL archiving / point-in-time recovery and (for read-scale and warm
standby) streaming replicas — configured at the database tier, outside
the stateless service processes.

## 8. Implemented vs. roadmap

| Capability | Status |
|---|---|
| Stateless services + horizontal scaling | **Implemented** |
| Per-service connection pooling (config / env) | **Implemented** |
| Health checks (`/_health` + `/_ping`; `/api/health` legacy) | **Implemented** |
| Graceful shutdown | **Implemented** |
| Non-root, multi-stage Podman / Debian 13-slim images | **Implemented** |
| MiMalloc for MUSL static builds | **Implemented** |
| Compose for dev / test (+ optional pgAdmin) | **Implemented** |
| Postgres-backed rate limiter (cross-replica correct) | **Implemented** |
| Per-database `pg_dump` backups | **Implemented** |
| **Durable event bus** for true multi-instance fan-out | **Roadmap** — move the in-memory ring to a durable, replayable bus (Fluvio transport + transactional outbox); design in [`agents/share/event-bus.md`](../../agents/share/event-bus.md). Until then, event subscription/fan-out is only correct on a single instance. |
| Tantivy full-text search across the family | **Roadmap** (loco services use `ILIKE` interim — see [search](../search/index.md)) |
| Read replicas / PgBouncer pooler at scale | **Roadmap** (sizing rule in §2 holds in the interim) |
