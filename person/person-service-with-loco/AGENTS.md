# AGENTS — Person Service

How to work in this crate. The canonical artefact is
[`spec.md`](spec/index.md). When in doubt, the spec wins. See
[`agents/spec-driven-development.md`](agents/spec-driven-development.md)
for the discipline this crate practises.

## Crate-local docs (`agents/`)

| Document | Description |
|----------|-------------|
| [agents/index.md](agents/index.md) | Directory index |
| [agents/spec-driven-development.md](agents/spec-driven-development.md) | SDD discipline — three-part PRs, section mapping, anti-patterns |
| [agents/models.md](agents/models.md) | Domain model reference (`Person`, `HumanName`, supporting types) |
| [agents/matching.md](agents/matching.md) | Matching algorithm reference (weights, rules, components) |
| [agents/restful.md](agents/restful.md) | REST API + FHIR R5 + library API reference |
| [agents/testing.md](agents/testing.md) | Testing strategy and guide |

## Shared docs (project root)

Shared reference docs live at the project root under
[`../../agents/share/`](../../agents/share/).

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

## Session context (auto-loaded)

`CLAUDE.md` is this crate's one-line `@AGENTS.md` include (the family
convention — see the root `AGENTS.md` "Per-subproject docs" table), so
these `@`-imports are what a Claude Code session in this crate actually
loads at start, beyond the tables above (which are plain links, for a
human or agent to open on demand rather than always pull into context):

@../../agents/share/overview.md
@agents/matching.md
@agents/models.md
@agents/restful.md
@agents/testing.md
@../../agents/share/architecture.md
@../../agents/share/auditability.md
@../../agents/share/availability.md
@../../agents/share/match-search-merge.md
@../../agents/share/observability.md
@../../agents/share/privacy.md
@../../agents/share/restful.md
@../../agents/share/loco.md

## Running this crate

```bash
# REST API (FHIR R5 mounted within it; gRPC is a stub) — boots via the
# loco CLI, which needs a subcommand. Bare `cargo run` will not start it.
cargo run -- start                              # or: cargo loco start
cargo loco db migrate                           # apply migrations (auto in dev)

# Tests
cargo test --lib                                # unit
cargo test --tests                              # integration (needs DATABASE_URL)
DATABASE_URL=… cargo test --test api_integration_test

# Benchmarks
cargo bench
```

## Durable event bus

The transactional-outbox event bus (`src/db/outbox.rs`, `src/relay.rs`)
is default-off via `PERSON_EVENT_TRANSPORT=memory`. With `outbox` set
and `PERSON_EVENT_RELAY` truthy, the Phase-3 relay drains unpublished
`event_outbox` rows to an `EventSink`. **Phase 3's real-broker sink**
(BUS-3, landed 2026-08-03, ported from case-service's BUS-1 reference)
is `FluvioSink` in `src/relay.rs`, behind this crate's own `fluvio`
Cargo feature (off by default): `PERSON_FLUVIO_ENDPOINT` selects it
over the default `LoggingSink`; unset without the feature ⇒ unchanged
behaviour; **set** without the feature ⇒ the relay refuses to start
(logged, not a silent no-broker fallback that would mark rows
published without reaching a real broker). `compose.fluvio.yaml` +
`Dockerfile.fluvio-cli` provision a local broker for opt-in manual runs
(not part of any automated CI stage).

## OpenTelemetry OTLP export

`src/observability.rs` (repo `tasks.md` PRO-H9, landed 2026-08-28) is a
close port of link-graph-service's `src/observability.rs` — the
family's first working exporter and this crate's reference. It
**replaces** the earlier `src/observability/` stub outright (a JSON
`tracing` subscriber with the OTLP exporter commented out behind
`// TODO: Initialize OTLP exporter`, never wired into `App`'s `Hooks`
impl at all) rather than filling the stub in place. `App::init_logger`
installs it (loco's own `EnvFilter` + formatted layer, plus the
`tracing-opentelemetry` bridge over an OTLP/gRPC exporter);
`App::on_shutdown` flushes it. Export is **on by default** — set
`OTLP_ENDPOINT=""` to disable it — at `OTLP_ENDPOINT` (default
`http://localhost:4317`) with `service.name` from `OTLP_SERVICE_NAME`
(default `person-service`); both variables are **deliberately
unprefixed**, matching link-graph-service and
`agents/share/rust-tracing-opentelemetry-stack.md`'s config table, not
the per-service `PERSON_*` convention `PERSON_REQUIRE_AUTH` and its
siblings use.

**Where this crate's shape forced real adaptation**, beyond the
`Hooks::init_logger`/`on_shutdown` seam (which ported close to
verbatim): this crate carries **two** router-construction surfaces —
the loco-native one (`api::rest::persons_routes()`, mounted via
`App::routes`/`App::after_routes`) and a standalone hand-rolled one
(`api::rest::create_router`, used only by the DB-gated integration
tests) — where link-graph-service has exactly one (pure loco). The
`observability::trace_mw` tower middleware (per-request span +
`http.server.request.duration` histogram + W3C `traceparent` response
header) is layered onto **both**, as the outermost layer in each, so
tracing behaves identically regardless of which router a caller or
test builds — the same precedent `auth::require_auth_middleware`
already set by being layered on both surfaces.

A second, narrower adaptation: this crate already depends on
`tonic = "0.12"` for its own gRPC stub (`src/api/grpc/`), so the
in-process OTLP collector tests' `tonic = "0.14"` dev-dependency (used
to serve the fake collector) is declared as
`otlp-test-tonic = { package = "tonic", version = "0.14" }` — an
unrenamed second `tonic` dependency at a different version collides in
a test binary's extern prelude (`E0464: multiple candidates for rlib
dependency tonic`). link-graph-service has no gRPC stub of its own, so
this collision — and the rename — has no analogue there; worker and
event both carry the same `tonic = "0.12"` gRPC-stub dependency
(`agents/share/overview.md`'s capability matrix) and will need the same
rename when this pattern rolls to them. The rename also required
teaching `src/compliance/soup.rs`'s SOUP-register parser to resolve a
`package = "…"` inline-table rename to its target crate name — the
unrenamed manifest alias satisfies neither the "every direct dependency
is annotated" check (which wants the alias annotated) nor the "no stale
register entries" check (which wants the annotated name to exist in
`Cargo.lock`, where only the resolved name appears) at once.

`tests/otlp_export.rs` and `tests/otlp_middleware.rs` (ported from
link-graph-service, with `tests/otlp_collector/` — an in-process
OTLP/gRPC collector, unchanged) prove real export against a real gRPC
listener in a normal `cargo test` run: a `tracing` span and a metric
both reach the collector's decoded protobuf, and a served HTTP request
returns a `traceparent` whose trace id matches the exported span's.
None of this needs a database.

## Container image

`Dockerfile` (multi-stage, Debian 13 slim runtime) builds this crate's
production image. **Build context must be the repository root**, not
this directory — this crate's sibling path dependencies
(`integrity-mac`, `person-matcher`, `entity-ref`,
`authentication-verifier`) live outside `person/person-service-with-loco/`:

```sh
podman build -f person/person-service-with-loco/Dockerfile \
  -t person-service .   # run from the repository root
```

Verified end-to-end (2026-08-03): builds clean, boots against a real
Postgres, and `GET /api/health` returns `200`. This crate's Dockerfile
pre-dated the family's repo-root-context convention (the other nine
service crates were fixed to it first, on 2026-08-03) and had **four**
real bugs beyond the build-context mismatch, found only by actually
running the built image rather than trusting that a `podman build`
success meant the container worked:

1. It never copied `config/` — the loco binary crashed at boot with
   `Message("no configuration file found in folder: config")`.
2. Its `CMD` was `["/app/person-service"]` with no subcommand — this
   crate's binary dispatches through `loco_rs::cli::main`
   (`src/bin/main.rs`), which needs an explicit `start` argument; a
   bare invocation just prints the CLI's own `--help` and exits 0
   (a "successful" container that serves nothing).
3. No `LOCO_ENV` was set, so the binary would have booted in loco's
   default `development` environment inside a `production`-tagged
   image.
4. `ENV SERVER_PORT=8080` is dead: loco's own `config/production.yaml`
   reads `PORT` (`server.port: {{ get_env(name="PORT", default="8080") }}`),
   not `SERVER_PORT` — the `SERVER_PORT` this crate's own
   `src/config/mod.rs` documents is a separate, unrelated config
   surface used by non-loco code paths. `PORT=8080` is now set
   explicitly so the family's port-8080 convention holds regardless.

See `.containerignore` at the repository root (excludes every crate's
`target/`, or the build context would try to copy hundreds of GB of
build artifacts). The wired multi-service `examples/compose/` stacks
(DEP-1) that build on this are not yet written.

## Doc hierarchy quick reference

| File | Role |
|------|------|
| `spec.md` | **Single source of truth** — what, how, status, tasks (§13) |
| `README.md` / `CLAUDE.md` | User-facing intro — must stay consistent with the spec |
| `AGENTS.md` / `agents/*.md` | How to work in the repo + per-topic reference |
| `index.md` | Navigation aid with worked examples |
| `CHANGELOG.md` | Historical record of releases and changes |
