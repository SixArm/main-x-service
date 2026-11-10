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

Export comes from `axa::opentelemetry`, axa's `opentelemetry` feature, since
2026-11-07. The crate's own `src/observability.rs`, a port of
person-service's, is gone. `App::init_logger` starts it and installs
axa's logger with its layers, `App::with_telemetry` layers its middleware
(the `http.server.request.duration` metric and a `traceparent` response
header) inside axa's standard stack, outermost of the service's own layers
so a `401` or `403` is measured, and `App::on_shutdown` sends what is
queued. Spans are axa's per-request `http-request` span and whatever the
code opens inside it. There is one router-construction surface,
`App::after_routes`, so the middleware is layered once.

| Variable | Meaning |
|---|---|
| `OTLP_ENDPOINT` | Collector URL. **Unset or blank means export is off**, where the old module exported to `http://localhost:4317` by default |
| `OTLP_SERVICE_NAME` | Replaces `place-service` |
| `OTLP_ALLOW_INSECURE` | `true` allows plain `http` to a host other than this one. Without it such an endpoint stops the service at boot, because spans carry what the logs do |

The variables are deliberately unprefixed, unlike `PLACE_*`.

`tests/otlp_export.rs`, with the in-process collector in
`tests/otlp_collector/`, proves the wiring in a normal `cargo test`: a
request through `App::with_telemetry` and `axa::middleware::apply` reaches
the collector as a span and a metric, and its `traceparent` names the
exported trace. axa tests its own pipeline. None of this needs a database.

The collector's `tonic` 0.14 dev-dependency is still renamed,
`otlp-test-tonic = { package = "tonic", version = "0.14" }`: this crate
declares `tonic = "0.12"` for the not-yet-built gRPC server (T-4), and two
unrenamed `tonic` dependencies at different versions make rustc refuse the
test binary (`E0464: multiple candidates for rlib dependency tonic`).

## Database: sqlx, all SQL in `src/db/`

Since 2026-11-07 this crate is the family's pilot of the move from
SeaORM to sqlx (spec §10, §13 T-18; the repo's
`spec/tech-stack/index.md` §5). The rules that keep it reviewable:

- Every statement is a `sqlx::query!` / `query_as!` / `query_scalar!`
  literal in `src/db/`. Handlers, the relay and tests call functions
  there; they never hold SQL.
- No `QueryBuilder`, `raw_sql` or `AssertSqlSafe`. The clippy stage
  refuses them unless `ci/sql-allowlist.txt` names the file. Write an
  optional filter as `($1::text IS NULL OR col = $1)` instead.
- A helper running several statements takes `&mut PgConnection` (or
  `axa::sqlx::Db`), so the outbox row and the child rows commit in the
  caller's transaction.
- Builds read `.sqlx/` with `SQLX_OFFLINE=true`. After changing a query
  or a migration, migrate a database and run `SQLX_OFFLINE=false cargo
  sqlx prepare -- --all-targets`, then commit `.sqlx/`. CI's `test-db`
  stage runs `cargo sqlx prepare --check`.
- Migrations are `migrations/<version>_<name>.up.sql` / `.down.sql`,
  embedded by `sqlx::migrate!` as `db::MIGRATOR`. Never edit an applied
  one: sqlx checks each file's checksum and refuses a changed one.
- Timestamps are `chrono` in the row structs. Do not turn on sqlx's
  `time` feature: axa enables `chrono`, and with both, sqlx 0.9.0's
  macros warn on every timestamp column even when `sqlx.toml` names one.

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
`place/place-service-with-axum/` (its matcher, `place-matcher`, is
pulled from crates.io, not a path dependency):

```sh
podman build -f place/place-service-with-axum/Dockerfile \
  -t place-service .   # run from the repository root
```

For local development, `podman compose up -d` (from this crate's own
directory) is usually simpler than building the image directly.

Verified end-to-end (2026-08-03): builds clean, boots against a real
Postgres, and `GET /api/health` returns `200`. This exercise found and
fixed two real bugs: Cargo.toml's `[[bench]] bridge_bench` declaration
requires `benches/bridge_bench.rs` to exist even for a `--bin`-only
build (cargo refuses to parse the manifest otherwise), and the
migration SQL directory had to be copied explicitly. Since the move to
sqlx (2026-11-07) the Dockerfile copies `migrations/`, which
`sqlx::migrate!` embeds, and `.sqlx/`, which the `query!` macros read
under `SQLX_OFFLINE=true`; there is no `migration/` crate any more. See
`.containerignore` at the repository root (excludes every crate's
`target/`, or the build context would try to copy hundreds of GB of
build artifacts). The wired multi-service `examples/compose/` stacks
(DEP-1) that build on this are not yet written.
