### Rust tracing opentelemetry stack

- Structured logging with `tracing` crate
- Telemetry with `opentelemetry` crate
- Configurable log levels (RUST_LOG)
- Request/response logging
- Error logging with context

## Status (2026-08-28)

This document describes the **target** shape.
[`link-graph-service`](../../link/link-graph-service-with-loco/) implemented
it first (`src/observability.rs` — the family's first working exporter,
proved against a real in-process OTLP/gRPC collector in its
`tests/otlp_export.rs` / `tests/otlp_middleware.rs`).
[`person-service`](../../person/person-service-with-loco/) is a close port
of it (repo `tasks.md` PRO-H9, 2026-08-28), with the same in-process-collector
test tier ported alongside. **worker and event still carry the original
stub** (`src/observability/`, exporter and `tracing_opentelemetry` layer
commented out behind `// TODO: Initialize OTLP exporter`) and should copy
person's port next — its `AGENTS.md` documents the two adaptations its
shape needed beyond link-graph-service's (the tower middleware wired onto
two router-construction surfaces instead of one, since person carries a
hand-rolled `create_router` alongside the loco-native path; and a renamed
`tonic` dev-dependency, `otlp-test-tonic = { package = "tonic", … }`, so
the in-process collector's tonic 0.14 does not collide with the crate's
own `tonic = "0.12"` gRPC-stub dependency in a test binary's extern
prelude — worker and event both carry that same gRPC-stub dependency, so
both will need the same rename). Three things settled that this doc does
not say:

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
