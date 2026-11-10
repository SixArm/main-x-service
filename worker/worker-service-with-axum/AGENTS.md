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
| [rust-axum-stack.md](../../agents/share/rust-axum-stack.md) | Full Rust + axa dependency stack |
| [loco.md](../../agents/share/axa.md) | Tech stack summary |
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
cargo test --tests                              # integration (needs DB_URL)
DB_URL=… cargo test --test api_integration_test

# Benchmarks
cargo bench
```

## Database: sqlx, all SQL in `src/db/`

This crate is on sqlx, since 2026-11-08 (spec §10, §13 SQ-1;
the repo's `spec/tech-stack/index.md` §5; person-service was the
template). The rules that keep it reviewable:

- Every statement is a `sqlx::query!` / `query_as!` / `query_scalar!`
  literal in `src/db/`. Handlers, the gRPC server, the relay, compliance
  and tasks call functions there; they never hold SQL. Tests may run
  their own ground-truth SQL.
- No `QueryBuilder`, `raw_sql` or `AssertSqlSafe`. The clippy stage
  refuses them unless `ci/sql-allowlist.txt` names the file. Write an
  optional filter as `($1::text IS NULL OR col = $1)` instead; a choice
  of table is a `match` over literal statements (`db::erasure`).
- A helper running several statements takes `&mut PgConnection` or an
  `axa::sqlx::Db`, so the outbox, audit and child rows commit in the
  caller's transaction. The audit chain's lock, head read and insert run
  on one connection.
- Every worker write stamps its digests over the record **as stored**,
  read back in its own transaction (`stamp_digests`, INT-V1). A write
  path that skips it is a false tamper report waiting to happen;
  `tests/api_integration_test.rs` `a_written_record_verifies` guards it.
  An assessment is hashed over the row it writes, in one statement.
- Builds read `.sqlx/` with `SQLX_OFFLINE=true`. After changing a query
  or a migration, migrate a database and run `SQLX_OFFLINE=false cargo
  sqlx prepare -- --all-targets`, then commit `.sqlx/`. CI's `test-db`
  stage runs `cargo sqlx prepare --check`.
- Migrations are `migrations/<version>_<name>.up.sql` / `.down.sql`,
  embedded by `sqlx::migrate!` as `db::MIGRATOR`. Never edit an applied
  one: sqlx checks each file's checksum and refuses a changed one.
- Timestamps are `chrono` in the worker rows. Do not turn on sqlx's
  `time` feature: with both, sqlx 0.9.0's macros warn on every timestamp
  column. The audit trail, assessments, outbox, entity links and review
  queue keep their `time` types above `src/db/`, filled from `chrono`
  twins (`*Raw` in `db::models`), so the chain and assessment pre-images
  and the wire are unchanged.
- Enum columns hold the spellings their CHECK constraints take: the serde
  wire token (`enum_to_tag`), except a link type, which has its own map
  (`link_type_tag`: the column's `replaced_by` is the wire's
  `replacedby`). Never `format!("{:?}")`: that spelling is refused, and
  every write carrying it answered `500` until WK-D1 (spec §13).

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
| `OTLP_SERVICE_NAME` | Replaces `worker-service` |
| `OTLP_ALLOW_INSECURE` | `true` allows plain `http` to a host other than this one. Without it such an endpoint stops the service at boot, because spans carry what the logs do |

The variables are deliberately unprefixed, unlike `WORKER_*`. This crate's
`Config.observability` substructure reads two of the same names and is
not consulted by export (see `src/config/mod.rs`'s `ObservabilityConfig`
doc comment).

`tests/otlp_export.rs`, with the in-process collector in
`tests/otlp_collector/`, proves the wiring in a normal `cargo test`: a
request through `App::with_telemetry` and `axa::middleware::apply` reaches
the collector as a span and a metric, and its `traceparent` names the
exported trace. axa tests its own pipeline. None of this needs a database.

## gRPC server (T-6, repo `tasks.md` PRO-H11)

`src/api/grpc/` is a real `tonic::transport::Server`, not the
commented-out stub it used to be — following person-service's
reference implementation for this repo's gRPC rollout.
`proto/worker.proto` (crate root) defines `WorkerService` —
`CreateWorker` / `GetWorker` / `ListWorkers` / `DeleteWorker` —
compiled by `build.rs` (`tonic-prost-build`, pinned to the same 0.14
line as the main `tonic`/`tonic-prost` dependencies — tonic 0.14 split
prost codegen out of `tonic-build` into this crate, 2026-09-07). `App::after_routes` spawns
`crate::api::grpc::serve` as a
background task on `GRPC_PORT` (config `server.grpc_port`, default
`50051`) alongside the REST router, sharing one cloned `AppState` — a
bind/serve failure is logged, not fatal, so REST still boots if the
gRPC port is unavailable.

**No duplicated business logic.** `src/api/grpc/service.rs`'s
`WorkerGrpcService` calls the exact same functions the REST handlers
do: `crate::validation::validate_worker`, the shared duplicate-detection
core (`handlers::check_duplicates_internal`, bumped to `pub(crate)`
rather than copied), the same `WorkerRepository` trait methods (whose
mutations take the `AuditContext` from `auth::audit_context_of`, as
person-service's do), and `auth::authorize_record` +
`crate::privacy::mask_worker` for `GetWorker`'s record-level ABAC +
masking — the wire format differs, the rules do not. `worker_type`
parses via the domain enum's existing `serde` implementation
(`serde_json::from_value`) rather than a hand-rolled second mapping.

**Auth parity with REST, by design.** `grpc_enforce` (in
`service.rs`) is the gRPC counterpart of `auth::enforce`: it reads the
bearer token from the `authorization` gRPC metadata entry and is gated
by the same `WORKER_REQUIRE_AUTH` flag. `GetWorker`/`DeleteWorker`
additionally run `auth::authorize_record` against the loaded record,
same as REST's single-record handlers.

**Deliberately not carried over yet** (tracked in spec §13 T-6, not
silently missing): `UpdateWorker` (no RPC); the HIPAA §164.528
disclosure-accounting audit row REST writes on every read; `ListWorkers`'
per-record read-visibility filtering (only the blanket `Read` check
applies today); match/merge/search/assessments/FHIR over gRPC; and
most of the domain model's fields on the proto `Worker` message
(identifiers, addresses, telecom, documents, emergency contacts,
links) — the message is a deliberate partial projection (id, name,
gender, `worker_type`, birth date, tax id, timestamps), not a 1:1
mirror of REST/FHIR.

`tests/grpc_integration_test.rs` proves it end to end against a real
Postgres: binds the server on an OS-assigned port (the same
`TcpListener::bind("127.0.0.1:0")` + `serve_with_incoming` pattern
`tests/otlp_collector` uses), connects a real
`worker_service_client::WorkerServiceClient`, and drives a
Create→Get→List→Delete→Get(`NOT_FOUND`) round trip plus three
error-path proofs (a blank family name → `INVALID_ARGUMENT`; an
unrecognised `worker_type` → `INVALID_ARGUMENT`; a malformed id →
`INVALID_ARGUMENT`, not `INTERNAL`). `#[ignore]`d like the REST
integration suite; run with `cargo test --test grpc_integration_test
-- --ignored` against `scripts/test-db.sh up
worker/worker-service-with-axum`. A manual `grpcurl
WorkerService.GetWorker` smoke check (this crate's spec §13 T-6's
original acceptance criterion) was not additionally run in this
sandbox (`grpcurl` unavailable) — the automated test proves the
identical claim, repeatably.

## Container image

`Dockerfile` (multi-stage, Debian 13 slim runtime) builds this crate's
production image. **Build context must be the repository root**, not
this directory — this crate's sibling path dependencies
(`integrity-mac`, `worker-matcher`, `authentication-verifier`,
`entity-ref`) live outside `worker/worker-service-with-axum/`:

```sh
podman build -f worker/worker-service-with-axum/Dockerfile \
  -t worker-service .   # run from the repository root
```

Verified end-to-end (2026-08-03): builds clean, boots against a real
Postgres, and `GET /api/health` returns `200`. Like person's, this
crate's Dockerfile pre-dated the family's repo-root-context convention
and had the same four bugs, found only by running the built image:
no `config/` copy (boot crash: "no configuration file found in folder:
config"); `CMD` with no `start` subcommand (the axa CLI just prints
`--help` and exits 0); no `APP_ENV` (would boot in `development`
inside a `production` image); and a dead `SERVER_PORT` env var — axa's
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
