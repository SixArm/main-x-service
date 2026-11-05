## 11. Testing Strategy

| Layer | Tool | Scope |
|---|---|---|
| Unit | `cargo test --lib` | 123 tests (+2 DB-gated `#[ignore]`) across db / db::outbox / matching / matching::adapter / search / validation / streaming / streaming::envelope / privacy / compliance (record + audit integrity, MAC) / config / relay / fhir / metrics / api::rest (router + handlers + auth + fhir + version). |
| Bridge | `cargo test --test duplicate_detection` | 14 tests pinning the service↔canonical-matcher contract. |
| Integration | `cargo test --test api_integration_test -- --ignored` | 12 `#[ignore]`-tagged tests driving `tower::oneshot` against the hand-built Axum router (`create_router`, retained for tests; the binary serves the same surface via loco controllers) with real Postgres + Tantivy. Requires `DATABASE_URL` against a migrated DB (see `docker-compose.yml`'s `postgres` service). |
| Auth activation | `cargo test --test enforcement -- --ignored` | 1 `#[ignore]`-tagged test proving `COURSE_REQUIRE_AUTH` actually guards the real router. Requires `DATABASE_URL`. |
| Fluvio round-trip | `cargo test --features fluvio --test fluvio_relay -- --ignored` | 1 feature-gated + `#[ignore]`-tagged test. Requires `DATABASE_URL` and a live Fluvio broker. |
| Benchmarks | `cargo bench` | matching + search + validation (Criterion). |

See [`agents/testing.md`](../agents/testing.md) for the full layout.

