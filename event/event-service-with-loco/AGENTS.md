# AGENTS — Event Service

How to work in this crate. The canonical artefact is
[`spec.md`](spec/index.md). When in doubt, the spec wins. See
[`agents/spec-driven-development.md`](agents/spec-driven-development.md)
for the discipline this crate practises.

## Crate-local docs (`agents/`)

| Document | Description |
|----------|-------------|
| [agents/index.md](agents/index.md) | Directory index |
| [agents/spec-driven-development.md](agents/spec-driven-development.md) | SDD discipline — three-part PRs, section mapping, anti-patterns |
| [agents/models.md](agents/models.md) | Domain model reference (schema.org/Event-aligned) |
| [agents/matching.md](agents/matching.md) | Matching algorithm reference (weights, components, rules) |
| [agents/restful.md](agents/restful.md) | REST API + library API reference |
| [agents/testing.md](agents/testing.md) | Testing strategy and guide |

## Shared docs (project root)

Shared reference docs live at the project root under
[`../agents/share/`](../../agents/share/).

| Document | Description |
|----------|-------------|
| [overview.md](../../agents/share/overview.md) | High-level project overview |
| [architecture.md](../../agents/share/architecture.md) | Layered architecture |
| [rust-loco-stack.md](../../agents/share/rust-loco-stack.md) | Full Rust + Loco dependency stack |
| [loco.md](../../agents/share/loco.md) | Tech stack summary |
| [match-search-merge.md](../../agents/share/match-search-merge.md) | Match / search / merge workflows |
| [match.md](../../agents/share/match.md) | Matching algorithms |
| [search.md](../../agents/share/search.md) | Search (Tantivy) |
| [merge.md](../../agents/share/merge.md) | Merge workflow |
| [dataflow.md](../../agents/share/dataflow.md) | Create / search / merge data flows |
| [privacy.md](../../agents/share/privacy.md) | Masking, GDPR, consent |
| [auditability.md](../../agents/share/auditability.md) | Audit logging and event streaming |
| [availability.md](../../agents/share/availability.md) | Health checks, scaling |
| [observability.md](../../agents/share/observability.md) | Tracing + OpenTelemetry (summary) |
| [rust-tracing-opentelemetry-stack.md](../../agents/share/rust-tracing-opentelemetry-stack.md) | Tracing + OpenTelemetry (full) |
| [restful.md](../../agents/share/restful.md) | REST API conventions |
| [postgresql.md](../../agents/share/postgresql.md) | PostgreSQL setup |
| [locales.md](../../agents/share/locales.md) | i18n & l10n |
| [compliance-for-healthcare.md](../../agents/share/compliance-for-healthcare.md) | HIPAA, NHS, … |
| [compliance-for-technology.md](../../agents/share/compliance-for-technology.md) | ISO, GDPR, … |

## Running this crate

```bash
# REST + gRPC API
cargo run --release

# Tests
cargo test --lib                                # unit
cargo test --tests                              # integration (needs DATABASE_URL)
DATABASE_URL=… cargo test --test api_integration_test

# Benchmarks
cargo bench
```

## Durable event bus — outbox relay + `FluvioSink`

`src/relay.rs` is the Phase-3 outbox relay (spec §13 T-11): it drains
`event_outbox` rows to an `EventSink`, default `LoggingSink` (no
broker; dev/CI). The real-broker sink, `FluvioSink` (BUS-3, ported
from case-service's BUS-1 reference), lives behind this crate's own
`fluvio` Cargo feature — **off by default**, so a default build's
dependency tree and behaviour are unchanged. Both the relay loop
(`EVENT_EVENT_TRANSPORT=outbox` + `EVENT_EVENT_RELAY`) and the sink
(`EVENT_FLUVIO_ENDPOINT`) are off unless explicitly configured; an
endpoint configured **without** the `fluvio` feature refuses to start
the relay (logged at `error`) rather than silently marking rows
published without reaching a real broker. `compose.fluvio.yaml` +
`Dockerfile.fluvio-cli` provision a local Fluvio broker for opt-in
manual runs — not part of any automated CI stage; run the crate's
default `cargo test --lib` / `--features fluvio` variants and the
DB-gated suite as usual, and see `tests/fluvio_relay.rs` for the
feature-gated, `#[ignore]`d live-broker round-trip.

## OpenTelemetry OTLP export

`src/observability.rs` (repo `tasks.md` PRO-H9, landed 2026-08-28) is a
close port of person-service's `src/observability.rs` (itself ported
from link-graph-service, the family's first working exporter). It
**replaces** the earlier `src/observability/` stub outright (a JSON
`tracing` subscriber with the OTLP exporter commented out behind
`// TODO: Initialize OTLP exporter`, spread across `mod.rs` +
`metrics.rs` + `traces.rs`, and never wired into `App`'s `Hooks` impl
at all — `init_telemetry`/`shutdown_telemetry` had no caller) rather
than filling the stub in place. `App::init_logger` installs it (loco's
own `EnvFilter` + formatted layer, plus the `tracing-opentelemetry`
bridge over an OTLP/gRPC exporter); `App::on_shutdown` flushes it.
Export is **on by default** — set `OTLP_ENDPOINT=""` to disable it —
at `OTLP_ENDPOINT` (default `http://localhost:4317`) with
`service.name` from `OTLP_SERVICE_NAME` (default `event-service`);
both variables are **deliberately unprefixed**, matching
link-graph-service/person-service and
`agents/share/rust-tracing-opentelemetry-stack.md`'s config table, not
the per-service `EVENT_*` convention `EVENT_REQUIRE_AUTH` and its
siblings use.

**This crate's shape matched person's almost exactly** (both are
"person-style" crates per `agents/share/architecture.md`), so the port
needed the same two adaptations, confirmed rather than assumed:

- **Two router-construction surfaces**, not link-graph-service's one:
  the loco-native one (`api::rest::events_routes()` +
  `controllers::fhir::routes()`, mounted via `App::routes`/
  `App::after_routes`) and a standalone hand-rolled one
  (`api::rest::create_router`, used by the DB-gated integration tests
  and by `controllers::fhir::axum_router` for its FHIR surface). Event
  was flagged as possibly "mid-conversion" with an additional
  `src/controllers/` route-registration surface beyond person's two —
  verified directly rather than assumed, and it does **not** add a
  third: `controllers::fhir` contributes routes to *both* of the
  existing two surfaces (a loco `routes()` registered in `App::routes`,
  and an `axum_router` merged inside `create_router`) rather than
  booting its own router. `observability::trace_mw` is layered as the
  outermost middleware on both surfaces, exactly as person's is.
- **A renamed `tonic` dev-dependency.** This crate already depends on
  `tonic = "0.12"` for its own gRPC stub (`src/api/grpc/`), so the
  in-process OTLP collector tests' `tonic = "0.14"` dev-dependency (used
  to serve the fake collector) is declared as
  `otlp-test-tonic = { package = "tonic", version = "0.14" }` — an
  unrenamed second `tonic` dependency at a different version collides
  in a test binary's extern prelude (`E0464: multiple candidates for
  rlib dependency tonic`).

**One difference from person's port**: this crate has **no
`src/compliance/soup.rs`** — its `src/compliance/` module covers only
row-level integrity (`mac.rs`, `audit_integrity.rs`,
`record_integrity.rs`; see that module's doc comment), with no SOUP
register or SBOM endpoint at all. Person's `renamed_package` parser fix
for a `package = "…"` inline-table rename therefore has nothing to
port here — checked directly rather than assumed absent.

`tests/otlp_export.rs` and `tests/otlp_middleware.rs` (ported from
person-service, with `tests/otlp_collector/` — an in-process OTLP/gRPC
collector, unchanged) prove real export against a real gRPC listener
in a normal `cargo test` run: a `tracing` span and a metric both reach
the collector's decoded protobuf, and a served HTTP request returns a
`traceparent` whose trace id matches the exported span's. None of this
needs a database. `cargo test --lib` grew from 159 to 167 (the eight
new `observability::tests` unit tests); `cargo test --test otlp_export
--test otlp_middleware` is 4 further tests, all green.

## Container image

`Dockerfile` (multi-stage, Debian 13 slim runtime) builds this crate's
production image. **Build context must be the repository root**, not
this directory — this crate's sibling path dependencies
(`integrity-mac`, `authentication-verifier`, and — since the
coordinate-field rename, PRO-H2 — its matcher `event-matcher`, pending
a 0.7.0 crates.io publish, see `Cargo.toml`) live outside
`event/event-service-with-loco/`:

```sh
podman build -f event/event-service-with-loco/Dockerfile \
  -t event-service .   # run from the repository root
```

Verified end-to-end (2026-08-03): builds clean, boots against a real
Postgres, and `GET /api/health` returns `200`. Like person's, this
crate's Dockerfile pre-dated the family's repo-root-context convention
and had the same four bugs, found only by running the built image:
no `config/` copy (boot crash: "no configuration file found in folder:
config"); `CMD` with no `start` subcommand (the loco CLI just prints
`--help` and exits 0); no `LOCO_ENV` (would boot in `development`
inside a `production` image); and a dead `SERVER_PORT` env var — loco's
own config reads `PORT`, not `SERVER_PORT` (the latter is a separate,
unrelated surface this crate's own `src/config/mod.rs` documents for
non-loco code paths). All four fixed; `PORT=8080` is now set
explicitly.

See `.containerignore` at the repository root (excludes every crate's
`target/`, or the build context would try to copy hundreds of GB of
build artifacts). The wired multi-service `examples/compose/` stacks
(DEP-1) that build on this are not yet written.

## Doc hierarchy quick reference

| File | Role |
|------|------|
| `spec.md` | **Single source of truth** — what, how, status, tasks (§13) |
| `README.md` (symlink to `index.md`) | User-facing intro — must stay consistent with the spec |
| `AGENTS.md` / `agents/*.md` / `CLAUDE.md` | How to work in the repo + per-topic reference. `CLAUDE.md` is a one-line `@AGENTS.md` include (matching the family convention — see root `AGENTS.md`), not a second user-facing doc; its content used to duplicate `index.md` and has been folded/removed rather than kept in sync by hand. |
| `index.md` | Navigation aid with worked examples |
| `CHANGELOG.md` | Historical record of releases and changes |
