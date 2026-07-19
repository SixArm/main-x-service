# Course Service

A high-performance, enterprise-grade Course Service system built with Rust.

A registry of **course identities** based on
[schema.org/Course](https://schema.org/Course). The Course Service is
the abstract template (CS101 — Introduction to Computer Science);
its `CourseInstance` sub-resource is the specific offering (CS101,
Fall 2026, Prof. Smith, in-person). One course → many instances.

Sits between the [Thing Service](../../thing/thing-service-with-loco/)
(anything with an identity) and the
[Event Service](../../event/event-service-with-loco/) (occurrences with
locations and parties).

> **Status.** Production-ready MVP. FR-1..FR-9 (CRUD / search /
> match / merge / dedup) + FR-10..FR-13 (instance sub-resource) +
> FR-14..FR-18 (audit / streaming / privacy) are all wired, plus the
> family-wide auth guard (T-15): offline PASETO v4.public bearer
> verification + ABAC blanket enforcement on `/api/*` and `/fhir/*`,
> **default-off** via `COURSE_REQUIRE_AUTH` (see
> [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)).
> See [`spec.md §13`](spec/13-tasks.md) for the per-task ledger.

## Quick start

### Option 1: Podman compose (recommended)

```bash
# From the repo root because the Dockerfile pulls in the sibling
# course-matcher crate via the path dependency.
cd course/course-service-with-loco
cp .env.example .env

# Brings up postgres + course-service.
podman compose up -d

# Wait for healthy:
podman compose logs -f course-service

# Service on host port 8084 (avoids clashing with person-service on 8080).
curl http://localhost:8084/api/health
```

### Option 2: native build

```bash
# Prerequisites: Rust 1.95+ (2024 edition), PostgreSQL 18+, podman (optional).
cp .env.example .env

# Set up the database.
podman run -d --name course-postgres -p 5434:5432 \
  -e POSTGRES_DB=course \
  -e POSTGRES_USER=course_user \
  -e POSTGRES_PASSWORD=course_password \
  postgres:18-alpine

# Build and run (loco.rs): point DATABASE_URL at the database, then
# start. Migrations run automatically in development (config auto_migrate).
export DATABASE_URL=postgres://course_user:course_password@localhost:5434/course
cargo loco start            # or: cargo run -- start

# Migrations can also be run explicitly:
cargo loco db migrate
```

## API

REST routes mount under `/api/courses/*` and `/api/courses/{id}/instances/*`.
See [`AGENTS/restful.md`](AGENTS/restful.md) for the full list. All
endpoints return the standard `{success, data, error}` envelope.

Interactive OpenAPI 3 documentation ships with the binary:

- Swagger UI: `http://localhost:8084/swagger-ui`
- Raw spec: `http://localhost:8084/api-docs/openapi.json`

Prometheus metrics are served at the application root:

- `GET /metrics.prom` — text-exposition format (`text/plain; version=0.0.4`), public (no bearer token). Counters: `course_created_total`, `course_updated_total`, `course_deleted_total`, `course_merged_total`, plus a labelled `http_requests_total{path,status}`. Configure your scraper with `metrics_path: /metrics.prom`.

The Event Service uses `/api/`; Course does NOT — direct `/api`.

### Worked example — duplicate on create (409, FR-1 / FR-20)

`POST /api/courses` blocks against existing records before inserting.
A deterministic short-circuit (e.g. a shared `provider_id` +
`course_code`, or a matching DOI / Wikidata / `same_as` URL) pins the
score and the create is rejected with `409 Conflict`; the ranked
candidates ride under `error.details` as a `ScoredCandidate[]`:

```bash
curl -s -X POST http://localhost:8084/api/courses \
  -H 'content-type: application/json' \
  -d '{"name":"Introduction to Computer Science","course_code":"CS101","provider_id":"3f1a…"}'
```

```jsonc
// HTTP/1.1 409 Conflict
{
  "success": false,
  "data": null,
  "error": {
    "code": "DUPLICATE",
    "message": "Potential duplicate course(s) detected",
    "details": [
      {
        "course_id": "9c2b…",          // existing record this clashes with
        "name": "Intro to Computer Science",
        "course_code": "CS101",
        "score": 1.0,                   // deterministic short-circuit
        "quality": "certain"
      }
    ]
  }
}
```

Re-POST with a distinct `course_code` (or call
`POST /api/courses/check-duplicates` first to preview the candidates
without writing).

### Worked example — scrape metrics (T-16)

```bash
curl -s http://localhost:8084/metrics.prom
```

```text
# HELP course_created_total Total course records created.
# TYPE course_created_total counter
course_created_total 12
# HELP course_merged_total Total course merges performed.
# TYPE course_merged_total counter
course_merged_total 3
```

The labelled `http_requests_total{path,status}` family is registered
but emits no sample line until a request-path middleware records one
(reserved; see spec §15 v0.4).

## Configuration

Server binding, logger, database pool, and the background queue are
owned by the loco environment config in `config/<environment>.yaml`
(development binds `localhost:8084`; `DATABASE_URL` is interpolated
into the `database` and `queue` blocks). Domain knobs still come
from the environment via `Config::from_env`:

| Variable                   | Description                | Default                 |
| -------------------------- | -------------------------- | ----------------------- |
| `DATABASE_URL`             | Postgres connection string | —                       |
| `SEARCH_INDEX_PATH`        | Tantivy index directory    | `./data/search_index`   |
| `MATCHING_THRESHOLD`       | Probabilistic match cutoff | `0.85`                  |
| `RUST_LOG`                 | tracing-subscriber filter  | `info`                  |
| `OTLP_ENDPOINT`            | OpenTelemetry collector    | `http://localhost:4317` |

## Testing

```bash
# 109+ unit tests (matcher facade, search index, validation, db
# helpers, streaming, privacy, metrics, router + handlers, fhir);
# run for the live count.
cargo test --lib

# 14 bridge tests pinning the service ↔ canonical course-matcher
# contract (identical clones, deterministic short-circuits, per-enum
# routing, config presets).
cargo test --test duplicate_detection

# 12 DB-backed integration tests. Skipped by default — opt in with
# `--ignored` after the Postgres bring-up below.
DATABASE_URL=postgres://course_user:course_password@localhost:5434/course \
  cargo test --test api_integration_test -- --ignored

# Three criterion benches (matching, search, validation).
cargo bench
```

See [`AGENTS/testing.md`](AGENTS/testing.md) for the layout and
[`docker-compose.yml`](docker-compose.yml) for the dev Postgres
bring-up the integration suite expects to be migrated against.

## Compliance

- **GDPR**: right of access via `GET /api/courses/{id}/export`;
  right to erasure via soft-delete + `/masked` view.
- **FERPA**: masked view conceals instructor / student identifiers
  on `CourseInstance` records; audit log preserves access trail.

## Status

- **Persistence**: SeaORM entities + transactional repository CRUD
  (T-2 / T-3); CourseInstance sub-resource (T-8); merge bookkeeping.
- **Search**: Tantivy `SearchEngine` with exact / fuzzy / blocking
  queries; reader reload after every commit (T-4).
- **Matching**: canonical [`course-matcher`](../course-matcher-rust-crate/)
  driven through the service-side adapter (T-6); 14 bridge tests
  pin the contract (T-11).
- **Validation**: FR-21..FR-28 with nested-instance path prefixes
  (T-5).
- **REST**: FR-1..FR-9 + FR-14..FR-18 wired (T-7 / T-8 / T-9 / T-10);
  OpenAPI via utoipa (T-14).
- **Audit + streaming**: `AuditLogRepository` + in-memory
  `EventPublisher` MVP (T-9). Fluvio adapter under feature flag
  pending.
- **Privacy**: `mask_course` + GDPR Article-15 export (T-10).
- **Metrics**: Prometheus `GET /metrics.prom` (T-16) — process-wide
  registry, CRUD/merge counters, reserved labelled
  `http_requests_total`.
- **Tests**: 109+ unit + 14 bridge + 12 #[ignore]-tagged integration
  (T-12) + 3 criterion benches (T-13).
- **Auth**: offline PASETO v4.public bearer verification + ABAC blanket
  guard on `/api/*` and `/fhir/*` (T-15), **default-off** via
  `COURSE_REQUIRE_AUTH`; `GET /api/whoami` echoes verified claims. See
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md).

## License

Dual-licensed under MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only
OR GPL-3.0-only.
