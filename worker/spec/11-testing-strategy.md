## 11. Testing Strategy

Each subproject owns its own pyramid; the entity level owns the
**contract tests at the seams**.

### 11.1 Per-subproject (owned by each crate / project)

| Subproject | Suite | Command | Current count |
|---|---|---|---|
| service | unit (matching, search, validation, privacy, models) | `cargo test --lib` | 99 |
| service | API integration (requires PostgreSQL) | `cargo test --test api_integration_test` | 7 |
| service | benchmarks (matching, search, validation) | `cargo bench` | 3 suites |
| matcher | unit + integration + doctests | `cargo test` | see [matcher §18](../worker-matcher-rust-crate/spec/18-testing-strategy.md) |
| front-end | unit (Vitest, mocked fetch) | `pnpm test` | 8 |
| front-end | e2e smoke (Playwright, no live service) | `pnpm test:e2e` | 6 |

Guides: [service `AGENTS/testing.md`](../worker-service-rust-crate/AGENTS/testing.md),
[matcher `AGENTS/testing.md`](../worker-matcher-rust-crate/AGENTS/testing.md),
[front-end §11](../worker-front-end-with-svelte/spec/11-testing-strategy.md).

### 11.2 Seam 1 — service↔matcher bridge tests (exists)

[`tests/duplicate_detection.rs`](../worker-service-rust-crate/tests/duplicate_detection.rs)
(14 tests) is the contract suite for §5.3: it drives the service
domain model through `to_matcher_worker()` and asserts on
`MatchingEngine::match_workers` output, pinning **both** the
adapter's field-routing and the matcher's scoring. Coverage spans
identical/near-duplicate ordering, deterministic short-circuits
(shared `uk_nhs_number`, tax-ID → US SSN routing, passport books,
NPI fall-through, ODS permanent fall-through incl. the shared-ODS-code
negative pin), negative cases, per-field routing
pins, and sparse-record edge cases.

```bash
cd worker-service-rust-crate
cargo test --test duplicate_detection
```

Rule: a new adapter routing rule, a new matcher scoring component
surfaced by the service, or a contract regression each REQUIRE a new
bridge test (see the "when to add" list in
[service `AGENTS/testing.md`](../worker-service-rust-crate/AGENTS/testing.md)).

### 11.3 Seam 2 — front-end↔service contract (gap)

Today the front-end's unit tests mock `fetch` and its Playwright
suite asserts page shells render with the API down. **No automated
test exercises the full trio** (real front-end against a real service
against a real database). Tracked as T-3 in §13. Until it lands, the
manual operator walkthrough in the front-end's
[§14](../worker-front-end-with-svelte/spec/14-implementation-status.md)
is the only end-to-end check.

### 11.4 Entity-level acceptance rule

A change to the integration contract (adapter routing, REST envelope,
endpoint shape, wire types) is NOT done until:

1. the owning crate's suite passes,
2. the relevant seam suite passes (11.2 today; 11.3 once it exists),
3. this spec's §5/§9 reflect the new contract.
