## 11. Testing Strategy

| Layer | Tool | Scope |
|---|---|---|
| Unit | `cargo test --lib` | 42 tests across db / matching / matching::adapter / search / validation / streaming / privacy / metrics / api::rest (router + handlers). |
| Bridge | `cargo test --test duplicate_detection` | 14 tests pinning the service↔canonical-matcher contract. |
| Integration | `cargo test --test api_integration_test -- --ignored` | 12 `#[ignore]`-tagged tests driving `tower::oneshot` against the hand-built Axum router (`create_router`, retained for tests; the binary serves the same surface via loco controllers) with real Postgres + Tantivy. Requires `DATABASE_URL` against a migrated DB (see `docker-compose.yml`'s `postgres` service). |
| Benchmarks | `cargo bench` | matching + search + validation (Criterion). |

See [`AGENTS/testing.md`](../AGENTS/testing.md) for the full layout.

