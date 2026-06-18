# Testing Strategy — Case Entity

Entity-level view; normative inventory in entity spec
[§11](../spec/11-testing-strategy.md). Per-crate detail: matcher
[AGENTS/testing.md](../case-matcher-rust-crate/AGENTS/testing.md).

## What exists today

| Subproject | Suite | Command | Covers |
|---|---|---|---|
| matcher | Unit tests (`#[cfg(test)]` per module) | `cargo test` | Components, normalisation, deterministic rules, rank/find_matches |
| matcher | Public-API integration | `cargo test --test public_api` | The `lib.rs` re-export surface |
| matcher | Doctests | `cargo test` | rustdoc examples |
| service | DB-free embedding tests | `cargo test --test matching` | Matcher embedding + `Case` JSON round-trip (the DTO/JSONB contract) |
| service | Module unit tests | `cargo test --lib` | Blank-title → `422` pin, validation cases, `merge`, `streaming` publish/read-back, `auth` crypto, `openapi` well-formedness, `CHECK_DUPLICATES_SCAN_CAP` value — all DB-free |
| service | Request-level tests (`tests/requests/cases.rs`) | `cargo test -- --ignored` + Postgres `DATABASE_URL` | CRUD, 422s, 404, `/search`, `/match`, `/check-duplicates` round-trip, `/merge`, audit/events, `whoami` 401, OpenAPI/Swagger. `#[ignore]`-gated so default `cargo test` is DB-free |
| front-end | Type-level check | `pnpm run check` | svelte-check strict, 0 errors / 0 warnings |
| front-end | Build | `pnpm run build` | Production build |
| front-end | vitest units | `pnpm test` | `ApiClient` + `CaseRepository` (incl. `check-duplicates` path regression) |
| front-end | Playwright smoke | `pnpm test:e2e` | The four routes, API-stubbed, against `vite preview` |

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

Three-part PR discipline: a behavioural change carries its spec edit and
its test edit in the same PR.

## What is missing (tracked in entity spec §13)

| Gap | Task |
|---|---|
| Request-level suite is `#[ignore]`-gated; no DB-backed run wired into CI yet | T-4 follow-up |
| No tests for privacy controls — none exist yet (masking / GDPR export) | T-10 |
| No front-end tests for a search box / audit views — those UIs don't exist yet | T-11 |

## Contract seams worth pinning

- **Service ↔ matcher.** No adapter exists, so the contract is just
  serde round-tripping plus engine behaviour — `tests/matching.rs` pins
  both. If an adapter ever appears, add a bridge suite.
- **Front-end ↔ service.** Types are hand-mirrored; the Playwright smoke
  plus the vitest `check-duplicates` path regression catch drift. Treat
  any matcher-type change as a mandatory `types.ts` review.

## Writing new tests

- Matcher: keep tests deterministic and IO-free, mirroring the
  library's own rules (no clocks, no RNG seeds that matter).
- Service: prefer DB-free tests where the behaviour allows; the
  request-level tests are `#[ignore]`-gated (need Postgres — run with
  `cargo test -- --ignored` and a `DATABASE_URL`). Auth/crypto tests
  mint a token + matching public key in-process, so they stay DB-free.
  (Auth pivot: the target is a PASETO v4.public token + published Ed25519
  key, replacing RS256 JWT + JWKS — see
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md);
  the current fixtures still mint the old credential, code follow-up
  tracked in the service spec §13.)
- Construct minimal records with `Case::new("…")` and set only the
  fields under test — renormalisation means absent components simply
  drop out of the score.
