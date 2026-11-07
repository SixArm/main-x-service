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
| [rust-axum-stack.md](../../agents/share/rust-axum-stack.md) | Full Rust + axa dependency stack |
| [loco.md](../../agents/share/axa.md) | axa framework conventions |
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

Export comes from `axa::opentelemetry`, axa's `opentelemetry` feature, since
2026-11-07. The crate's own `src/observability.rs`, a port of
person-service's, is gone. `App::init_logger` starts it and installs
axa's logger with its layers, `App::with_telemetry` layers its middleware
(the `http.server.request.duration` metric and a `traceparent` response
header) inside axa's standard stack, outermost of the service's own layers
so a `401` or `403` is measured, and `App::on_shutdown` sends what is
queued. Spans are axa's per-request `http-request` span and whatever the
code opens inside it.

| Variable | Meaning |
|---|---|
| `OTLP_ENDPOINT` | Collector URL. **Unset or blank means export is off**, where the old module exported to `http://localhost:4317` by default |
| `OTLP_SERVICE_NAME` | Replaces `thing-service` |
| `OTLP_ALLOW_INSECURE` | `true` allows plain `http` to a host other than this one. Without it such an endpoint stops the service at boot, because spans carry what the logs do |

The variables are deliberately unprefixed, unlike `THING_*`.

The in-process collector's `tonic` 0.14 dev-dependency is still declared
as `otlp-test-tonic = { package = "tonic", version = "0.14" }`: this
crate declares `tonic = "0.12"` in `[dependencies]` (unwired scaffolding
for spec/13-tasks.md T-3), and an unrenamed second `tonic` at another
version collides in a test binary's extern prelude (`E0464`).

`tests/otlp_export.rs`, with the in-process collector in
`tests/otlp_collector/`, proves the wiring in a normal `cargo test`: a
request through `App::with_telemetry` and `axa::middleware::apply` reaches
the collector as a span and a metric, and its `traceparent` names the
exported trace. axa tests its own pipeline. None of this needs a database.

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
`thing/thing-service-with-axum/`:

```sh
podman build -f thing/thing-service-with-axum/Dockerfile \
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
