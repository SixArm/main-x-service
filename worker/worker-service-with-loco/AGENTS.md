# AGENTS — Worker Service

How to work in this crate. The canonical artefact is
[`spec.md`](spec/index.md). When in doubt, the spec wins. See
[`agents/spec-driven-development.md`](agents/spec-driven-development.md)
for the discipline this crate practises.

## Crate-local docs (`agents/`)

| Document | Description |
|----------|-------------|
| [agents/index.md](agents/index.md) | Directory index |
| [agents/spec-driven-development.md](agents/spec-driven-development.md) | SDD discipline — three-part PRs, section mapping, anti-patterns |
| [agents/models.md](agents/models.md) | Domain model reference (Worker-specific) |
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

## Durable event bus relay (Fluvio)

The Phase-3 outbox relay (`src/relay.rs`, default-off via
`WORKER_EVENT_TRANSPORT=outbox` + `WORKER_EVENT_RELAY`) ships a real
**`FluvioSink`** (BUS-3, ported from the case-service BUS-1 reference)
behind this crate's own `fluvio` Cargo feature — off by default, so a
plain `cargo build`/`cargo test` is unaffected. `WORKER_FLUVIO_ENDPOINT`
selects it over the default `LoggingSink`; an endpoint configured
**without** the `fluvio` feature compiled in makes the relay refuse to
start (logged, not a silent no-broker fallback). See spec §13 (Phase 3 /
BUS-3) for the full contract, `compose.fluvio.yaml` +
`Dockerfile.fluvio-cli` for a local broker, and `tests/fluvio_relay.rs`
for the feature-gated, `#[ignore]`d live round-trip.

```bash
cargo build --lib --features fluvio     # proves the real fluvio 0.50 API compiles
cargo clippy --all-targets --features fluvio -- -D warnings
```

## Container image

`Dockerfile` (multi-stage, Debian 13 slim runtime) builds this crate's
production image. **Build context must be the repository root**, not
this directory — this crate's sibling path dependencies
(`integrity-mac`, `worker-matcher`, `authentication-verifier`,
`entity-ref`) live outside `worker/worker-service-with-loco/`:

```sh
podman build -f worker/worker-service-with-loco/Dockerfile \
  -t worker-service .   # run from the repository root
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
| `README.md` (symlink to `index.md`) | User-facing intro — quick start, config, must stay consistent with the spec |
| `CLAUDE.md` | A one-line `@AGENTS.md` include, loaded by Claude Code at session start (root `AGENTS.md`'s per-subproject convention) — not a second user-facing intro |
| `AGENTS.md` / `agents/*.md` | How to work in the repo + per-topic reference |
| `index.md` | Navigation aid with worked examples |
| `CHANGELOG.md` | Historical record of releases and changes |
