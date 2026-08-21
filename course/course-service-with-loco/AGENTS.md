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

## Container image

`Dockerfile` (multi-stage, Debian 13 slim runtime) builds this crate's
production image. **Build context must be the repository root**, not
this directory — this crate's sibling path dependencies
(`integrity-mac`, `authentication-verifier`; its matcher,
`course-matcher`, is a crates.io registry dependency, not a path
dependency, despite an earlier version of this Dockerfile assuming
otherwise) live outside `course/course-service-with-loco/`:

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
