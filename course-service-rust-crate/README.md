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

> **Status.** MVP scaffold — REST routes return `501 Not Implemented`;
> models, migrations, docs, and the binary boot path are complete.
> Track per-endpoint work in [`spec.md §13 Tasks`](spec.md#13-tasks).

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
cargo test --lib                              # unit tests
DATABASE_URL=… cargo test --test api_integration_test
cargo test --test duplicate_detection         # bridge tests (planned T-11)
cargo bench                                   # criterion benches (planned T-13)
```

See [`AGENTS/testing.md`](AGENTS/testing.md) for the test layout
and [`docker-compose.test.yml`](docker-compose.test.yml) for the test
Postgres bring-up.

## Compliance

- **GDPR**: right of access via `GET /api/courses/{id}/export`;
  right to erasure via soft-delete + `/masked` view.
- **FERPA**: masked view conceals instructor / student identifiers
  on `CourseInstance` records; audit log preserves access trail.

## Status

- **MVP scaffold**: complete. Models, migrations, REST routes,
  binary boot path, docs.
- **Next milestones** (T-2..T-7): SeaORM entities, repository CRUD,
  Tantivy search engine, validation, matcher adapter, REST handler
  implementations.

## License

Dual-licensed under MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only
OR GPL-3.0-only.
