# AGENTS — Place Service

How to work in this crate. The canonical artefact is
[`spec.md`](spec/index.md). When in doubt, the spec wins. See
[`agents/spec-driven-development.md`](agents/spec-driven-development.md)
for the discipline this crate practises.

## Crate-local docs (`agents/`)

| Document | Description |
|----------|-------------|
| [agents/index.md](agents/index.md) | Directory index |
| [agents/spec-driven-development.md](agents/spec-driven-development.md) | SDD discipline — three-part PRs, section mapping, anti-patterns |
| [agents/models.md](agents/models.md) | Domain model reference (Place-specific) |
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
(default `place-service`); both variables are **deliberately
unprefixed**, matching every other crate that carries this pipeline,
not the per-service `PLACE_*` convention `PLACE_REQUIRE_AUTH` and its
siblings use.

**Where this crate's shape forced real adaptation** — confirmed rather
than assumed:

- **Two router-construction surfaces**, exactly as person/worker/event/
  course needed: the loco-native one (`api::rest::places_routes()`,
  mounted via `App::routes`/`App::after_routes`) and a standalone
  hand-rolled one (`api::rest::create_router`, used only by the
  DB-gated integration tests). `observability::trace_mw` is layered
  onto **both**, as the outermost layer in each.
- **A renamed `tonic` dev-dependency was needed after all** — the one
  place where this crate's port diverged from course's (PRO-H12 slice
  1), which needed no rename. The capability matrix in
  `agents/share/overview.md` marks this crate `–` on the gRPC-stub row
  because there is no `src/api/grpc` **module** — but this crate
  already declares `tonic = "0.12"` + `tonic-build` in `Cargo.toml`, in
  anticipation of the still-open T-4 (gRPC implementation, not yet
  started). A declared-but-unused Cargo dependency collides with an
  unrenamed dev-dependency at a different version exactly the same way
  a genuinely-used one does (`E0464: multiple candidates for rlib
  dependency tonic`) — confirmed by trying the plain form first and
  watching it fail. So the in-process OTLP collector tests' `tonic
  0.14` dev-dependency is declared as
  `otlp-test-tonic = { package = "tonic", version = "0.14" }`, same as
  person/worker/event. This crate carries no SOUP register (unlike
  person/worker), so no matching `soup.rs` rename-resolution fix was
  needed.

`tests/otlp_export.rs` and `tests/otlp_middleware.rs` (ported from
person-service, with `tests/otlp_collector/` — an in-process OTLP/gRPC
collector) prove real export against a real gRPC listener in a normal
`cargo test` run: a `tracing` span and a metric both reach the
collector's decoded protobuf, and a served HTTP request returns a
`traceparent` whose trace id matches the exported span's. None of this
needs a database. Landing this raised `cargo test --lib` from 221 to
229 (8 new `src/observability.rs` unit tests), plus 4 new tests across
the two `tests/otlp_*.rs` binaries. Verified independently: `cargo fmt
--check` clean, `cargo clippy --all-targets -- -D warnings` clean,
`cargo deny check` clean, MSRV check clean, `cargo bench --no-run`
compiles clean.

## Doc hierarchy quick reference

| File | Role |
|------|------|
| `spec.md` | **Single source of truth** — what, how, status, tasks (§13) |
| `README.md` (symlink to `index.md`) | User-facing intro — must stay consistent with the spec |
| `AGENTS.md` / `agents/*.md` / `CLAUDE.md` | How to work in the repo + per-topic reference. `CLAUDE.md` is a one-line `@AGENTS.md` include (matching the family convention — see root `AGENTS.md`), not a second user-facing doc; its content used to duplicate `index.md` and has been folded/removed rather than kept in sync by hand. |
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
