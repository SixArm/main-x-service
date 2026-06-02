### Technology stack for Rust Loco

| Component            | Technology                                 | Purpose                                  |
| -------------------- | ------------------------------------------ | ---------------------------------------- |
| **Language**         | Rust 1.93+ 2024 Edition                    | Systems programming, performance, safety |
| **Async Runtime**    | Tokio                                      | Asynchronous I/O and concurrency         |
| **Web Framework**    | Axum, Loco                                 | HTTP server and routing (backend-only)   |
| **Database**         | PostgreSQL 18+                             | Data persistence                         |
| **ORM**              | SeaORM                                     | Async database object-relational mapper  |
| **Search Engine**    | Tantivy                                    | Full-text search indexing                |
| **Data Streaming**   | Fluvio                                     | Event publishing and data streaming      |
| **API Docs**         | Utoipa                                     | OpenAPI 3.0 specification                |
| **Serialization**    | Serde                                      | JSON serialization/deserialization       |
| **Logging**          | Tracing                                    | Structured logging                       |
| **Observability**    | OpenTelemetry                              | Distributed tracing, metrics, logs       |
| **String Matching**  | strsim, fuzzy-matcher                      | Jaro-Winkler, Levenshtein                |
| **Geo**              | geo, haversine                             | Coordinate distance calculations         |
| **Containerization** | Docker                                     | Deployment packaging                     |
| **gRPC**             | Tonic, Prost                               | High-performance RPC framework           |
| **Utilities**        | uuid, chrono, dotenvy                      | UUID generation, timestamps, env config  |
| **Error Handling**   | thiserror, anyhow                          | Typed and contextual error handling      |
| **Security**         | argon2, jsonwebtoken                       | Password hashing, JWT authentication     |
| **Testing**          | assertables, tokio-test, mockall, tempfile | Unit, integration, and mock testing      |
| **Benchmarking**     | Criterion                                  | Statistical performance benchmarking     |
