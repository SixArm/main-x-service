## 11. Testing Strategy

Entity-level summary; detail in [`AGENTS/testing.md`](../AGENTS/testing.md).

### 11.1 Per subproject

| Subproject | Layer | Today |
|---|---|---|
| matcher | Unit tests embedded per module (`#[cfg(test)]`); integration suite `tests/public_api.rs` over the re-exported surface; rustdoc examples as doctests | Delivered |
| service | DB-free tests in [`tests/matching.rs`](../care-pathway-service-rust-crate/tests/matching.rs): matcher embedding + JSON round-trip of the DTO | Delivered |
| service | Controller validation unit tests (`src/controllers/care_pathways.rs`): blank-name → `422` pin, DB-free | Delivered |
| service | Request-level integration tests against PostgreSQL ([`tests/requests/care_pathways.rs`](../care-pathway-service-rust-crate/tests/requests/care_pathways.rs), loco testing harness): CRUD, 422s, 404, `/match`, `/check-duplicates`. `#[ignore]`-gated; run with `cargo test -- --ignored` and a Postgres URL | Delivered (gated) — §13 T-4 |
| front-end | `pnpm run check` (svelte-check strict, 0 errors / 0 warnings) + production build | Delivered |
| front-end | vitest unit tests (`ApiClient`, `CarePathwayRepository`) and Playwright smoke over the four routes | Deferred — §13 T-5 |

### 11.2 Cross-subproject contract tests

The integration contract this spec owns needs pinning at two seams:

- **Service ↔ matcher** — `tests/matching.rs` already exercises the
  embedded engine and the JSONB round-trip, which is the whole
  contract (no adapter exists to drift). The request-level suite adds
  a check-duplicates round-trip (store a pathway, post a
  near-duplicate query, assert the ranked hit).
- **Front-end ↔ service** — no automated contract test today; the
  TypeScript types in `src/lib/api/types.ts` are hand-mirrored.
  A Playwright smoke against a running service is the planned guard
  (§13 T-5).

### 11.3 Gates

Rust: `cargo test`, `cargo clippy --all-targets -- -D warnings`,
`cargo fmt --check` in both crates. Front-end: `pnpm run check`,
`pnpm run build`. A behavioural change PR MUST carry its test edit
(three-part PR discipline).
