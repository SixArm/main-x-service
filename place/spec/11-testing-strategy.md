## 11. Testing Strategy

Each subproject owns its own pyramid; the entity layer cares about
the **contract seams** between them.

### 11.1 Per-subproject (summary)

| Subproject | Suite | Reference |
|---|---|---|
| place-service | 104 unit + 67 integration + 16 Criterion benchmarks | [spec §11](../place-service-with-loco/spec/11-testing-strategy.md), [agents/testing.md](../place-service-with-loco/agents/testing.md) |
| place-matcher | unit + integration + property tests + doctests; `cargo test` must pass with clippy `-D warnings` clean | [agents/testing.md](../place-matcher-rust-crate/agents/testing.md) |
| place-front-end | 8 Vitest unit tests (API client + repository, mocked `fetch`) + 6 Playwright e2e smoke tests (no live service required) | [spec §11](../place-front-end-with-svelte/spec/11-testing-strategy.md), [agents/testing.md](../place-front-end-with-svelte/agents/testing.md) |

### 11.2 Contract seams (entity-owned)

**Service ↔ matcher — the bridge suite.**
[`tests/duplicate_detection.rs`](../place-service-with-loco/tests/duplicate_detection.rs)
(14 tests) drives service-side records through
`adapter::to_matcher_place` and asserts on
`MatchingEngine::match_places` output, pinning *both* the adapter's
field-routing rules ([§5.3](05-domain-model.md)) and the matcher's
scoring. A regression on either side fails here. Any new adapter
routing rule MUST add a bridge test in the same PR (FR-19).

**Front-end ↔ service — wire-type tests.** The front-end unit tests
pin the envelope handling and repository wrapping against the wire
format in `src/lib/api/types.ts`. They mock `fetch`; they do **not**
verify against a live service.

### 11.3 Gaps (entity level)

- **No live trio test.** Nothing today runs front-end → service →
  database end to end (front-end spec §14 lists "live integration:
  pending operator walkthrough"). Tracked as [§13](13-tasks.md) E-9.
- **Front-end verification pending.** `pnpm install` / `pnpm test`
  have not been run and verified per front-end spec §14 — E-8.
- **No contract test against OpenAPI.** The front-end types are
  hand-mirrored, not generated or schema-checked — see
  [§16](16-open-questions.md).
