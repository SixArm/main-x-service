# AGENTS — Thing Service

How to work in this crate. The canonical artefact is
[`spec.md`](spec/index.md). When in doubt, the spec wins. See
[`AGENTS/spec-driven-development.md`](AGENTS/spec-driven-development.md)
for the discipline this crate practises.

## Crate-local docs (`AGENTS/`)

| Document | Description |
|----------|-------------|
| [AGENTS/index.md](AGENTS/index.md) | Directory index |
| [AGENTS/spec-driven-development.md](AGENTS/spec-driven-development.md) | SDD discipline — three-part PRs, section mapping, anti-patterns |
| [AGENTS/models.md](AGENTS/models.md) | Domain model reference (Thing-specific) |
| [AGENTS/matching.md](AGENTS/matching.md) | Matching algorithm reference (weights, components, rules) |
| [AGENTS/restful.md](AGENTS/restful.md) | REST API + library API reference |
| [AGENTS/testing.md](AGENTS/testing.md) | Testing strategy and guide |

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
# REST + gRPC API
cargo run --release

# Tests
cargo test --lib                                # unit
cargo test --tests                              # integration (needs DATABASE_URL)
DATABASE_URL=… cargo test --test api_integration_test

# Benchmarks
cargo bench
```

## Doc hierarchy quick reference

| File | Role |
|------|------|
| `spec.md` | **Single source of truth** — what, how, status, tasks (§13) |
| `README.md` / `CLAUDE.md` | User-facing intro — must stay consistent with the spec |
| `AGENTS.md` / `AGENTS/*.md` | How to work in the repo + per-topic reference |
| `index.md` | Navigation aid with worked examples |
| `CHANGELOG.md` | Historical record of releases and changes |
