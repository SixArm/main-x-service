## 11. Testing Strategy

Each subproject owns its own test pyramid; the entity level owns the
**contract tests** that pin the seams between them.

### 11.1 Seam 1 — service ↔ matcher: bridge tests

[`tests/duplicate_detection.rs`](../event-service-with-loco/tests/duplicate_detection.rs)
in the service (16 tests) drives service-side records through
`adapter::to_matcher_event` and asserts on
`MatchingEngine::match_events` output. It pins **both sides** — the
adapter's field-routing rules and the matcher's scoring — so a
regression on either side fails here. Any change to the §5.3 DTO
contract MUST land with a bridge-test change in the same PR.

Run: `cargo test --test duplicate_detection` (in the service crate).

### 11.2 Seam 2 — front-end ↔ service: typed client tests

The front-end pins the wire format with Vitest unit tests against a
mocked `fetch` (`tests/unit/client.test.ts`, `tests/unit/events.test.ts`
— 8 tests) and Playwright e2e smoke tests (6 tests, no live service
required). There is **no live cross-process integration test** of
front-end against a running service yet — tracked as ET-7.

### 11.3 Per-subproject pyramids (owned by each spec)

| Subproject | Layers | Reference |
|---|---|---|
| Service | 62+ unit tests; integration tests against real PostgreSQL + Tantivy; 16 bridge tests; Criterion benches; CI (`test.yml`, `quality.yml`, `security.yml`) | [agents/testing.md](../event-service-with-loco/agents/testing.md) |
| Matcher | Unit + integration + property tests + doctests; `cargo clippy -D warnings` clean; benches | [agents/testing.md](../event-matcher-rust-crate/agents/testing.md) |
| Front-end | Vitest unit (mocked fetch) + Playwright e2e smoke; `svelte-check` | [agents/testing.md](../event-front-end-with-svelte/agents/testing.md) |

### 11.4 Entity-level rules

- A three-part PR that changes the integration contract touches the
  seam test, not just the owner's unit tests.
- No real personal data in any test fixture, in any subproject —
  synthetic parties only (RFC 2606 `example.org` emails, drama
  phone ranges). See matcher
  [agents/security-and-privacy.md](../event-matcher-rust-crate/agents/security-and-privacy.md).
- Time fixtures use explicit UTC instants; tests that depend on
  wall-clock "now" are forbidden (the matcher forbids clocks
  outright).
