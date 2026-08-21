## 11. Testing Strategy

Entity-level summary; detail in [`agents/testing.md`](../agents/testing.md).

### 11.1 Per subproject

| Subproject | Layer | Today |
|---|---|---|
| matcher | Unit tests embedded per module (`#[cfg(test)]`); integration suite `tests/public_api.rs` over the re-exported surface; rustdoc examples as doctests | Delivered |
| service | DB-free tests in [`tests/matching.rs`](../case-service-with-loco/tests/matching.rs): matcher embedding + JSON round-trip of the DTO | Delivered |
| service | Controller + module unit tests (`src/`): blank-title → `422` pin, validation cases, `merge`, `streaming` publish/read-back, `auth` crypto, `openapi` well-formedness, `CHECK_DUPLICATES_SCAN_CAP` value — all DB-free | Delivered |
| service | Request-level integration tests against PostgreSQL ([`tests/requests/cases.rs`](../case-service-with-loco/tests/requests/cases.rs), loco testing harness): CRUD, 422s, 404, `/search`, `/match`, `/check-duplicates`, `/merge`, audit/events, `whoami`, OpenAPI/Swagger. `#[ignore]`-gated; run with `cargo test -- --ignored` and a Postgres URL | Delivered (gated) |
| front-end | `pnpm run check` (svelte-check strict, 0 errors / 0 warnings) + production build | Delivered |
| front-end | vitest units (`ApiClient`, `CaseRepository`, incl. a `check-duplicates` path regression) + Playwright smoke over the four routes (API-stubbed, runs on `vite preview`) | Delivered |

### 11.2 Cross-subproject contract tests

The integration contract this spec owns needs pinning at two seams:

- **Service ↔ matcher** — `tests/matching.rs` exercises the embedded
  engine and the JSONB round-trip, which is the whole contract (no
  adapter exists to drift). The request-level suite adds a
  check-duplicates round-trip (store a case, post a near-duplicate
  query, assert the ranked hit) and a merge round-trip.
- **Front-end ↔ service** — the TypeScript types in
  `src/lib/api/types.ts` are hand-mirrored; the Playwright smoke
  against a running (or stubbed) service is the guard, plus a vitest
  regression pinning the `check-duplicates` path.

### 11.3 Gates

Rust: `cargo test`, `cargo clippy --all-targets -- -D warnings`,
`cargo fmt --check` in both crates. Front-end: `pnpm run check`,
`pnpm run build`. A behavioural change PR MUST carry its test edit
(three-part PR discipline).
