# Testing — Event Entity

Each subproject owns its pyramid; the entity owns the **seam tests**.
Strategy rationale: entity spec
[§11](../spec/11-testing-strategy.md).

## Run everything

```bash
# Matcher (pure — fast, no setup)
(cd event-matcher-rust-crate && cargo test && cargo clippy --all-targets -- -D warnings)

# Service
(cd event-service-rust-crate && cargo test --lib)                          # 62+ unit tests
(cd event-service-rust-crate && cargo test --test duplicate_detection)     # 16 seam tests
(cd event-service-rust-crate && DATABASE_URL=… cargo test --test api_integration_test)
(cd event-service-rust-crate && cargo bench)                               # Criterion

# Front-end
(cd event-front-end-with-svelte && pnpm test && pnpm test:e2e && pnpm check)
```

## The seam tests (entity-critical)

| Seam | Test | Pins |
|---|---|---|
| Service ↔ matcher | [`tests/duplicate_detection.rs`](../event-service-rust-crate/tests/duplicate_detection.rs) | Adapter field-routing **and** matcher scoring — both sides of the §5.3 DTO contract |
| Front-end ↔ service | `tests/unit/client.test.ts`, `tests/unit/events.test.ts` (mocked fetch) | Envelope handling + endpoint wrapping against the documented wire format |
| Front-end resilience | `tests/e2e/events.spec.ts` (Playwright, no live service) | Page shells render with the API down |

Rule: a PR that changes the integration contract (adapter, wire
format, shared invariant) must change a seam test in the same PR.

## Known gaps (tracked)

- No live cross-process front-end ↔ service integration run yet
  (entity ET-7; front-end §14 records install/test unverified).
- Service integration tests cover only health / create round-trip /
  validation; dedup, merge, and privacy workflows are service T-5.
- No load test at governmental volumes (entity ET-9).

## Per-subproject guides

- Service: [`AGENTS/testing.md`](../event-service-rust-crate/AGENTS/testing.md)
  — layers, factories, tempdir search index, CI workflows, when to
  add a bridge test.
- Matcher: [`AGENTS/testing.md`](../event-matcher-rust-crate/AGENTS/testing.md)
  — test pyramid, property tests, doctest hygiene; **no real
  personal data in fixtures** (see
  [security-and-privacy](../event-matcher-rust-crate/AGENTS/security-and-privacy.md)).
- Front-end: [`AGENTS/testing.md`](../event-front-end-with-svelte/AGENTS/testing.md)
  — Vitest + Playwright layout.

## Entity-wide fixture rules

- Synthetic parties only: RFC 2606 `example.org` emails, drama
  phone ranges.
- Explicit UTC instants in time fixtures; never wall-clock "now".
