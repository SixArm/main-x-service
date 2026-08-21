# Testing Guide — Organization Entity

Entity-level test map. Strategy and seam obligations: entity
[spec §11](../spec/11-testing-strategy.md). Matcher detail:
[`agents/testing.md`](../organization-matcher-rust-crate/agents/testing.md).

## Commands per subproject

```bash
# matcher (organization-matcher-rust-crate)
cargo test                                  # unit + tests/public_api.rs + doctests
cargo clippy --all-targets -- -D warnings
cargo fmt --check

# service (organization-service-with-loco)
cargo test                                  # DB-free: unit (validation 422 pin, OpenAPI, streaming) + tests/matching.rs
cargo test -- --ignored                     # request-level suite (tests/requests/organizations.rs); needs Postgres per config/test.yaml
cargo clippy --all-targets

# front-end (organization-front-end-with-svelte)
pnpm run check                              # svelte-check strict — 0 errors / 0 warnings expected
pnpm run build                              # production build must succeed
# vitest / playwright: none yet (front-end spec §13)
```

## What exists today

| Layer | File(s) | Pins |
|---|---|---|
| Matcher unit tests | `#[cfg(test)]` modules in `src/*.rs` | Component scores, deterministic rules R-0/R-1/R-2, normalisation, presets |
| Matcher integration | `tests/public_api.rs` | The re-exported SemVer surface |
| Matcher doctests | rustdoc examples | Usage snippets stay correct |
| Service ↔ matcher seam | [`tests/matching.rs`](../organization-service-with-loco/tests/matching.rs) | (1) shared LEI fires R-0 through the embedded engine; (2) `Organization` serde round-trip — the exact contract the JSONB `data` column relies on |
| Service validation (DB-free) | `#[cfg(test)]` in [`src/controllers/organizations.rs`](../organization-service-with-loco/src/controllers/organizations.rs) | Blank `name` → `422 Unprocessable Entity` (T-2 pin) |
| Service REST behaviour | [`tests/requests/organizations.rs`](../organization-service-with-loco/tests/requests/organizations.rs) (Postgres, `#[ignore]`-gated) | Create round-trip (snake_case wire), `422` blank name on create + update, `404` unknown pid, search + blank-`q` `400`, check-duplicates ranking |
| Front-end | svelte-check + build | Type-level conformance of the TS mirror and routes |

## What to add when you change things

| You changed… | Add / update tests in… |
|---|---|
| Matcher algorithm / weights / normalisation | Matcher unit tests (same module) + `tests/public_api.rs` if the surface moved |
| The `Organization` DTO (fields, serde) | Matcher tests + the service round-trip test + front-end `types.ts` (and its future vitest suite) |
| Service endpoint behaviour | Request-level suite (`tests/requests/organizations.rs`) — keep it `#[ignore]`-gated and `#[serial]` |
| Front-end form / routes | svelte-check must stay 0/0; future vitest/playwright |

## Known gaps (queued)

- No front-end unit/e2e tests (front-end spec §13, entity T-11).
- No audit-endpoint request coverage yet (grows with T-9).
- No benchmarks; performance targets in entity §7 are unmeasured.
