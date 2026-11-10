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
@../../agents/share/axa.md

## Running this crate

```bash
# REST API (FHIR R5 mounted within it) + a real gRPC server on
# GRPC_PORT (default 50051, T-6) — boots via the axa CLI, which needs
# a subcommand. Bare `cargo run` will not start it.
cargo run -- start                              # or: cargo run -- start
cargo run -- db migrate                           # apply migrations (auto in dev)

# Tests
cargo test --lib                                # unit
cargo test --tests                              # integration (needs DB_URL)
DB_URL=… cargo test --test api_integration_test

# Benchmarks
cargo bench
```

## Database: sqlx, all SQL in `src/db/`

Since 2026-11-08 this crate is on sqlx, not SeaORM (spec §10, §13 SQ-1;
the repo's `spec/tech-stack/index.md` §5; event-service was the
person-style template, case and care-pathway the templates for the hash
chain and erasure). The rules that keep it reviewable:

- Every statement is a `sqlx::query!` / `query_as!` / `query_scalar!`
  literal in `src/db/`. Handlers, the gRPC server, the relay, bulk,
  compliance and tasks call functions there; they never hold SQL. Tests
  may run their own ground-truth SQL.
- No `QueryBuilder`, `raw_sql` or `AssertSqlSafe`. The clippy stage
  refuses them unless `ci/sql-allowlist.txt` names the file. Write an
  optional filter as `($1::text IS NULL OR col = $1)` instead; a choice
  of table is a `match` over literal statements (`db::erasure`).
- A helper running several statements takes `&mut PgConnection` or an
  `axa::sqlx::Db`, so the outbox, audit and child rows commit in the
  caller's transaction. The audit chain's lock, head read and insert run
  on one connection.
- Every person write stamps its digests over the record **as stored**,
  read back in its own transaction (`stamp_digests`, INT-V1). A write
  path that skips it is a false tamper report waiting to happen;
  `tests/api_integration_test.rs` `a_written_record_verifies` guards it.
- Builds read `.sqlx/` with `SQLX_OFFLINE=true`. After changing a query
  or a migration, migrate a database and run `SQLX_OFFLINE=false cargo
  sqlx prepare -- --all-targets`, then commit `.sqlx/`. CI's `test-db`
  stage runs `cargo sqlx prepare --check`. The bulk reconcile query
  reads `axa_queue`, which a boot or `db migrate` creates.
- Migrations are `migrations/<version>_<name>.up.sql` / `.down.sql`,
  embedded by `sqlx::migrate!` as `db::MIGRATOR`. Never edit an applied
  one: sqlx checks each file's checksum and refuses a changed one. Some
  names are legacy (`create_patients`); they stay as they are.
- Timestamps are `chrono` in the person rows. Do not turn on sqlx's
  `time` feature: with both, sqlx 0.9.0's macros warn on every timestamp
  column. The audit trail, bulk jobs, outbox and entity links keep their
  `time` types above `src/db/`, filled from `chrono` twins (`*Raw` in
  `db::models`), so the chain pre-image and the wire are unchanged.

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

Export comes from `axa::opentelemetry`, axa's `opentelemetry` feature, since
2026-11-07. The crate's own `src/observability.rs`, a port of
link-graph-service's, is gone. `App::init_logger` starts it and installs
axa's logger with its layers, `App::with_telemetry` layers its middleware
(the `http.server.request.duration` metric and a `traceparent` response
header) inside axa's standard stack, outermost of the service's own layers
so a `401` or `403` is measured, and `App::on_shutdown` sends what is
queued. Spans are axa's per-request `http-request` span and whatever the
code opens inside it.

| Variable | Meaning |
|---|---|
| `OTLP_ENDPOINT` | Collector URL. **Unset or blank means export is off**, where the old module exported to `http://localhost:4317` by default |
| `OTLP_SERVICE_NAME` | Replaces `person-service` |
| `OTLP_ALLOW_INSECURE` | `true` allows plain `http` to a host other than this one. Without it such an endpoint stops the service at boot, because spans carry what the logs do |

The variables are deliberately unprefixed, unlike `PERSON_*`.

`tests/otlp_export.rs`, with the in-process collector in
`tests/otlp_collector/`, proves the wiring in a normal `cargo test`: a
request through `App::with_telemetry` and `axa::middleware::apply` reaches
the collector as a span and a metric, and its `traceparent` names the
exported trace. axa tests its own pipeline. None of this needs a database.

## gRPC server (T-6, repo `tasks.md` PRO-H11 reference)

`src/api/grpc/` is a real `tonic::transport::Server`, not the
commented-out stub it used to be. `proto/person.proto` (crate root)
defines `PersonService` — `CreatePerson` / `GetPerson` / `ListPersons`
/ `DeletePerson` — compiled by `build.rs` (`tonic-prost-build`, pinned
to the same 0.14 line as the main `tonic`/`tonic-prost` dependencies —
see the OTLP export section above for why this is `tonic-prost-build`
and not `tonic-build` as of 2026-09-07) into
`crate::api::grpc::proto`. `App::after_routes` spawns
`crate::api::grpc::serve` as a background task on `GRPC_PORT` (config
`server.grpc_port`, default `50051`) alongside the REST router, sharing
one cloned `AppState` — a bind/serve failure is logged, not fatal, so
REST still boots if the gRPC port is unavailable.

**No duplicated business logic.** `src/api/grpc/service.rs`'s
`PersonGrpcService` calls the exact same functions the REST handlers
do: `crate::validation::validate_person`, the shared duplicate-detection
core (`handlers::check_duplicates_internal`, bumped to `pub(crate)`
rather than copied), the same `PersonRepository` trait methods, and
`auth::authorize_record` + `crate::privacy::mask_person` for
`GetPerson`'s record-level ABAC + masking — the wire format differs,
the rules do not.

**Auth parity with REST, by design.** `grpc_enforce` (in
`service.rs`) is the gRPC counterpart of `auth::enforce`: it reads the
bearer token from the `authorization` gRPC metadata entry and is gated
by the same `PERSON_REQUIRE_AUTH` flag, so turning enforcement on
protects both surfaces together — there is no way to leave gRPC open
while REST is locked down. `GetPerson`/`DeletePerson` additionally run
`auth::authorize_record` against the loaded record, same as REST's
single-record handlers.

**Deliberately not carried over yet** (tracked in spec §13 T-6, not
silently missing): `UpdatePerson` (no RPC); the HIPAA §164.528
disclosure-accounting audit row REST writes on every read (needs a
gRPC-side `AccessContext` equivalent); `ListPersons`' SEC-G3 per-record
read-visibility filtering/masking (only the blanket `Read` check
applies today); match/merge/search/bulk/FHIR over gRPC; and most of the
domain model's fields on the proto `Person` message (identifiers,
addresses, telecom, documents, emergency contacts, links) — the
message is a deliberate partial projection (id, name, gender, birth
date, tax id, timestamps), not a 1:1 mirror of REST/FHIR.

`tests/grpc_integration_test.rs` proves it end to end against a real
Postgres: binds the server on an OS-assigned port (the same
`TcpListener::bind("127.0.0.1:0")` + `serve_with_incoming` pattern
`tests/otlp_collector` uses), connects a real
`person_service_client::PersonServiceClient`, and drives a
Create→Get→List→Delete→Get(`NOT_FOUND`) round trip plus two error-path
proofs (a blank family name → `INVALID_ARGUMENT`, proving the shared
validator actually runs; a malformed id → `INVALID_ARGUMENT`, not
`INTERNAL`). `#[ignore]`d like the REST integration suite; run with
`cargo test --test grpc_integration_test -- --ignored` against
`scripts/test-db.sh up person/person-service-with-axum`. A manual
`grpcurl PersonService.GetPerson` smoke check (this crate's spec §13
T-6's original acceptance criterion) was not additionally run in this
sandbox (`grpcurl` unavailable, `brew install` blocked by directory
ownership) — the automated test proves the identical claim, repeatably.

## Container image

`Dockerfile` (multi-stage, Debian 13 slim runtime) builds this crate's
production image. **Build context must be the repository root**, not
this directory — this crate's sibling path dependencies
(`integrity-mac`, `person-matcher`, `entity-ref`,
`authentication-verifier`) live outside `person/person-service-with-axum/`:

```sh
podman build -f person/person-service-with-axum/Dockerfile \
  -t person-service .   # run from the repository root
```

Verified end-to-end (2026-08-03): builds clean, boots against a real
Postgres, and `GET /api/health` returns `200`. This crate's Dockerfile
pre-dated the family's repo-root-context convention (the other nine
service crates were fixed to it first, on 2026-08-03) and had **four**
real bugs beyond the build-context mismatch, found only by actually
running the built image rather than trusting that a `podman build`
success meant the container worked:

1. It never copied `config/` — the axa binary crashed at boot with
   `Message("no configuration file found in folder: config")`.
2. Its `CMD` was `["/app/person-service"]` with no subcommand — this
   crate's binary dispatches through `axa::cli::main`
   (`src/bin/main.rs`), which needs an explicit `start` argument; a
   bare invocation just prints the CLI's own `--help` and exits 0
   (a "successful" container that serves nothing).
3. No `APP_ENV` was set, so the binary would have booted in axa's
   default `development` environment inside a `production`-tagged
   image.
4. `ENV SERVER_PORT=8080` is dead: axa's own `config/production.yaml`
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
