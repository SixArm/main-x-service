# Testing — Course Entity

Orientation only. Per-subproject guides:
[`course-service/agents/testing.md`](../course-service-with-loco/agents/testing.md),
[`course-matcher/agents/testing.md`](../course-matcher-rust-crate/agents/testing.md),
[`course-front-end/agents/testing.md`](../course-front-end-with-svelte/agents/testing.md).
Normative strategy: entity spec [§11](../spec/11-testing-strategy.md).

## Layers across the trio

| Layer | Command (in the subproject dir) | Count today |
|---|---|---|
| Matcher unit | `cargo test` | full crate suite (short-circuits, ordering, normalisation) |
| Service unit | `cargo test --lib` | 35 |
| **Bridge (S↔M pin)** | `cargo test --test duplicate_detection` | 14 |
| Service integration | `cargo test --test api_integration_test -- --ignored` (needs migrated Postgres via `DATABASE_URL`) | 12 |
| Service bench | `cargo bench` | 3 Criterion suites |
| Front-end unit (F↔S pin) | `pnpm test` (Vitest, mocked `fetch`) | 9 |
| Front-end e2e | `pnpm test:e2e` (Playwright, API-down resilient) | 5 |
| Front-end types | `pnpm check` | — |

## What pins what

- **Bridge tests** (`course-service/tests/duplicate_detection.rs`)
  pin the adapter routing (entity spec §5.3) **and** the canonical
  matcher behaviour as the service consumes it. Touch the adapter, a
  matcher weight, or a deterministic scheme → touch these tests.
- **Front-end unit tests** pin the envelope and `CourseRepository`
  wrapping — the consumer-side view of the wire contract. Touch the
  service wire format → touch these tests.
- **Integration tests** pin the full HTTP surface against real
  Postgres + Tantivy; they are `#[ignore]`-tagged so plain
  `cargo test` stays hermetic.

## Entity-level gaps (tracked in entity spec §13)

- No end-to-end harness drives the real front-end against a live
  service; the Playwright suite is deliberately API-down smoke only.
- The front-end's live-integration operator walkthrough is pending
  (front-end spec §14).

## Before declaring success

Run the owning subproject's full local gate (fmt + clippy + tests for
Rust; `pnpm test` + `pnpm check` for the front-end), **plus** the
cross-subproject pin for any contract you touched.
