### Technology stack for Rust Axum

| Component            | Technology                                        | Purpose                                     |
| -------------------- | ------------------------------------------------- | ------------------------------------------- |
| **Language**         | Rust 2024 Edition; MSRV **1.97** (N-2, `spec/rust-msrv-n-minus-2/index.md`) | Systems programming, performance, safety    |
| **Async Runtime**    | Tokio                                             | Asynchronous I/O and concurrency            |
| **HTTP**             | tower-http                                        | HTTP layer, CORS, compression               |
| **Web Framework**    | Axum, axa                                         | HTTP server and routing (backend-only)      |
| **Database**         | PostgreSQL 18+                                    | Data persistence                            |
| **Database access**  | sqlx, `query!` literals, `sqlx::migrate!` | Literal, compile-checked SQL; plain SQL migrations |
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
- chrono (axa requires it); sqlx's `chrono` feature only, never `time`

¹ `axa` hardcoded `sqlx-sqlite` into every crate that used it. Now that
axa is gone, this rule can be enforced by the dependency graph as well as by
convention.

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
