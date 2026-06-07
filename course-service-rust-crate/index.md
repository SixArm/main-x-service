# Course Service

A registry of **course identities** based on
[schema.org/Course](https://schema.org/Course). The Course Service is
the abstract template (CS101 — Introduction to Computer Science);
its `CourseInstance` sub-resource is the specific offering (CS101,
Fall 2026, Prof. Smith, in-person). One course → many instances.

Sits between the [Thing Service](../thing-service-rust-crate/)
(anything with an identity) and the
[Event Service](../event-service-rust-crate/) (occurrences with
locations and parties).

> **Status.** Production-ready MVP. FR-1..FR-9 (CRUD / search /
> match / merge / dedup) + FR-10..FR-13 (instance sub-resource) +
> FR-14..FR-18 (audit / streaming / privacy) are all wired. Only
> JWT auth (T-15) remains, blocked on the family-wide auth rollout.
> See [`spec.md §13`](spec.md#13-tasks) for the per-task ledger.

## Quick start

### Option 1: Podman compose (recommended)

```bash
# From the repo root because the Dockerfile pulls in the sibling
# course-matcher crate via the path dependency.
cd course-service-rust-crate
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
# Prerequisites: Rust 1.93+, PostgreSQL 17+, podman (optional).
cp .env.example .env

# Set up the database.
podman run -d --name course-postgres -p 5434:5432 \
  -e POSTGRES_DB=course \
  -e POSTGRES_USER=course_user \
  -e POSTGRES_PASSWORD=course_password \
  postgres:17-alpine

# Apply migrations (manual — auto-migrate is out of scope for MVP).
for m in migrations/*/up.sql; do
  podman exec -i course-postgres psql -U course_user -d course < "$m"
done

# Build and run.
cargo run --release
```

## API

REST routes mount under `/api/courses/*` and `/api/courses/{id}/instances/*`.
See [`AGENTS/restful.md`](AGENTS/restful.md) for the full list. All
endpoints return the standard `{success, data, error}` envelope.

Interactive OpenAPI 3 documentation ships with the binary:

- Swagger UI: `http://localhost:8084/swagger-ui`
- Raw spec: `http://localhost:8084/api-docs/openapi.json`

The Event Service uses `/api/v1/`; Course does NOT — direct `/api`.

## Configuration

| Variable | Description | Default |
|---|---|---|
| `DATABASE_URL` | Postgres connection string | — |
| `DATABASE_MAX_CONNECTIONS` | Pool max | `10` |
| `DATABASE_MIN_CONNECTIONS` | Pool min | `2` |
| `SERVER_HOST` | REST bind address | `0.0.0.0` |
| `SERVER_PORT` | REST port | `8080` |
| `SEARCH_INDEX_PATH` | Tantivy index directory | `./data/search_index` |
| `MATCHING_THRESHOLD` | Probabilistic match cutoff | `0.85` |
| `RUST_LOG` | tracing-subscriber filter | `info` |
| `OTLP_ENDPOINT` | OpenTelemetry collector | `http://localhost:4317` |

## Testing

```bash
# 35 unit tests (matcher facade, search index, validation, db
# helpers, streaming, privacy, repository conversions).
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
- **Tests**: 35 unit + 14 bridge + 12 #[ignore]-tagged integration
  (T-12) + 3 criterion benches (T-13).
- **Auth**: JWT (T-15) blocked on the family-wide rollout.

## License

Dual-licensed under MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only
OR GPL-3.0-only.
