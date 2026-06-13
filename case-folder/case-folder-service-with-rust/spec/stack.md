# Stack & versions

> Part of the [Loco edition specification](index.md).

| Layer                  | Choice                                      | Pin                   |
| ---------------------- | ------------------------------------------- | --------------------- |
| Web framework          | [Loco](https://loco.rs) (on Axum 0.8)       | `loco-rs = "0.16"`    |
| Serialization          | `serde` + `serde_json`                      | `serde = "1"`         |
| Database               | PostgreSQL 14+ (boot-time only — no tables) | server-side           |
| ORM                    | [SeaORM](https://www.sea-ql.org/SeaORM/)    | `sea-orm = "1.1"`     |
| Migrations             | `sea-orm-migration`                         | `1.1`                 |
| Async runtime          | Tokio (rustls)                              | `1.45`                |
| HTTP client (upstream) | `reqwest` (rustls)                          | `0.12`                |
| Rust toolchain         | stable                                      | `rust-toolchain.toml` |
| Observability          | OpenTelemetry                               |                       |
| Metrics                | Prometheus                                  |                       |

**Loco-version caveat.** Loco's public Rust API moves between minor
releases. This spec describes the project as written for `0.16`. If the
crate has moved on (e.g. the `Routes` builder or `create_app` signature
has changed), `app.rs` and the controllers will need small adjustments —
the domain model and JSON shapes are stable.
