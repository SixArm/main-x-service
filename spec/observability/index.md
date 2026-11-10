# Observability — monorepo-wide spec

Single source of truth for **observability** across the Main X Index
family of crates: structured logging/tracing, metrics, and distributed
tracing. This spec describes what is *actually wired today* in the repo
and what remains roadmap, so an operator can stand up dashboards and
alerts against the real surface.

It complements the two short briefs:

- [`agents/share/observability.md`](../../agents/share/observability.md) — one-paragraph summary
- [`agents/share/rust-tracing-opentelemetry-stack.md`](../../agents/share/rust-tracing-opentelemetry-stack.md) — the crate stack + env-var reference

Sibling topic specs that exist on disk and are cross-linked below:

- [`../postgresql/index.md`](../postgresql/index.md) — database, extensions, operations (§13)
- [`../restful/index.md`](../restful/index.md) — REST surface, OpenAPI/Swagger

Availability does **not** yet have a `spec/availability/index.md`; that
material lives in the brief
[`agents/share/availability.md`](../../agents/share/availability.md)
and is linked there throughout.

---

## 1. The three pillars as implemented

Observability is structured as the three classic pillars. The table
states the implementation status across the two generations of service
crate (older Axum services vs. controller-style services — see §2).

| Pillar | Crate / mechanism | Where | Status |
|---|---|---|---|
| **Logging / tracing** | `tracing` + `tracing-subscriber` (JSON or compact); axa's `LOG_*` settings; SeaORM SQL query logging via `DB_LOGGING` | every service | implemented |
| **Metrics** | Prometheus text exposition at `GET /metrics.prom` (the `prometheus` crate registry in `src/metrics.rs`) | every service | implemented (all 10 services) |
| **Distributed tracing** | OpenTelemetry OTLP/gRPC export through `axa::opentelemetry` (axa's `opentelemetry` feature) | `App::init_logger` and `App::with_telemetry` in each service's `src/app.rs` | **implemented in 10 services**: link-graph and the nine registries in this repo (person, worker, event, course, place, thing, organization, care-pathway, case). Off unless `OTLP_ENDPOINT` is set (§3.4). |

### 1.1 Logging / tracing

Process-wide logging is set up exactly once at startup.

- **Every service** sets up logging through axa's logger. With OTLP export
  on, `App::init_logger` installs it with `axa::opentelemetry`'s layers
  and `DIAGNOSTIC_DIRECTIVES` (`axa::logger::init_with`); with export off
  it returns `false` and axa installs its plain logger. `App::on_shutdown`
  sends what export still holds.

#### 1.1.1 axa logger settings

axa reads its settings from the environment; there are no config files.

| Variable | Meaning |
|---|---|
| `LOG_ENABLE` | Logging on (default on, off under test) |
| `LOG_LEVEL` | `trace`, `debug`, `info`, `warn`, or `error` |
| `LOG_FORMAT` | `compact`, `pretty`, or `json`; use `json` for machine ingestion |
| `LOG_FILTER` | A full `EnvFilter` directive, including third-party crates |
| `RUST_LOG` | Overrides the filter outright when set |

#### 1.1.2 SQL query logging

SeaORM statement logging is gated by axa's `DB_LOGGING` setting
(default off). Enable it to surface every SQL statement on the
`tracing` stream during development; leave it **off** in production —
query-level statistics belong to PostgreSQL `pg_stat_statements`
instead (see [`../postgresql/index.md`](../postgresql/index.md) §13 and
§5 below).

### 1.2 Metrics

The older Axum services own a process-wide `prometheus::Registry`
(`src/metrics.rs`) populated once via a `LazyLock<Metrics>` static
(`METRICS`). Application code increments handles directly, e.g.
`METRICS.person_created_total.inc()`. The registry is rendered to
Prometheus text-exposition format and served at `GET /metrics.prom`
(see §2).

### 1.3 Distributed tracing

OTLP export is **active in 10 services**, through `axa::opentelemetry`:
link-graph and the nine registries in this repo. Each:

- starts export in `App::init_logger` from `Config::from_env` (§3.4);
- layers `axa::opentelemetry::middleware` with `App::with_telemetry`,
  outermost of its own layers and inside `axa::middleware::apply`, so
  each request's metric covers versioning and the auth guard, and its
  response carries a W3C `traceparent`;
- sends the last spans from `App::on_shutdown`;
- proves that wiring in `tests/otlp_export.rs` against an in-process
  OTLP/gRPC collector.

The exported request span is axa's `http-request`. Until 2026-11-07 each
service carried its own `src/observability.rs`, ported in turn from
link-graph-service's (repo `tasks.md` PRO-H9, PRO-H12), which named the
span `http.server.request`.

---

## 2. The Prometheus endpoint

### 2.1 Surface

| Property | Value |
|---|---|
| Method / path | `GET /metrics.prom` |
| Content-Type | `text/plain; version=0.0.4; charset=utf-8` |
| Mount point | service **root** — not under `/api` |
| Handler | `api::rest::handlers::metrics_prom` → `crate::metrics::METRICS.render()` |
| Scrape config | `metrics_path: /metrics.prom` |

The endpoint is mounted at the root precisely so a default Prometheus
scrape job finds it. The renderer uses `prometheus::TextEncoder`; the
content-type constant is `crate::metrics::CONTENT_TYPE`.

### 2.2 Metric inventory

The fixed metric set registered by `Metrics::new()` (names shown for
the person service; other older services use the same shape with their
own entity prefix, e.g. `worker_created_total`):

| Name | Type | Labels | Meaning |
|---|---|---|---|
| `<entity>_created_total` | counter | — | records created |
| `<entity>_updated_total` | counter | — | records updated |
| `<entity>_deleted_total` | counter | — | records soft-deleted |
| `<entity>_matched_total` | counter | — | match operations performed |
| `http_requests_total` | counter vec | `method`, `path`, `status` | HTTP requests handled |
| `http_request_duration_seconds` | histogram | — | end-to-end request latency |
| `<entity>_match_score` | histogram | — | match-confidence scores in `[0.0, 1.0]` |
| `<entity>_search_duration_seconds` | histogram | — | search query latency |

Histogram buckets are tuned per metric:

- `http_request_duration_seconds`: `0.001 … 10.0` s (12 buckets)
- `<entity>_match_score`: `0.0 … 1.0`, denser near the
  certain/probable thresholds (`0.85`, `0.9`, `0.95`)
- `<entity>_search_duration_seconds`: `0.001 … 2.5` s

### 2.3 Which services expose it

| Generation | Services | `/metrics.prom`? |
|---|---|---|
| Older Axum services | person, worker, place, thing, event, course | **Yes** — `src/metrics.rs` + handler |
| axa-native services | organization, care-pathway, case, portfolio, authentication | **Yes** — `src/metrics.rs` + `src/controllers/metrics.rs` |

Every service exposes the Prometheus surface at the root path
`/metrics.prom` (public, `text/plain; version=0.0.4`) alongside axa's
default `/_health/live`, `/_health/ready`, `/_health/startup`. The two generations differ only in how
the route is wired (an Axum `metrics_prom` handler vs. an axa controller
`Routes`), not in the exposition format.

### 2.4 Scrape configuration

```yaml
# prometheus.yml
scrape_configs:
  - job_name: main-x-index
    metrics_path: /metrics.prom
    static_configs:
      - targets:
          - person-service:8080
          - worker-service:8080
          - place-service:8080
          - thing-service:8080
          - event-service:8080
```

---

## 3. Tracing / OTLP setup

### 3.1 Stack

Per [`agents/share/rust-tracing-opentelemetry-stack.md`](../../agents/share/rust-tracing-opentelemetry-stack.md):

| Concern | Crate | Notes |
|---|---|---|
| Structured logging | `tracing`, `tracing-subscriber` | JSON in production, compact in dev |
| Metrics + traces export | `axa::opentelemetry` | OTLP/gRPC, with the `tracing-opentelemetry` bridge; carries `opentelemetry`, `opentelemetry-otlp`, `opentelemetry_sdk` |

### 3.2 Spans and events

Intended span coverage (emitted at call sites via `tracing` macros):

- a span per HTTP request,
- a span per DB query,
- a span per match-scoring run,
- a span per search query,
- `info` / `warn` / `error` events for create / update / delete.

Audit-log entries are written to the `audit_log` table and are a
**separate** stream from traces (see §4.4).

### 3.3 Log levels

`RUST_LOG` is the `tracing-subscriber::EnvFilter` directive (default
`info`). It overrides the configured `log_level` / axa `logger.level`
when present in the process environment. Examples:

```bash
RUST_LOG=info cargo run --release
RUST_LOG=debug,sea_orm=warn cargo run    # quiet the ORM
```

### 3.4 OTLP exporter and `Resource`

| Variable | Default | Purpose |
|---|---|---|
| `RUST_LOG` | unset | `EnvFilter` directive; with export on it also governs what is exported |
| `OTLP_SERVICE_NAME` | the service's name | `service.name` attribute |
| `OTLP_ENDPOINT` | unset, export off | OTLP/gRPC collector URL |
| `OTLP_ALLOW_INSECURE` | off | allows plain `http` to a host other than this one |

The `Resource` carries `service.name` and `service.version`
(`CARGO_PKG_VERSION`); `OTEL_RESOURCE_ATTRIBUTES` adds to it, for example
`service.instance.id`. An endpoint must be `https` or loopback unless
`OTLP_ALLOW_INSECURE` is set, and anything else stops the service at boot,
because spans carry what the logs do. A collector that is down is not an
error: spans wait in a bounded queue and are dropped when it fills.

### 3.5 Correlation across services

The services form a federation (one per entity) plus the central
[authentication-service](../../authentication/authentication-service-with-axum).
Each response carries the `traceparent` of its own trace, so an operator
can go from a response to its trace. `axa::opentelemetry` does **not**
continue a trace from an incoming `traceparent`, so each service's trace
starts at its own edge; a request is followed across services by its
request id (`axa_tracing::propagate`, not yet adopted here).

---

## 4. What to instrument

### 4.1 Request lifecycle

Every HTTP request increments `http_requests_total{method,path,status}`
and observes `http_request_duration_seconds`. This is the primary
RED-method signal (Rate / Errors / Duration); alert on the `status`
label for 5xx rate and on the duration histogram for latency SLOs. See
[`../restful/index.md`](../restful/index.md) for the endpoint surface
and status-code conventions.

### 4.2 Matching and search latency

- Matching: `<entity>_matched_total` (volume), `<entity>_match_score`
  (the confidence distribution — watch for drift around the
  certain/probable thresholds).
- Search: `<entity>_search_duration_seconds` (Tantivy query latency).

These are the most CPU-intensive paths and the first place to look when
duration SLOs regress.

### 4.3 Database pool

Connection-pool saturation is a leading indicator of latency. Pool
sizing is configured per service (`DATABASE_MAX_CONNECTIONS` /
`DATABASE_MIN_CONNECTIONS` on the older services; axa
`database.max_connections` / `min_connections` on the controller-style services).
Surface pool gauges as the pool integration matures; until then, watch
DB-query span latency and PostgreSQL-side metrics (§5). Pooling itself
is part of availability — see
[`agents/share/availability.md`](../../agents/share/availability.md).

### 4.4 Background jobs

Per [`agents/share/axa.md`](../../agents/share/axa.md), background
jobs run on a **Postgres-backed** queue (not SQLite), with
`num_workers: 2`. Instrument job throughput, failures, and queue depth
once workers carry domain work; today the queue is configured but
largely idle.

### 4.5 Audit and event publishing

Two write-time side channels accompany every CRUD operation, both
distinct from the trace stream:

- **Audit log** → `audit_log` table (old/new JSON values,
  `user_id` / `user_ip_address` / `user_agent`, timestamp). Queryable
  over REST (`GET /api/audit/recent`, `GET /api/<plural>/{id}/audit`).
  See [`agents/share/auditability.md`](../../agents/share/auditability.md).
- **Event stream** → `*Created` / `*Updated` / `*Deleted` /
  `*Merged` / `*Linked` / `*Unlinked` events. In-memory by default
  (`<ENTITY>_EVENT_TRANSPORT=memory`); a durable Postgres-outbox +
  `FluvioSink` relay is **shipped, default-off, on all ten entity
  registries** (see [`spec/event-streaming/index.md`](../event-streaming/index.md)
  §4/§8) — a deployment opts in by flipping the transport, it is not
  unwritten code. The controller-style services expose a frozen `EventView`
  projection at `GET /api/<plural>/events/recent`.

Instrument publish counts and failures so a stalled audit/event writer
is visible.

---

## 5. Operational signals

### 5.1 Health checks

| Generation | Endpoint |
|---|---|
| Older Axum services | `GET /api/health` |
| axa-native services | `GET /_health/live`, `GET /_health/ready`, `GET /_health/startup` |

These back container/orchestrator liveness and readiness probes (Podman
health checks, non-root execution, graceful shutdown). The full
availability story — pooling, horizontal scaling, graceful shutdown —
is in [`agents/share/availability.md`](../../agents/share/availability.md).

### 5.2 PostgreSQL signals / slow queries

Enable the `pg_stat_statements` extension for execution-statistics and
slow-query analysis, as specified in
[`../postgresql/index.md`](../postgresql/index.md) §13 (Operations).
This is the canonical source for per-statement latency — prefer it over
SeaORM `DB_LOGGING` in production (§1.1.2). Other relevant
extensions (`pg_trgm` for the `ILIKE` name/title search, `postgis`
where geo is modeled) are catalogued in the same postgresql spec.

### 5.3 First places to look

1. `GET /api/health` (or `/_health/live`) — is the process up?
2. `/metrics.prom` `http_requests_total{status=~"5.."}` — error rate (older services).
3. `http_request_duration_seconds` p95/p99 — latency SLO.
4. `pg_stat_statements` — slow queries / DB pressure.
5. `GET /api/audit/recent` — recent write activity, who/what/when.

---

## 6. Implemented vs. roadmap

### 6.1 Implemented today

- JSON / compact structured `tracing` with `RUST_LOG` and axa's
  `LOG_*` settings; per-call `service.name` / `service.version`.
- Prometheus `/metrics.prom` text exposition on the **6 older Axum
  services** (person, worker, place, thing, event, **and course** —
  this list previously omitted course, contradicting §2.3, which has
  always listed all six correctly) plus organization, care-pathway,
  case, portfolio, and authentication (§2.3): CRUD counters,
  `http_requests_total`, latency + match-score histograms.
- **OpenTelemetry OTLP export on 10 services** (§1.3) through
  `axa::opentelemetry`, each proved against an in-process collector.
- SeaORM SQL query logging toggle (`DB_LOGGING`).
- Health endpoints on every service; audit-log table + query API;
  event stream (in-memory by default, durable outbox + Fluvio relay
  shipped default-off on all ten registries — §4.5).

### 6.2 Roadmap

| Item | Status | Detail |
|---|---|---|
| Continuing a caller's trace | not offered by `axa::opentelemetry` | each service's trace starts at its edge; follow a request by its request id |
| DB connection-pool gauges | deferred, family-wide | surface pool saturation as a first-class metric |
| Durable event bus | **shipped**, default-off, all ten registries | see [`spec/event-streaming/index.md`](../event-streaming/index.md) — not a roadmap item any more; only actually flipping a deployment's transport, and building the search-re-indexer/cache/analytics consumers, remain open |
| Background-job metrics | deferred, family-wide | Postgres-backed queue is configured but idle |

---

## 7. Cross-references

| Topic | Location |
|---|---|
| Observability summary brief | [`agents/share/observability.md`](../../agents/share/observability.md) |
| Tracing / OTLP crate stack + env vars | [`agents/share/rust-tracing-opentelemetry-stack.md`](../../agents/share/rust-tracing-opentelemetry-stack.md) |
| Audit logging + event streaming | [`agents/share/auditability.md`](../../agents/share/auditability.md) |
| Availability / health / pooling | [`agents/share/availability.md`](../../agents/share/availability.md) |
| PostgreSQL extensions + operations (§13) | [`../postgresql/index.md`](../postgresql/index.md) |
| REST surface + status codes | [`../restful/index.md`](../restful/index.md) |
| axa conventions + background jobs | [`agents/share/axa.md`](../../agents/share/axa.md) |
