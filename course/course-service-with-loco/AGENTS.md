# AGENTS — Course Service

How to work in this crate. The canonical artefact is
[`spec.md`](spec/index.md). When in doubt, the spec wins. See
[`agents/spec-driven-development.md`](agents/spec-driven-development.md)
for the discipline this crate practises.

## Crate-local docs (`agents/`)

| Document | Description |
|---|---|
| [agents/index.md](agents/index.md) | Directory index |
| [agents/spec-driven-development.md](agents/spec-driven-development.md) | SDD discipline — three-part PRs, section mapping, anti-patterns |
| [agents/models.md](agents/models.md) | Domain model reference (`Course`, `CourseInstance`, schema.org property mapping) |
| [agents/matching.md](agents/matching.md) | Matching algorithm reference (weights, rules, components) |
| [agents/restful.md](agents/restful.md) | REST API + library API reference |
| [agents/testing.md](agents/testing.md) | Testing strategy and guide |

## Shared docs (project root)

Shared reference docs live at the project root under
[`../agents/share/`](../../agents/share/).

| Document | Description |
|---|---|
| [overview.md](../../agents/share/overview.md) | High-level project overview |
| [architecture.md](../../agents/share/architecture.md) | Layered architecture |
| [rust-loco-stack.md](../../agents/share/rust-loco-stack.md) | Full Rust + Loco dependency stack |
| [loco.md](../../agents/share/loco.md) | Tech stack summary |
| [match-search-merge.md](../../agents/share/match-search-merge.md) | Match / search / merge workflows |
| [restful.md](../../agents/share/restful.md) | REST API conventions |
| [postgresql.md](../../agents/share/postgresql.md) | PostgreSQL conventions |
| [auditability.md](../../agents/share/auditability.md) | Audit-log conventions |
| [privacy.md](../../agents/share/privacy.md) | Masking, GDPR, consent |
| [observability.md](../../agents/share/observability.md) | Tracing + OpenTelemetry summary |

## Where work lives

| Concern | Location |
|---|---|
| Behavioural truth | [`spec.md`](spec/index.md) (§1–§18; live work queue in §13) |
| Domain models | `src/models/` |
| REST handlers | `src/api/rest/handlers.rs` |
| Database access | `src/db/` |
| Search index | `src/search/` |
| Matcher adapter | `src/matching/` (thin wrapper over [`course-matcher`](../course-matcher-rust-crate/)) |
| Validation | `src/validation/` (T-5, FR-21..FR-28) |
| Privacy | `src/privacy/` (T-10, mask + GDPR Article-15 export) |
| Audit log | `src/db/audit.rs` (T-9) |
| Event streaming | `src/streaming/` (T-9, in-memory MVP) + the durable transactional-outbox bus: `src/db/outbox.rs` (Phase 2, the `course_outbox` table, T-21) and `src/relay.rs` (Phase 3 relay + retention, T-22; the real-broker `FluvioSink` behind the `fluvio` cargo feature, T-23/BUS-3) |
| Bridge tests | [`tests/duplicate_detection.rs`](tests/duplicate_detection.rs) (T-11) |
| Integration tests | [`tests/api_integration_test.rs`](tests/api_integration_test.rs) (T-12, `#[ignore]`-tagged) |
| Benchmarks | `benches/` (T-13, three criterion files) |
| OpenAPI | served at `/swagger-ui` + `/api-docs/openapi.json` (T-14) |
| Metrics | `src/metrics.rs` (process-wide Prometheus registry, `OnceLock`); served at root `/metrics.prom` via `metrics_routes()` in `src/api/rest/mod.rs` (T-16) |
| Migrations | `migrations/` |

## OpenTelemetry OTLP export

`src/observability.rs` (repo `tasks.md` PRO-H12, landed 2026-08-30) is a
close port of person-service's `src/observability.rs` — itself a port of
link-graph-service's, the family's first working exporter. Person, not
link-graph-service, was the copy source here for the same reason worker
and event's PRO-H9 ports were: person had already solved the
two-router-construction-surfaces adaptation this crate's shape also
needs. This module is genuinely new — this crate carried **no**
`src/observability` module at all before this change, unlike
person/worker/event, which each had a dead stub to replace (PRO-H9).
`App::init_logger` installs it (loco's own `EnvFilter` + formatted
layer, plus the `tracing-opentelemetry` bridge over an OTLP/gRPC
exporter); `App::on_shutdown` flushes it. Export is **on by default** —
set `OTLP_ENDPOINT=""` to disable it — at `OTLP_ENDPOINT` (default
`http://localhost:4317`) with `service.name` from `OTLP_SERVICE_NAME`
(default `course-service`); both variables are **deliberately
unprefixed**, matching every other crate that carries this pipeline and
`agents/share/rust-tracing-opentelemetry-stack.md`'s config table, not
the per-service `COURSE_*` convention `COURSE_REQUIRE_AUTH` and its
siblings use.

**Where this crate's shape forced real adaptation** — confirmed rather
than assumed:

- **Two router-construction surfaces**, exactly as person/worker/event
  needed: this crate carries the loco-native one
  (`api::rest::courses_routes()`, mounted via `App::routes`/
  `App::after_routes`) and a standalone hand-rolled one
  (`api::rest::create_router`, used only by the DB-gated integration
  tests) — where link-graph-service has exactly one (pure loco). The
  `observability::trace_mw` tower middleware (per-request span +
  `http.server.request.duration` histogram + W3C `traceparent` response
  header) is layered onto **both**, as the outermost layer in each, so
  tracing behaves identically regardless of which router a caller or
  test builds — the same precedent `auth::require_auth_mw` already set
  by being layered on both surfaces. It is a **second**, complementary
  middleware to the existing `metrics::track_http_requests_mw` (T-18's
  Prometheus counter) — the two are independent sinks layered side by
  side, not a replacement of one by the other.
- **No `tonic` rename needed** — the one adaptation person/worker/event
  all needed that this crate does **not**: this crate carries no gRPC
  stub of its own (`agents/share/overview.md`'s capability matrix —
  course is `–` on gRPC), so the in-process OTLP collector tests' `tonic
  0.14` dev-dependency is declared as a plain `tonic = "0.14"` — no
  `package = "…"` rename, and no matching SOUP-register fix (this crate
  carries no SOUP register at all — see `agents/models.md`'s peers'
  notes on that).

`tests/otlp_export.rs` and `tests/otlp_middleware.rs` (ported from
person-service, with `tests/otlp_collector/` — an in-process OTLP/gRPC
collector, unchanged from link-graph-service's original bar the
un-renamed `tonic` import) prove real export against a real gRPC
listener in a normal `cargo test` run: a `tracing` span and a metric
both reach the collector's decoded protobuf, and a served HTTP request
returns a `traceparent` whose trace id matches the exported span's. None
of this needs a database. Landing this raised `cargo test --lib` from
124 to 132 (8 new `src/observability.rs` unit tests), plus 4 new tests
across the two `tests/otlp_*.rs` binaries. Verified independently:
`cargo fmt --check` clean, `cargo clippy --all-targets -- -D warnings`
clean, `cargo deny check` clean, MSRV check (`cargo +1.96 check
--all-targets`) clean.

## Container image

`Dockerfile` (multi-stage, Debian 13 slim runtime) builds this crate's
production image. **Build context must be the repository root**, not
this directory — this crate's sibling path dependencies
(`integrity-mac`, `authentication-verifier`, and — as of
`course-matcher` 0.7.0 / PRO-H7 — its matcher `course-matcher` itself,
previously a crates.io-only registry dependency with no sibling COPY)
live outside `course/course-service-with-loco/`:

```sh
podman build -f course/course-service-with-loco/Dockerfile \
  -t course-service .   # run from the repository root
```

Verified end-to-end (2026-08-03): builds clean, boots against a real
Postgres, and `GET /api/health` returns `200`. This crate's Dockerfile
already used a build context one level above this directory
(`course/`), but never copied `integrity-mac` or
`authentication-verifier` (which live outside `course/` entirely) —
so it was just as broken as person/worker/event's `context: .`, only
by a different sibling-dependency gap. Fixed to the repo-root
convention, plus the same three further bugs found by actually running
the built image (not merely getting `podman build` to succeed): no
`config/` copy (boot crash: "no configuration file found in folder:
config"); `CMD` with no `start` subcommand (the loco CLI just prints
`--help` and exits 0); and no `LOCO_ENV` (would boot in `development`
inside a `production` image). Also: this crate's own
`config/production.yaml` defaults `PORT` to `8084` (its siblings
default to `8080`), so `PORT=8080` is now set explicitly to match
`EXPOSE`/`HEALTHCHECK` rather than relying on the family's usual
default.

See `.containerignore` at the repository root (excludes every crate's
`target/`, or the build context would try to copy hundreds of GB of
build artifacts). The wired multi-service `examples/compose/` stacks
(DEP-1) that build on this are not yet written.
