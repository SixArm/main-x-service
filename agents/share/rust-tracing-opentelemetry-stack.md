### Rust tracing opentelemetry stack

- Structured logging with `tracing` crate
- Telemetry with `opentelemetry` crate
- Configurable log levels (RUST_LOG)
- Request/response logging
- Error logging with context

## Status (2026-11-07 — `axa::opentelemetry`)

Every service that exports OTLP does it through axa's `opentelemetry`
feature, the `axa-opentelemetry` crate. Until 2026-11-07 each carried its
own `src/observability.rs`, ported from link-graph-service's, the family's
first working exporter; those copies are gone. A service:

- starts export in `App::init_logger` with
  `axa::opentelemetry::start(&Config::from_env(<service name>, <version>))`,
  and installs axa's logger with `telemetry.layers()` and
  `DIAGNOSTIC_DIRECTIVES`, so the SDK's own export errors are logged;
- layers `axa::opentelemetry::middleware` (the
  `http.server.request.duration` histogram, the route and status on the
  span, and a W3C `traceparent` response header) through its own
  `App::with_telemetry`, outermost of its layers and inside
  `axa::middleware::apply`, whose `http-request` span it reports on;
- calls `telemetry.shutdown()` from `App::on_shutdown`, so the last spans
  are sent;
- proves that wiring in `tests/otlp_export.rs` against an in-process
  OTLP/gRPC collector (`tests/otlp_collector/`). axa tests the pipeline
  itself.

Two operational facts carry over from the old modules:

- **Spans carry what the logs do.** Treat the collector as you treat the
  logs; a span field must not hold personal data or a credential.
- **`RUST_LOG` gains a second job.** With the bridge layer installed the
  filter governs what is **exported**, not only what is logged.

## Stack

| Concern                 | Crate                                                      | Notes                                      |
| ----------------------- | ---------------------------------------------------------- | ------------------------------------------ |
| Structured logging      | `tracing`, `tracing-subscriber`                            | JSON in production, compact in development |
| Metrics + traces export | `axa::opentelemetry` (axa's `opentelemetry` feature)       | OTLP/gRPC; carries `opentelemetry`, `opentelemetry-otlp`, `opentelemetry_sdk` and the `tracing-opentelemetry` bridge |

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
| `OTLP_SERVICE_NAME` | the service's name      | Service.name attribute                    |
| `OTLP_ENDPOINT`     | unset, export off       | OTLP/gRPC collector URL; `https` unless loopback |
| `OTLP_ALLOW_INSECURE` | off                   | `true` allows plain `http` to another host; without it such an endpoint stops the service at boot |

In `config/*.yaml`, the `logger:` block controls format (`compact` / `json`), level, and pretty backtraces.

## Where to look first

- Health: `GET /api/health`
- Audit history: `GET /api/audit/recent`, `GET /api/<plural>/{id}/audit`
- Per-request tracing: every response carries a `traceparent` header when an OTLP endpoint is configured
