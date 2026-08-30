### Rust tracing opentelemetry stack

- Structured logging with `tracing` crate
- Telemetry with `opentelemetry` crate
- Configurable log levels (RUST_LOG)
- Request/response logging
- Error logging with context

## Status (2026-08-30)

This document describes the **target** shape.
[`link-graph-service`](../../link/link-graph-service-with-loco/) implemented
it first (`src/observability.rs` — the family's first working exporter,
proved against a real in-process OTLP/gRPC collector in its
`tests/otlp_export.rs` / `tests/otlp_middleware.rs`).
[`person-service`](../../person/person-service-with-loco/) is a close port
of it (repo `tasks.md` PRO-H9, 2026-08-28), with the same in-process-collector
test tier ported alongside; **worker and event** followed in the same
task (also 2026-08-28), each copying person's already-adapted port rather
than re-deriving link-graph-service's. **course and place** are the
newest ports (`tasks.md` PRO-H12, 2026-08-30), both copying person's
port. All six crates' `AGENTS.md` document the adaptations each needed
beyond link-graph-service's shape: the tower middleware wired onto
**two** router-construction surfaces instead of one, for every crate
that (like person/worker/event/course/place) carries a hand-rolled
`create_router` alongside the loco-native path; and — for a crate that
**declares** a `tonic` Cargo dependency at all — a renamed
dev-dependency, `otlp-test-tonic = { package = "tonic", … }`, so the
in-process collector's tonic 0.14 does not collide with it in a test
binary's extern prelude. **This is not the same test as
[overview.md](overview.md)'s gRPC-stub capability row**, which tracks
only whether a `src/api/grpc` module exists: person/worker/event need
the rename because they have both a stub and the dependency; place
needs the rename too despite showing `–` on that row, because it
already declares `tonic` + `tonic-build` in `Cargo.toml` in
anticipation of a not-yet-built gRPC server (its own spec T-4) — Cargo
does not care that no code calls `tonic::` yet, only that the name and
version collide. course needed no rename because it is the one crate
here that declares no `tonic` dependency at all. **Check each
remaining crate's actual `Cargo.toml`, not the capability matrix**,
before assuming either way. **thing, organization, care-pathway, case,
portfolio** carry no observability module at all yet (PRO-H12's
remaining scope); the last four are loco-idiomatic (`src/controllers/`),
not person-style, so their router-construction surface count and
layering point still need working out fresh rather than assumed
identical to the six done so far. Three things settled that this doc
does not say:

- **Versions.** `opentelemetry` / `_sdk` / `-otlp` /
  `-semantic-conventions` **0.32**, `tracing-opentelemetry` **0.33**, with
  the `grpc-tonic` feature (the crate defaults to HTTP + blocking reqwest,
  which does not match the `:4317` endpoint default below). person's
  0.27/0.28 pins are stale — 0.27's `install_batch(runtime::Tokio)`
  pipeline API no longer exists upstream.
- **Where it hooks in.** loco 1.0's `Hooks::init_logger` returning
  `Ok(true)`, composing loco's public `logger::init_env_filter` +
  `logger::init_layer` so the deployment's configured filter policy and
  log format are reused rather than re-derived. `Hooks::on_shutdown`
  flushes.
- **`RUST_LOG` gains a second job.** With the bridge layer installed the
  filter governs what is **exported**, not only what is logged. Loco's
  module whitelist is what keeps the trace stream to the service's own
  spans — and, until widened, also swallows the SDK's own export-failure
  errors, so a broken exporter looks exactly like a working one.

## Stack

| Concern                 | Crate                                                      | Notes                                      |
| ----------------------- | ---------------------------------------------------------- | ------------------------------------------ |
| Structured logging      | `tracing`, `tracing-subscriber`                            | JSON in production, compact in development |
| Metrics + traces export | `opentelemetry`, `opentelemetry-otlp`, `opentelemetry_sdk` | OTLP gRPC or HTTP                          |
| Bridge                  | `tracing-opentelemetry`                                    | Forwards `tracing` spans to OTel           |
| Semantic conventions    | `opentelemetry-semantic-conventions`                       | HTTP, DB, RPC attributes                   |

## What gets emitted

- **Spans** for every HTTP request, every DB query, every match scoring run, every search query
- **Events** at `info` / `warn` / `error` for create/update/delete operations
- **Audit log entries** to the `audit_log` table (separate from the trace stream)
- **Metrics** for request latency histograms, match scoring duration, search hit counts

## Configuration

Environment variables (`.env` or process env):

| Variable            | Default                 | Purpose                                   |
| ------------------- | ----------------------- | ----------------------------------------- |
| `RUST_LOG`          | `info`                  | `tracing-subscriber::EnvFilter` directive |
| `OTLP_SERVICE_NAME` | crate name              | Service.name attribute                    |
| `OTLP_ENDPOINT`     | `http://localhost:4317` | OTLP collector endpoint                   |

In `config/*.yaml`, the `logger:` block controls format (`compact` / `json`), level, and pretty backtraces.

## Where to look first

- Health: `GET /api/health`
- Audit history: `GET /api/audit/recent`, `GET /api/<plural>/{id}/audit`
- Per-request tracing: every response carries a `traceparent` header when an OTLP endpoint is configured
