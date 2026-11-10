# Stack & versions

> Part of the [Axum edition specification](index.md).

| Layer                  | Choice                                      | Pin                   |
| ---------------------- | ------------------------------------------- | --------------------- |
| Web framework          | [axa](https://github.com/publicprojects-io/axa) (on Axum 0.8) | `axa = "0.11"` |
| Serialization          | `serde` + `serde_json`                      | `serde = "1"`         |
| Database               | none — the service owns no tables           | —                     |
| Async runtime          | Tokio (rustls)                              | `1.45`                |
| HTTP client (upstream) | `reqwest` (rustls)                          | `0.12`                |
| Rust toolchain         | stable                                      | `rust-toolchain.toml` |
| Observability          | OpenTelemetry                               |                       |
| Metrics                | Prometheus                                  |                       |

**axa-version caveat.** axa's public Rust API moves between minor
releases. This spec describes the project as written for `0.11`. A later
release may need small adjustments to `app.rs` and the controllers; the
domain model and JSON shapes are stable.
