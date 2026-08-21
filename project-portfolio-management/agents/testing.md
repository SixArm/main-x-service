# Testing Strategy — Portfolio Entity

Entity-level view; normative inventory in entity spec
[§11](../spec/11-testing-strategy.md). Per-crate detail: matcher
[agents/testing.md](../project-portfolio-management-matcher-rust-crate/agents/testing.md).

## What exists today

| Subproject | Suite | Command | Covers |
|---|---|---|---|
| matcher | Unit tests (`#[cfg(test)]` per module) | `cargo test` | Components, normalisation, deterministic rules, kind-agnostic scoring, rank/find_matches |
| matcher | Public-API integration | `cargo test --test public_api` | The `lib.rs` re-export surface |
| matcher | Doctests | `cargo test` | rustdoc examples |
| service | DB-free embedding tests | `cargo test --test matching` | Matcher embedding + `Plan` JSON round-trip (the DTO/JSONB contract) |
| service | Controller validation units | `cargo test --lib` | Blank-name → `422` pin; containment-cycle → `422` (DB-free) |
| service | Request-level tests (`tests/requests/`) | `cargo test -- --ignored` + Postgres `DATABASE_URL` | Plan CRUD, 422 on blank name + containment cycle (create/update), get 200/404, list, `/match`, `/check-duplicates`, `/merge`; sub-resource CRUD; timeline / burndown reads. `#[ignore]`-gated so default `cargo test` is DB-free |
| front-end | Type-level check | `pnpm run check` | svelte-check strict, 0 errors / 0 warnings |
| front-end | Build | `pnpm run build` | Production build |

## Quality gates (all PRs)

```bash
# both Rust crates
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check

# front-end
pnpm run check
pnpm run build
```

Three-part PR discipline: a behavioural change carries its spec edit
and its test edit in the same PR.

## What is missing (tracked in entity spec §13)

| Gap | Task |
|---|---|
| Request-level suite is `#[ignore]`-gated; no DB-backed run wired into CI yet | T-4 follow-up |
| Front-end vitest units (`ApiClient`, `PlanRepository`) | T-5 |
| Playwright smoke over the core routes against a running service | T-5 |
| Derived-view (timeline / burndown) computation tests | T-6 |

## Contract seams worth pinning

- **Service ↔ matcher.** No adapter exists, so the contract is just
  serde round-tripping plus engine behaviour — `tests/matching.rs`
  pins both. If an adapter ever appears, add a bridge suite.
- **Kind-agnostic matching.** Matching never consults `kind`; pin it
  directly in the matcher (two plans with differing `kind` labels can
  still match) and confirm `check-duplicates` scores across labels.
- **Containment cycles.** A `parent_ref` that points a plan at itself or a
  descendant must return `422`; pin this at the service.
- **Front-end ↔ service.** Types are hand-mirrored; only an e2e smoke
  (T-5) catches drift. Until then, treat any matcher-type change as a
  mandatory `types.ts` review.
- **Plan ↔ sub-resources.** Cascade / soft-delete behaviour and the
  goal-title → match-component coupling (the goals bridge) are worth
  explicit request-level coverage.

## Writing new tests

- Matcher: keep tests deterministic and IO-free, mirroring the library's
  own rules (no clocks, no RNG seeds that matter). Note the `Timeframe`
  component uses dates — pin fixed dates, never `now()`. Include at least
  one pair with differing `kind` labels to pin that matching is
  kind-agnostic.
- Service: prefer DB-free tests where the behaviour allows; request-level
  tests are `#[ignore]`-gated (need Postgres — run with
  `cargo test -- --ignored` and a `DATABASE_URL`).
- Construct minimal records with `Plan::new("…")` (kind defaults to
  `None`) and set only the fields under test — renormalisation means
  absent components simply drop out of the score.
