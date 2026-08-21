## 11. Testing Strategy

Entity-level rule: **the cross-subproject contract must be pinned by
tests on both sides of each seam.** Per-subproject detail:
[`agents/testing.md`](../agents/testing.md).

### 11.1 Current inventory

| Subproject | Suite | What it covers |
|---|---|---|
| matcher | unit tests per module + [`tests/public_api.rs`](../organization-matcher-rust-crate/tests/) + doctests | Component scores, deterministic rules, normalisation, config presets, the re-exported public surface |
| service | [`tests/matching.rs`](../organization-service-with-loco/tests/matching.rs) (DB-free, 2 tests) + `src/` unit tests (validation `422` pin, OpenAPI shape, streaming) | (a) the embedded matcher fires R-0 on a shared LEI; (b) the `Organization` JSON round-trip the JSONB storage relies on |
| service | [`tests/requests/organizations.rs`](../organization-service-with-loco/tests/requests/organizations.rs) (Postgres, `#[ignore]`-gated, 6 tests) | Create round-trip (snake_case wire), blank-name `422` on create + update, unknown-pid `404`, search + blank-`q` `400`, check-duplicates ranking |
| front-end | `pnpm run check` (svelte-check strict, 0 errors / 0 warnings) + production build | Type-level conformance of the TS mirror and routes |

Commands:

```bash
# matcher
cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check
# service (DB-free; the request suite is #[ignore]-gated)
cargo test && cargo clippy --all-targets
# service request-level suite (needs Postgres per config/test.yaml)
cargo test -- --ignored
# front-end
pnpm run check && pnpm run build
```

### 11.2 Seam coverage (the entity-level concern)

| Seam | Pinned by | Status |
|---|---|---|
| service ↔ matcher (DTO + engine) | service `tests/matching.rs` | ✔ minimal |
| service ↔ database (JSONB round-trip) | service `tests/matching.rs` round-trip test + the create round-trip request test | ✔ serde + live-Postgres (gated) |
| front-end ↔ service (wire shapes) | TypeScript types only — no contract tests | ✘ gap |
| REST behaviour (status codes, caps, search) | `tests/requests/organizations.rs` (`422` / `404` / `400`, search, check-duplicates ranking) | ✔ gated on Postgres |

### 11.3 Gaps → tasks (§13)

- ~~Request-level integration tests against Postgres~~ — done (T-4,
  2026-06-13): `tests/requests/organizations.rs`, `#[ignore]`-gated;
  `rstest` / `insta` dev-deps remain available for parametrised /
  snapshot growth.
- Front-end vitest unit tests (`ApiClient`, `OrganizationRepository`)
  and a Playwright smoke across the four routes (front-end spec §13).
- Audit-endpoint request coverage (grows with T-9 actor wiring).
- Benchmarks (Criterion) once §15 performance targets are measured.
