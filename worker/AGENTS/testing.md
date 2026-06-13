# Testing — Worker entity

Entity-level orientation: each subproject owns its pyramid; the
entity owns the seams. Normative strategy:
[entity spec §11](../spec/11-testing-strategy.md).

## Per-subproject suites

| Where | What | Command |
|---|---|---|
| service | 99 unit tests (matching, search, validation, privacy, models) | `cargo test --lib` |
| service | 7 API integration tests (needs PostgreSQL) | `cargo test --test api_integration_test` |
| service | 3 Criterion benchmark suites | `cargo bench` |
| matcher | unit + integration + doctests; clippy `-D warnings` gate | `cargo test` |
| front-end | 8 Vitest unit tests (mocked fetch) | `pnpm test` |
| front-end | 6 Playwright smoke tests (no live service) | `pnpm test:e2e` |

Guides:
[service `AGENTS/testing.md`](../worker-service-rust-crate/AGENTS/testing.md) ·
[matcher `AGENTS/testing.md`](../worker-matcher-rust-crate/AGENTS/testing.md) ·
[front-end spec §11](../worker-front-end-with-svelte/spec/11-testing-strategy.md).

## Seam tests — the entity's own concern

**Service↔matcher (exists):**
[`tests/duplicate_detection.rs`](../worker-service-rust-crate/tests/duplicate_detection.rs)
— 13 black-box tests pinning both the adapter's field routing and the
matcher's scoring.

```bash
cd worker-service-rust-crate
cargo test --test duplicate_detection
```

Add a bridge test whenever the adapter gains a routing rule, the
matcher exposes a new component the service surfaces, or a contract
regression escapes the unit layers.

**Front-end↔service (gap):** no automated full-trio test yet —
tracked as [entity spec §13 T-3](../spec/13-tasks.md).

## Rules of thumb

- A contract change (adapter routing, envelope, endpoint shape) is
  not done until the owning crate's suite **and** the seam suite pass,
  and entity spec §5/§9 match (entity §11.4).
- Matcher tests use synthetic fixtures only — no real PII, ever
  ([matcher `AGENTS.md`](../worker-matcher-rust-crate/AGENTS.md)
  golden rule 7).
- Don't pin `NicknameTable::english()` contents in tests; construct
  your own `with_class` tables.
- Front-end e2e suites must keep passing with the API down (degraded
  shell render is itself a requirement, entity NFR-14).
