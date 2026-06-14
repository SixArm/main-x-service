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
crate (older Axum services vs. loco-native services — see §2).

| Pillar | Crate / mechanism | Where | Status |
|---|---|---|---|
| **Logging / tracing** | `tracing` + `tracing-subscriber` (JSON or compact); loco `logger:` config block; SeaORM SQL query logging via `database.enable_logging` | every service | implemented |
| **Metrics** | Prometheus text exposition at `GET /metrics.prom` (the `prometheus` crate registry in `src/metrics.rs`) | every service | implemented (all 10 services) |
| **Distributed tracing** | OpenTelemetry OTLP export, `opentelemetry-semantic-conventions` | `src/observability/` bootstrap | scaffolded; OTLP exporter stubbed (`todo!` / commented) |

### 1.1 Logging / tracing

Process-wide logging is set up exactly once at startup.

- **Older Axum services** — `src/observability/mod.rs::init_telemetry`
  builds an OTel `Resource` tagging `service.name` / `service.version`,
  then installs a JSON `tracing` subscriber. The level comes from the
  `RUST_LOG` env filter (`EnvFilter::try_from_default_env`), falling
  back to the configured `log_level`. `shutdown_telemetry` flushes the
  tracer provider on graceful exit.
- **Loco-native services** — logging is driven by loco's `logger:`
  config block (see §1.1.1), not a hand-rolled subscriber.

#### 1.1.1 Loco `logger:` config block

The loco services configure logging declaratively in
`config/<env>.yaml`:

```yaml
logger:
  enable: true
  pretty_backtrace: true       # sets RUST_BACKTRACE=1
  level: debug                 # trace | debug | info | warn | error
  format: compact              # compact | pretty | json
  # override_filter: trace     # uncomment to include third-party crates
```

| Field | development / test | production |
|---|---|---|
| `level` | `debug` | loco default (`info`) |
| `format` | `compact` | loco default |
| `pretty_backtrace` | `true` | — |

Production `config/production.yaml` omits the `logger:` block and
inherits loco defaults; tune it there before going live (set
`format: json` for machine ingestion, `level: info`).

#### 1.1.2 SQL query logging

SeaORM statement logging is gated by the loco `database.enable_logging`
flag (default `false` in `config/development.yaml` and
`config/test.yaml`). Enable it to surface every SQL statement on the
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

The OTLP pipeline is **scaffolded but not yet active**. In
`src/observability/mod.rs` the OTLP exporter and the
`tracing_opentelemetry` layer are commented out (`TODO`), and the
`custom_metrics::*Metrics` OpenTelemetry instrument set
(`Counter`/`Histogram`) is constructed by a `todo!()` stub. The
`traces.rs` and `metrics.rs` submodules under `src/observability/` are
reserved placeholders for span/context propagation helpers and the OTLP
metric counterpart respectively. See §3 and §6.

---

## 2. The Prometheus endpoint

### 2.1 Surface

| Property | Value |
|---|---|
| Method / path | `GET /metrics.prom` |
| Content-Type | `text/plain; version=0.0.4; charset=utf-8` |
| Mount point | service **root** — not under `/api/v1` |
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
| Loco-native services | organization, care-pathway, case, authentication | **Yes** — `src/metrics.rs` + `src/controllers/metrics.rs` |

Every service exposes the Prometheus surface at the root path
`/metrics.prom` (public, `text/plain; version=0.0.4`) alongside loco's
default `/_health` and `/_ping`. The two generations differ only in how
the route is wired (an Axum `metrics_prom` handler vs. a loco controller
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
| Metrics + traces export | `opentelemetry`, `opentelemetry-otlp`, `opentelemetry_sdk` | OTLP gRPC or HTTP |
| Bridge | `tracing-opentelemetry` | forwards `tracing` spans to OTel |
| Semantic conventions | `opentelemetry-semantic-conventions` | HTTP / DB / RPC attributes |

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
`info`). It overrides the configured `log_level` / loco `logger.level`
when present in the process environment. Examples:

```bash
RUST_LOG=info cargo run --release
RUST_LOG=debug,sea_orm=warn cargo run    # quiet the ORM
```

### 3.4 OTLP exporter and `Resource`

The exporter targets an OTLP collector. Environment variables (see the
stack brief):

| Variable | Default | Purpose |
|---|---|---|
| `RUST_LOG` | `info` | `EnvFilter` directive |
| `OTLP_SERVICE_NAME` | crate name | `service.name` attribute |
| `OTLP_ENDPOINT` | `http://localhost:4317` | OTLP collector endpoint |

The OTel `Resource` is tagged with `service.name` (from config) and
`service.version` (`CARGO_PKG_VERSION`) at bootstrap. The exporter
wiring itself is currently commented out (§6); once enabled, each
response carries a `traceparent` header for cross-service correlation.

### 3.5 Correlation across services

The services form a federation (one per entity) plus the central
[authentication-service](../../authentication/authentication-service-rust-crate).
Trace correlation relies on W3C `traceparent` propagation: an inbound
request's context is extracted, attached to the request span, and
propagated outbound on DB / RPC / HTTP calls. The reserved
`src/observability/traces.rs` is where the extraction/propagation
helpers land. Until OTLP is wired, correlation is best-effort via the
shared `service.name` / request-path log fields.

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
`DATABASE_MIN_CONNECTIONS` on the older services; loco
`database.max_connections` / `min_connections` on the loco services).
Surface pool gauges as the pool integration matures; until then, watch
DB-query span latency and PostgreSQL-side metrics (§5). Pooling itself
is part of availability — see
[`agents/share/availability.md`](../../agents/share/availability.md).

### 4.4 Background jobs

Per [`agents/share/loco.md`](../../agents/share/loco.md), background
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
  `*Merged` / `*Linked` / `*Unlinked` events. In-memory today
  (durable bus is roadmap); the loco services expose a frozen
  `EventView` projection at `GET /api/<plural>/events/recent`.

Instrument publish counts and failures so a stalled audit/event writer
is visible.

---

## 5. Operational signals

### 5.1 Health checks

| Generation | Endpoint |
|---|---|
| Older Axum services | `GET /api/health` |
| Loco-native services | `GET /_health`, `GET /_ping` |

These back container/orchestrator liveness and readiness probes (Podman
health checks, non-root execution, graceful shutdown). The full
availability story — pooling, horizontal scaling, graceful shutdown —
is in [`agents/share/availability.md`](../../agents/share/availability.md).

### 5.2 PostgreSQL signals / slow queries

Enable the `pg_stat_statements` extension for execution-statistics and
slow-query analysis, as specified in
[`../postgresql/index.md`](../postgresql/index.md) §13 (Operations).
This is the canonical source for per-statement latency — prefer it over
SeaORM `enable_logging` in production (§1.1.2). Other relevant
extensions (`pg_trgm` for the `ILIKE` name/title search, `postgis`
where geo is modeled) are catalogued in the same postgresql spec.

### 5.3 First places to look

1. `GET /api/health` (or `/_health`) — is the process up?
2. `/metrics.prom` `http_requests_total{status=~"5.."}` — error rate (older services).
3. `http_request_duration_seconds` p95/p99 — latency SLO.
4. `pg_stat_statements` — slow queries / DB pressure.
5. `GET /api/audit/recent` — recent write activity, who/what/when.

---

## 6. Implemented vs. roadmap

### 6.1 Implemented today

- JSON / compact structured `tracing` with `RUST_LOG` + loco
  `logger:` config; per-call `service.name` / `service.version`.
- Prometheus `/metrics.prom` text exposition on the **5 older Axum
  services** (person, worker, place, thing, event): CRUD counters,
  `http_requests_total`, latency + match-score histograms.
- SeaORM SQL query logging toggle (`database.enable_logging`).
- Health endpoints on every service; audit-log table + query API;
  in-memory event stream.

### 6.2 Roadmap

| Item | Status | Blocking detail |
|---|---|---|
| Full OTLP exporter wiring | scaffolded | exporter + `tracing_opentelemetry` layer commented out in `src/observability/mod.rs`; `custom_metrics::*Metrics::new()` is `todo!()` |
| `traceparent` propagation helpers | reserved | `src/observability/traces.rs` placeholder |
| OTLP metric counterpart | reserved | `src/observability/metrics.rs` placeholder (mirrors the Prometheus set over OTLP) |
| DB connection-pool gauges | deferred | surface pool saturation as a first-class metric |
| Durable event bus | deferred | replace in-memory stream (Fluvio per the stack reference) |
| Background-job metrics | deferred | Postgres-backed queue is configured but idle |

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
| Loco conventions + background jobs | [`agents/share/loco.md`](../../agents/share/loco.md) |
