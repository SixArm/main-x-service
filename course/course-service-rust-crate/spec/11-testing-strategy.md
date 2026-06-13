## 11. Testing Strategy

| Layer | Tool | Scope |
|---|---|---|
| Unit | `cargo test --lib` | 35 tests across db / matching / matching::adapter / search / validation / streaming / privacy / handlers. |
| Bridge | `cargo test --test duplicate_detection` | 14 tests pinning the service↔canonical-matcher contract. |
| Integration | `cargo test --test api_integration_test -- --ignored` | 12 `#[ignore]`-tagged tests over the full Axum router with real Postgres + Tantivy. Requires `DATABASE_URL` against a migrated DB (see `docker-compose.yml`'s `postgres` service). |
| Benchmarks | `cargo bench` | matching + search + validation (Criterion). |

See [`AGENTS/testing.md`](../AGENTS/testing.md) for the full layout.

