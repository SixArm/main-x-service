# AGENTS — Thing Service

How to work in this crate. The canonical artefact is
[`spec.md`](spec/index.md). When in doubt, the spec wins. See
[`agents/spec-driven-development.md`](agents/spec-driven-development.md)
for the discipline this crate practises.

## Crate-local docs (`agents/`)

| Document | Description |
|----------|-------------|
| [agents/index.md](agents/index.md) | Directory index |
| [agents/spec-driven-development.md](agents/spec-driven-development.md) | SDD discipline — three-part PRs, section mapping, anti-patterns |
| [agents/models.md](agents/models.md) | Domain model reference (Thing-specific) |
| [agents/matching.md](agents/matching.md) | Matching algorithm reference (weights, components, rules) |
| [agents/restful.md](agents/restful.md) | REST API + library API reference |
| [agents/testing.md](agents/testing.md) | Testing strategy and guide |

## Shared docs (project root)

Shared reference docs live at the repo root under
[`../../agents/share/`](../../agents/share/).

| Document | Description |
|----------|-------------|
| [overview.md](../../agents/share/overview.md) | High-level project overview |
| [architecture.md](../../agents/share/architecture.md) | Layered architecture |
| [rust-loco-stack.md](../../agents/share/rust-loco-stack.md) | Full Rust + Loco dependency stack |
| [loco.md](../../agents/share/loco.md) | Loco framework conventions |
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

## Durable event bus — Phase 3 real-broker sink

`src/relay.rs`'s outbox relay ships to a `LoggingSink` (no-broker,
dev/CI) by default. `FluvioSink` (BUS-3, landed 2026-08-03, ported
from case-service's BUS-1 reference) is the real-broker `EventSink`,
compiled only under this crate's own `fluvio` Cargo feature (off by
default — `cargo build`/`test`/`clippy` behave identically without
it). Set `THING_FLUVIO_ENDPOINT` (broker SC address) and optionally
`THING_EVENT_TOPIC` (default `mxi.thing.events`) to select it; an
endpoint configured without the `fluvio` feature refuses to start the
relay (logged at `error`) rather than silently falling back to
`LoggingSink`. `compose.fluvio.yaml` + `Dockerfile.fluvio-cli`
provision a local broker for opt-in manual testing — see the run
command documented in `tests/fluvio_relay.rs`, not part of any
automated CI stage.

## Running this crate

```bash
# REST API (no gRPC — the Tonic dependency is unwired scaffolding
# for spec/13-tasks.md T-3, not a running server)
cargo run --release

# Tests
cargo test --lib                                # unit
cargo test --tests                              # integration (needs DATABASE_URL)
DATABASE_URL=… cargo test --test api_integration_test

# Benchmarks
cargo bench
```

## OpenTelemetry OTLP export

`src/observability.rs` (repo `tasks.md` PRO-H12, landed 2026-08-30) is a
close port of person-service's `src/observability.rs` — itself a port of
link-graph-service's, the family's first working exporter. This crate
carried no *working* observability module before this change:
`opentelemetry`/`opentelemetry-otlp`/`opentelemetry_sdk`/
`tracing-opentelemetry` were declared in `Cargo.toml` at stale 0.27/0.28
pins with **zero consumers anywhere in `src/`** — dead scaffolding from
an earlier, since-deleted stub — bumped to the family's settled 0.32/0.33
pins in the same change that added the real module.
`App::init_logger` installs it (loco's own `EnvFilter` + formatted
layer, plus the `tracing-opentelemetry` bridge over an OTLP/gRPC
exporter); `App::on_shutdown` flushes it. Export is **on by default** —
set `OTLP_ENDPOINT=""` to disable it — at `OTLP_ENDPOINT` (default
`http://localhost:4317`) with `service.name` from `OTLP_SERVICE_NAME`
(default `thing-service`); both variables are **deliberately
unprefixed**, matching every other crate that carries this pipeline,
not the per-service `THING_*` convention `THING_REQUIRE_AUTH` and its
siblings use.

**Where this crate's shape forced real adaptation** — confirmed rather
than assumed, and identical to place's slice (PRO-H12 slice 2) before
it:

- **Two router-construction surfaces**: the loco-native one
  (`api::rest::things_routes()`, mounted via `App::routes`/
  `App::after_routes`) and a standalone hand-rolled one
  (`api::rest::create_router`, used only by the DB-gated integration
  tests). `observability::trace_mw` is layered onto **both**, as the
  outermost layer in each.
- **A renamed `tonic` dev-dependency was needed**, exactly as place's
  slice needed and course's did not: this crate's own "Running this
  crate" section below already stated the gRPC dependency is "unwired
  scaffolding for spec/13-tasks.md T-3, not a running server" — but a
  *declared* `tonic = "0.12"` dependency collides with an unrenamed
  `tonic` dev-dependency at a different version regardless of whether
  any code actually calls `tonic::` (`E0464`). So the in-process OTLP
  collector tests' `tonic 0.14` dev-dependency is declared as
  `otlp-test-tonic = { package = "tonic", version = "0.14" }`, same as
  person/worker/event/place. This crate carries no SOUP register, so no
  matching `soup.rs` rename-resolution fix was needed.

`tests/otlp_export.rs` and `tests/otlp_middleware.rs` (ported from
place, with `tests/otlp_collector/`) prove real export against a real
gRPC listener in a normal `cargo test` run: a `tracing` span and a
metric both reach the collector's decoded protobuf, and a served HTTP
request returns a `traceparent` whose trace id matches the exported
span's. None of this needs a database. Landing this raised `cargo test
--lib` from 197 to 205 (8 new `src/observability.rs` unit tests), plus
4 new tests across the two `tests/otlp_*.rs` binaries. Verified
independently: `cargo fmt --check` clean, `cargo clippy --all-targets
-- -D warnings` clean, `cargo deny check` clean, MSRV check clean,
`cargo bench --no-run` compiles clean.

## Doc hierarchy quick reference

| File | Role |
|------|------|
| `spec.md` | **Single source of truth** — what, how, status, tasks (§13) |
| `README.md` (symlink to `index.md`) | User-facing intro — quick start, config, must stay consistent with the spec |
| `CLAUDE.md` | A one-line `@AGENTS.md` include, loaded by Claude Code at session start (root `AGENTS.md`'s per-subproject convention) — not a second user-facing intro |
| `AGENTS.md` / `agents/*.md` | How to work in the repo + per-topic reference |
| `index.md` | Navigation aid with worked examples |
| `CHANGELOG.md` | Historical record of releases and changes |

## Container image

`Dockerfile` (multi-stage, Debian 13 slim runtime) builds this crate's
production image. **Build context must be the repository root**, not
this directory — this crate's sibling path dependencies
(`integrity-mac`, `authentication-verifier`, and — since 2026-08-28 /
T-PRO-H7 — `thing-matcher`, now `../thing-matcher-rust-crate` rather
than a crates.io dependency) live outside
`thing/thing-service-with-loco/`:

```sh
podman build -f thing/thing-service-with-loco/Dockerfile \
  -t thing-service .   # run from the repository root
```

Verified end-to-end (2026-08-03): builds clean, boots against a real
Postgres, and `GET /api/health` returns `200`. This exercise found and
fixed two real bugs: Cargo.toml's `[[bench]] bridge_bench` declaration
requires `benches/bridge_bench.rs` to exist even for a `--bin`-only
build (cargo refuses to parse the manifest otherwise), and the
`migration/src/*.rs` wrappers `include_str!` the raw numbered SQL from
a `migrations/` directory that is a **sibling** of `migration/` at the
crate root (not nested inside it — this varies by crate; link-graph-
service nests `migrations/` inside `migration/` instead). Both
directories are now copied explicitly in the Dockerfile. See
`.containerignore` at the repository root (excludes every crate's
`target/`, or the build context would try to copy hundreds of GB of
build artifacts). The wired multi-service `examples/compose/` stacks
(DEP-1) that build on this are not yet written.
