## 11. Testing Strategy

Entity-level summary; detail in [`AGENTS/testing.md`](../AGENTS/testing.md).
The entity is **spec-only; no code exists yet** (§14), so every row
below is **planned** — the table records the intended layers, not
delivered suites.

### 11.1 Per subproject (planned)

| Subproject | Layer | Plan |
|---|---|---|
| matcher | Unit tests embedded per module (`#[cfg(test)]`); integration suite `tests/public_api.rs` over the re-exported surface; rustdoc examples as doctests | Build T-2 |
| matcher | Kind-gate tests (different `kind` → 0.0, never reaching any other rule), deterministic-rule tests (R-0 each scheme, R-1 owner+code, R-2 `same_as`) and probabilistic-component tests (name, goal-title Jaccard, code, owner org, parent portfolio, timeframe, keywords, relationships, tags); renormalisation; presets | Build T-2 |
| service | DB-free tests: matcher embedding + JSON round-trip of the thin DTO (incl. `kind`) | Build T-3 |
| service | Controller validation unit tests: blank-name → `422`, `kind`/collection mismatch → `422`, malformed `EntityRef` / `portfolio_ref` / deterministic identifier / relationship / `in_language` → `422` (DB-free) | Build T-3 |
| service | Request-level integration tests against PostgreSQL (loco harness): per-collection CRUD, 422s, 404, `/match`, `/check-duplicates`, `409` on create, merge with sub-resource re-homing; sub-resource CRUD; derived timeline / burndown; the portfolio roll-up filter; `#[ignore]`-gated, run with a Postgres URL | Build T-4 |
| service | The goals bridge: a goal sub-resource write is visible in `data.goals[]` and in a subsequent match (regression for §5.3) | Build T-4 |
| front-end | `pnpm run check` (svelte-check strict, 0/0) + production build | Build T-6 |
| front-end | vitest units (`ApiClient`, `WorkItemRepository` per collection, sub-resource repositories) + Playwright smoke over the routes | Build T-6 |

### 11.2 Cross-subproject contract tests (planned)

The integration contract this spec owns needs pinning at four seams:

- **Service ↔ matcher** — a matcher-embedding test plus the JSONB
  round-trip is the whole contract for the thin record (no adapter
  exists to drift). The request-level suite adds a check-duplicates
  round-trip (store a work item, post a near-duplicate query of the
  same kind, assert the ranked hit) and a `409`-on-create assertion.
- **The kind gate** — a test asserts that a cross-collection
  comparison (and a payload `kind` that disagrees with the collection)
  never yields a positive match: the matcher returns 0.0 and the
  service feeds only same-kind candidates.
- **The partition** — a test asserts that no sub-resource field ever
  appears in any `data` column and that a sub-resource write never
  changes the match score (except a goal write, which is the deliberate
  crossover — §5.3 / §5.6).
- **Front-end ↔ service** — the TypeScript types in
  `src/lib/api/types.ts` are hand-mirrored; a Playwright smoke against
  a running service is the planned guard.

### 11.3 Gates

Rust: `cargo test`, `cargo clippy --all-targets -- -D warnings`,
`cargo fmt --check` in both crates. Front-end: `pnpm run check`,
`pnpm run build`. A behavioural change PR MUST carry its test edit
(three-part PR discipline).
