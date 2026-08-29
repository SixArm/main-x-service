### Technology stack for Rust Loco

| Component            | Technology                                        | Purpose                                     |
| -------------------- | ------------------------------------------------- | ------------------------------------------- |
| **Language**         | Rust 2024 Edition; MSRV **1.96** (N-2, `spec/rust-msrv-n-minus-2/index.md`) | Systems programming, performance, safety    |
| **Async Runtime**    | Tokio                                             | Asynchronous I/O and concurrency            |
| **HTTP**             | tower-http                                        | HTTP layer, CORS, compression               |
| **Web Framework**    | Axum, Loco                                        | HTTP server and routing (backend-only)      |
| **Database**         | PostgreSQL 18+                                    | Data persistence                            |
| **ORM**              | SeaORM, sea-orm-migration                         | Async database object-relational mapper     |
| **Search Engine**    | Tantivy                                           | Embed full-text search indexing             |
| **Data Streaming**   | Fluvio                                            | Event publishing and durable data streaming |
| **API Docs**         | Utoipa                                            | OpenAPI 3.0 specification                   |
| **Swagger Docs **    | utoipa-swagger-ui                                 | OpenAPI 3.0 specification                   |
| **Serialization**    | Serde                                             | JSON serialization/deserialization          |
| **Logging**          | Tracing                                           | Structured logging                          |
| **Observability**    | OpenTelemetry, opentelemetry-semantic-conventions | Distributed tracing, metrics, spans, logs   |
| **String Matching**  | strsim, fuzzy-matcher                             | Jaro-Winkler, Levenshtein                   |
| **Geo**              | geo, haversine                                    | Coordinate distance calculations            |
| **Containerization** | Podman                                            | Deployment OpenContainer packaging          |
| **gRPC**             | Tonic                                             | High-performance RPC framework              |
| **Protocols**        | Prost                                             | Protocol buffers                            |
| **Environment**      | dotenvy                                           | Config env var                              |
| **Timestamps**       | chrono                                              | Dates, times, durations                     |
| **Error Handling**   | thiserror, anyhow                                 | Typed and contextual error handling         |
| **Security**         | argon2                                            | Password hashing                            |
| **Authentication**   | rusty_paseto, authentication-verifier             | PASETO v4.public cross-service tokens (offline verification); cookie sessions for humans |
| **Testing**          | assertables, tokio-test                           | Unit testing, integration                   |
| **Mock Testing**     | mockall, tempfile                                 | Mock testing                                |
| **Benchmarking**     | Criterion                                         | Statistical performance benchmarking        |
| **Memory Allocator** | MiMalloc                                          | High-performance MUSL static builds         |
| **Numbers**          | bigdecimal                                        |                                             |
| **Identifiers**      | uuid                                              | UUID generation                             |
| **Validation**       | validator                                         | Declarative validation                      |

Constraints:

- Podman NOT Docker
- Tokio NOT async_std
- MiMalloc NOT jemalloc
- PostgreSQL NOT SQLite¹
- chrono (sea-orm and loco-rs require it; sea-orm has no `with-jiff` feature)
- sea-orm feature "with-chrono" (loco services) or "with-time" (older services)

¹ This governs which driver we connect with and which URLs/config we
ship, not what's compiled in: `loco-rs`'s own `with-db` feature
hardcodes `sea-orm`/`sea-orm-migration`'s `sqlx-sqlite` unconditionally
for every loco-based crate in the family, regardless of what database
features that crate itself requests (verified present in patient-flow,
care-pathway, person, worker, organization, and case's `Cargo.lock`s;
found and documented 2026-08-29, `tasks.md` PRO-P28). No crate here can
remove it from its own manifest; doing so needs an upstream loco-rs
change.

## Configurations

Add to files `lib.rs` `main.rs` immediately after top-level doc comment.

```rust
// Always start with high quality coding conventions.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]

// When we build for MUSL static, use faster memory allocator.
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
```
