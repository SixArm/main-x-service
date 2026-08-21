# Testing Strategy — Care Pathway Entity

Entity-level view; normative inventory in entity spec
[§11](../spec/11-testing-strategy.md). Per-crate detail: matcher
[agents/testing.md](../care-pathway-matcher-rust-crate/agents/testing.md).

## What exists today

| Subproject | Suite | Command | Covers |
|---|---|---|---|
| matcher | Unit tests (`#[cfg(test)]` per module) | `cargo test` | Components, normalisation, deterministic rules, rank/find_matches |
| matcher | Public-API integration | `cargo test --test public_api` | The `lib.rs` re-export surface |
| matcher | Doctests | `cargo test` | rustdoc examples |
| service | DB-free embedding tests | `cargo test --test matching` | Matcher embedding + `CarePathway` JSON round-trip (the DTO/JSONB contract) |
| service | Controller validation units | `cargo test --lib` | Blank-name → `422` pin (DB-free) |
| service | Request-level tests (`tests/requests/care_pathways.rs`) | `cargo test -- --ignored` + Postgres `DATABASE_URL` | All seven endpoints: create, 422 on blank name (create/update), get 200/404, list, `/match`, `/check-duplicates` round-trip. `#[ignore]`-gated so default `cargo test` is DB-free |
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
| Front-end vitest units (`ApiClient`, `CarePathwayRepository`) | T-5 |
| Playwright smoke over the four routes against a running service | T-5 |

## Contract seams worth pinning

- **Service ↔ matcher.** No adapter exists, so the contract is just
  serde round-tripping plus engine behaviour — `tests/matching.rs`
  pins both. If an adapter ever appears, add a bridge suite like the
  person service's `tests/duplicate_detection.rs`.
- **Front-end ↔ service.** Types are hand-mirrored; only an e2e
  smoke (T-5) catches drift. Until then, treat any matcher-type
  change as a mandatory `types.ts` review.

## Writing new tests

- Matcher: keep tests deterministic and IO-free, mirroring the
  library's own rules (no clocks, no RNG seeds that matter).
- Service: prefer DB-free tests where the behaviour allows;
  request-level tests are `#[ignore]`-gated (need Postgres — run
  with `cargo test -- --ignored` and a `DATABASE_URL`).
- Construct minimal records with `CarePathway::new("…")` and set
  only the fields under test — renormalisation means absent
  components simply drop out of the score.
