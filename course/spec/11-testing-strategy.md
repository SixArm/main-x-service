## 11. Testing Strategy

Each subproject tests its internals; the **bridge test** is the
entity-level pin that the trio composes correctly.

| Layer | Owner | Tool / command | Scope |
|---|---|---|---|
| Matcher unit | M | `cargo test` (in the matcher crate) | Deterministic short-circuits, probabilistic ordering, normalisation, config presets |
| Service unit | S | `cargo test --lib` | 35 tests across db / matching / matching::adapter / search / validation / streaming / privacy / handlers |
| **Bridge** | S (pins S↔M) | `cargo test --test duplicate_detection` | 14 tests pinning the adapter routing (§5.3) and the canonical matcher contract: identical / typo / deterministic short-circuits / negatives / enum routing / config presets |
| Service integration | S | `cargo test --test api_integration_test -- --ignored` | 12 `#[ignore]`-tagged tests over the full router with real Postgres + Tantivy (lifecycle, 409/422, search, match, merge, dedup, instances, audit, masked, export) |
| Service bench | S | `cargo bench` | 3 Criterion suites: matching, search, validation |
| Front-end unit | F | `pnpm test` (Vitest) | 9 tests: `ApiClient` envelope + error handling, `CourseRepository` wrapping — pins the wire contract from the consumer side |
| Front-end e2e | F | `pnpm test:e2e` (Playwright) | 5 smoke tests: every MVP route shell renders even with the API down |
| Type check | F | `pnpm check` | svelte-check, TypeScript strict |

### 11.1 Entity-level rules

- **Cross-subproject changes require cross-subproject tests.** A
  matcher behaviour change MUST update the service bridge test in
  the same change cycle (matcher AGENTS golden rule); a service
  wire-format change MUST update the front-end unit tests.
- A new deterministic identifier scheme without a bridge test is the
  worst-case bug (false positive at score 1.0) — reviewers reject it.
- There is no end-to-end harness driving front-end → live service
  today; the front-end's live-integration walkthrough is pending
  (front-end spec §14) — tracked in §13.

Per-subproject layouts: service
[agents/testing.md](../course-service-with-loco/agents/testing.md),
matcher [agents/testing.md](../course-matcher-rust-crate/agents/testing.md),
front-end [agents/testing.md](../course-front-end-with-svelte/agents/testing.md).
