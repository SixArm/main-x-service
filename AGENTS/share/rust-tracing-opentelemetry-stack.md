### Rust tracing opentelemetry stack

- Structured logging with `tracing` crate
- Telemetry with `opentelemetry` crate
- Configurable log levels (RUST_LOG)
- Request/response logging
- Error logging with context

## Status (2026-08-05)

This document describes the **target** shape. Only
[`link-graph-service`](../../link/link-graph-service-with-loco/) implements
it today (`src/observability.rs` — the family's first and so far only
working exporter, proved against a real in-process OTLP/gRPC collector in
its `tests/otlp_export.rs` / `tests/otlp_middleware.rs`). Copy that, not
person / worker / event's `src/observability/`, whose exporter and
`tracing_opentelemetry` layer are still commented out behind
`// TODO: Initialize OTLP exporter`. Three things it settled that this doc
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
