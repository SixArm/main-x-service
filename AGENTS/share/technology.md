# Technology stack

This is the short summary. The full inventory (with feature flags and rationale) is in [stack-for-rust-loco.md](stack-for-rust-loco.md). The web tier is documented separately in [web-stack.md](web-stack.md).

## Runtime

- **Rust** 2024 edition
- **Tokio** 1.x — async runtime, multi-threaded
- **PostgreSQL** 15+ (production); SQLite supported for some dev/test paths

## Web

- **Axum** 0.7 — HTTP handlers, extractors, routing
- **Loco.rs** 0.14 — full-stack conventions (config, hooks, workers, static middleware)
- **Tera** 1.20 — server-rendered templates
- **HTMX** 2.0 — server-driven AJAX
- **Alpine.js** 3.14 — light client-side reactivity
- **Lily Design System — HTML Headless** — accessible component contracts (ARIA + semantic HTML + class names; no CSS)
- **NHS UK theme** (`lily.css`) — consumer-supplied visual layer that styles the Lily class names
- **tower-http** 0.6 — CORS, compression, tracing, `ServeDir` (`fs` feature)

## Data

- **SeaORM** 1.1 — async ORM with migrations
- **sea-orm-migration** — schema migrations
- **bigdecimal**, **chrono**, **uuid** — domain types

## Search

- **Tantivy** 0.22 — embedded full-text search index

## RPC

- **Tonic** 0.12 — gRPC server + client
- **prost** 0.13 — Protocol Buffers

## API documentation

- **utoipa** 5.x + **utoipa-swagger-ui** — OpenAPI schema + Swagger UI

## Event streaming

- **Fluvio** 0.23 — durable streaming

## Observability

- **tracing** + **tracing-subscriber** — structured logs
- **opentelemetry** + **opentelemetry-otlp** + **tracing-opentelemetry** — OTLP export

## Security

- **argon2** — password hashing
- **jsonwebtoken** — JWT auth

## Quality

- **validator** — declarative validation
- **strsim** + **fuzzy-matcher** — string similarity

## Testing

- **assertables**, **mockall**, **tempfile**, **tokio-test**
- **criterion** — benchmarks
