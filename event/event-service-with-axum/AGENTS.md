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

Since 2026-11-07 this crate is on sqlx, not SeaORM (spec §10, §13 T-17;
the repo's `spec/tech-stack/index.md` §5; place-service was the pilot,
thing-service the closest template). The rules that keep it reviewable:

- Every statement is a `sqlx::query!` / `query_as!` / `query_scalar!`
  literal in `src/db/`. Handlers, the gRPC server, the relay and tests
  call functions there; they never hold SQL.
- No `QueryBuilder`, `raw_sql` or `AssertSqlSafe`. The clippy stage
  refuses them unless `ci/sql-allowlist.txt` names the file. Write an
  optional filter as `($1::text IS NULL OR col = $1)` instead.
- A helper running several statements takes `&mut PgConnection`, so the
  outbox row and the child rows commit in the caller's transaction.
- Builds read `.sqlx/` with `SQLX_OFFLINE=true`. After changing a query
  or a migration, migrate a database and run `SQLX_OFFLINE=false cargo
  sqlx prepare -- --all-targets`, then commit `.sqlx/`. CI's `test-db`
  stage runs `cargo sqlx prepare --check`.
- Migrations are `migrations/<version>_<name>.up.sql` / `.down.sql`,
  embedded by `sqlx::migrate!` as `db::MIGRATOR`. Never edit an applied
  one: sqlx checks each file's checksum and refuses a changed one. Some
  names are legacy (`create_organizations`); they stay as they are.
- Timestamps are `chrono` in the row structs. Do not turn on sqlx's
  `time` feature: with both, sqlx 0.9.0's macros warn on every timestamp
  column. `time` remains for the outbox's `OutboxInsert` and for the
  audit row's wire form of `timestamp` (`db::models::AuditLogRow`).

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
| `OTLP_SERVICE_NAME` | Replaces `event-service` |
| `OTLP_ALLOW_INSECURE` | `true` allows plain `http` to a host other than this one. Without it such an endpoint stops the service at boot, because spans carry what the logs do |

The variables are deliberately unprefixed, unlike `EVENT_*`.

The in-process collector's dev-dependencies use plain `tonic = "0.14"`,
the same line as the gRPC server's, so no `otlp-test-tonic` rename is
needed. This crate has no SOUP register (`compliance/soup.tsv`), so the
move needed no register rows.

`tests/otlp_export.rs`, with the in-process collector in
`tests/otlp_collector/`, proves the wiring in a normal `cargo test`: a
request through `App::with_telemetry` and `axa::middleware::apply` reaches
the collector as a span and a metric, and its `traceparent` names the
exported trace. axa tests its own pipeline. None of this needs a database.

## gRPC server (T-6, repo `tasks.md` PRO-H11)

`src/api/grpc/` is a real `tonic::transport::Server`, not the
commented-out stub it used to be — following person-service's and
worker-service's reference implementations for this repo's gRPC
rollout. `proto/event.proto` (crate root) defines `EventService` —
`CreateEvent` / `GetEvent` / `ListEvents` / `DeleteEvent` — compiled
by `build.rs` (`tonic-prost-build`, pinned to the same 0.14 line as
the main `tonic`/`tonic-prost` dependencies — tonic 0.14 split prost
codegen out of `tonic-build` into this crate, 2026-09-07). `App::after_routes` spawns `crate::api::grpc::serve` as
a background task on `GRPC_PORT` (config `server.grpc_port`, default
`50051`) alongside the REST router, sharing one cloned `AppState` — a
bind/serve failure is logged, not fatal, so REST still boots if the
gRPC port is unavailable.

**No duplicated business logic.** `src/api/grpc/service.rs`'s
`EventGrpcService` calls the exact same functions the REST handlers
do: `crate::validation::validate_event`, the shared duplicate-detection
core (`handlers::check_duplicates_internal`, bumped to `pub(crate)`
rather than copied), and the same `EventRepository` trait methods
(which take no `AuditContext`, like worker's). `ListEvents` calls
`EventRepository::list_active` directly rather than mirroring a REST
handler — this crate has **no REST list endpoint at all** (confirmed
by grep, not assumed), so the repository method itself, real
already-tested domain logic, is what the RPC delegates to.
`event_status` parses via the domain enum's existing `serde`
implementation in both directions (`EventStatus` has no `Display` impl
unlike `WorkerType`, so there is no shortcut for the output side
either) rather than a hand-rolled mapping.

**Auth parity, and a genuine simplification confirmed by reading REST,
not assumed.** `grpc_enforce` (in `service.rs`) mirrors this crate's
blanket-guard `require_auth_mw`, gated by the same `EVENT_REQUIRE_AUTH`
flag. Unlike person's/worker's gRPC slices, there is **no record-level
ABAC pass** to add here: this crate's own `create_event`/`get_event`/
`delete_event` REST handlers apply only the blanket guard too, with no
`authorize_record` call to mirror.

**Deliberately not carried over yet** (tracked in spec §13 T-6, not
silently missing): `UpdateEvent` (no RPC); match/merge/search/FHIR
over gRPC; and most of the schema.org/Event domain model's fields on
the proto `Event` message (identifiers, location, organizer,
performer, offers, …) — the message is a deliberate partial projection
(id, name, start/end date, `event_status`, timestamps), not a 1:1
mirror of REST/FHIR.

`tests/grpc_integration_test.rs` proves it end to end against a real
Postgres: binds the server on an OS-assigned port (the same
`TcpListener::bind("127.0.0.1:0")` + `serve_with_incoming` pattern
`tests/otlp_collector` uses), connects a real
`event_service_client::EventServiceClient`, and drives a
Create→Get→List→Delete→Get(`NOT_FOUND`) round trip plus three
error-path proofs (a blank name → `INVALID_ARGUMENT`; an unrecognised
`event_status` → `INVALID_ARGUMENT`; a malformed id →
`INVALID_ARGUMENT`, not `INTERNAL`). `#[ignore]`d like the REST
integration suite; run with `cargo test --test grpc_integration_test
-- --ignored` against `scripts/test-db.sh up
event/event-service-with-axum`. A manual `grpcurl
EventService.GetEvent` smoke check (this crate's spec §13 T-6's
original acceptance criterion) was not additionally run in this
sandbox (`grpcurl` unavailable) — the automated test proves the
identical claim, repeatably.

## Container image

`Dockerfile` (multi-stage, Debian 13 slim runtime) builds this crate's
production image. **Build context must be the repository root**, not
this directory — this crate's sibling path dependencies
(`integrity-mac`, `authentication-verifier`, and — since the
coordinate-field rename, PRO-H2 — its matcher `event-matcher`, pending
a 0.7.0 crates.io publish, see `Cargo.toml`) live outside
`event/event-service-with-axum/`:

```sh
podman build -f event/event-service-with-axum/Dockerfile \
  -t event-service .   # run from the repository root
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
| `README.md` (symlink to `index.md`) | User-facing intro — must stay consistent with the spec |
| `AGENTS.md` / `agents/*.md` / `CLAUDE.md` | How to work in the repo + per-topic reference. `CLAUDE.md` is a one-line `@AGENTS.md` include (matching the family convention — see root `AGENTS.md`), not a second user-facing doc; its content used to duplicate `index.md` and has been folded/removed rather than kept in sync by hand. |
| `index.md` | Navigation aid with worked examples |
| `CHANGELOG.md` | Historical record of releases and changes |
