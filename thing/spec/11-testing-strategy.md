## 11. Testing Strategy

Each subproject tests its internals; the entity contract is pinned by
the **bridge tests** and (pending) live end-to-end runs.

### 11.1 Per-subproject layers

| Subproject | Layers | Guide |
|---|---|---|
| service | ~100 unit tests (`cargo test --lib`), `tests/integration_*.rs`, Criterion benchmarks, CI (`test.yml`, `quality.yml`, `security.yml`) | [service `agents/testing.md`](../thing-service-with-loco/agents/testing.md) |
| matcher | Unit + integration + property tests + doctests (`cargo test`), Criterion benchmarks; clippy `-D warnings` and `cargo doc` clean as gates | [matcher `agents/testing.md`](../thing-matcher-rust-crate/agents/testing.md) |
| front-end | 8 Vitest unit tests (`client.test.ts`, `things.test.ts`), 6 Playwright e2e smoke tests | [front-end `agents/testing.md`](../thing-front-end-with-svelte/agents/testing.md) |

### 11.2 The integration-contract pin

[`thing-service-with-loco/tests/duplicate_detection.rs`](../thing-service-with-loco/tests/duplicate_detection.rs)
— 15 black-box bridge tests that drive service-shaped records through
`matching::adapter::to_matcher_thing` and assert on
`MatchingEngine::match_things` output. They pin **both sides** of the
DTO contract (§5.3): the adapter's field-routing rules and the
matcher's scoring. A regression on either side fails here.

```bash
cd thing-service-with-loco
cargo test --test duplicate_detection
```

When this spec's §5.3 table changes, a bridge test MUST change in the
same PR (three-part-PR rule).

### 11.3 End-to-end (front-end ↔ live service)

The Playwright suite runs against mocked responses today; a
live-service operator walkthrough is pending (front-end §14, entity
§13 T-5). Target: every front-end route exercised against a running
service seeded with canonical test data.

### 11.4 Test-data conventions

Well-known canonical things for readability — books (Pride and
Prejudice, War and Peace), software, papers — with real ISBNs / DOIs
where format validation matters. No real personal data anywhere in
the trio; the matcher additionally mandates synthetic fixtures
(RFC 2606 domains, drama-reserved phone ranges).
