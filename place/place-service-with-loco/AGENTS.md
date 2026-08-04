# AGENTS — Place Service

How to work in this crate. The canonical artefact is
[`spec.md`](spec/index.md). When in doubt, the spec wins. See
[`AGENTS/spec-driven-development.md`](AGENTS/spec-driven-development.md)
for the discipline this crate practises.

## Crate-local docs (`AGENTS/`)

| Document | Description |
|----------|-------------|
| [AGENTS/index.md](AGENTS/index.md) | Directory index |
| [AGENTS/spec-driven-development.md](AGENTS/spec-driven-development.md) | SDD discipline — three-part PRs, section mapping, anti-patterns |
| [AGENTS/models.md](AGENTS/models.md) | Domain model reference (Place-specific) |
| [AGENTS/matching.md](AGENTS/matching.md) | Matching algorithm reference (weights, components, rules) |
| [AGENTS/restful.md](AGENTS/restful.md) | REST API + library API reference |
| [AGENTS/testing.md](AGENTS/testing.md) | Testing strategy and guide |

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

### Durable event bus — the relay's real-broker sink (BUS-3)

`src/relay.rs` drains the transactional `event_outbox` (Phase 2/3,
default-off via `PLACE_EVENT_TRANSPORT=outbox` + `PLACE_EVENT_RELAY`) to
an `EventSink`. The default is the no-broker `LoggingSink`; a real
`FluvioSink` (ported 2026-08-03 from the case-service reference, BUS-1)
lives behind this crate's own `fluvio` Cargo feature, off by default, so
a plain `cargo build`/`cargo test --lib` is unchanged. `PLACE_FLUVIO_ENDPOINT`
selects it; an endpoint configured **without** the feature refuses to
start the relay (logged) rather than silently falling back. Exercise it
locally with `compose.fluvio.yaml` + `Dockerfile.fluvio-cli` (opt-in,
not part of any CI stage) and `cargo test --features fluvio --test
fluvio_relay -- --ignored` — see that file's module docs for the exact
commands.

## Doc hierarchy quick reference

| File | Role |
|------|------|
| `spec.md` | **Single source of truth** — what, how, status, tasks (§13) |
| `README.md` (symlink to `index.md`) | User-facing intro — must stay consistent with the spec |
| `AGENTS.md` / `AGENTS/*.md` / `CLAUDE.md` | How to work in the repo + per-topic reference. `CLAUDE.md` is a one-line `@AGENTS.md` include (matching the family convention — see root `AGENTS.md`), not a second user-facing doc; its content used to duplicate `index.md` and has been folded/removed rather than kept in sync by hand. |
| `index.md` | Navigation aid with worked examples |
| `CHANGELOG.md` | Historical record of releases and changes |

## Container image

`Dockerfile` (multi-stage, Debian 13 slim runtime) builds this crate's
production image. **Build context must be the repository root**, not
this directory — this crate's sibling path dependencies
(`integrity-mac`, `authentication-verifier`) live outside
`place/place-service-with-loco/` (its matcher, `place-matcher`, is
pulled from crates.io, not a path dependency):

```sh
podman build -f place/place-service-with-loco/Dockerfile \
  -t place-service .   # run from the repository root
```

For local development, `podman compose up -d` (from this crate's own
directory) is usually simpler than building the image directly.

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
