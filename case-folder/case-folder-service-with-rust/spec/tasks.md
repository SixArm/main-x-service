# Tasks (Loco edition delivery)

> Part of the [Loco edition specification](index.md). Cross-edition
> board: [root tasks](../../spec/tasks.md). Tasks trace to
> [requirements.md](requirements.md) (AR-) and [design.md](design.md) (LD-).

## Status legend

- [x] done · [~] in progress · [ ] not started

## Delivered (in repo)

- [x] **LT-1** Five `Client` traits + `Http` + `Stub` impls + RoutingClient (LD-1) — AR-8, AR-9
- [x] **LT-2** Typed `responses::` structs + 404/422/503 helpers (LD-2) — AR-4
- [x] **LT-3** Controllers: healthz, stats, folders, moves, patients, places, workers (LD-2) — AR-1
- [x] **LT-4** `POST /api/moves` with worker snapshot + free-text fallback (LD-3) — AR-2
- [x] **LT-5** `POST /api/folders` find-or-create patient + initial event (LD-3) — AR-3
- [x] **LT-6** `nhs.rs` Modulus 11 validator + 8 unit tests (—) — AR-5.
  Tests cover the full worked-example table in [nhs-number.md](../../spec/nhs-number.md),
  including the `check == 10 → invalid` branch and grouped/bare-form parity.
- [x] **LT-7** Soft-fail stats + patient lookup; 503 elsewhere (LD-6) — AR-6, AR-7
- [x] **LT-8** Stub mode + seed task (3 buildings / 4 rooms / 5 cabinets / 6 patients / 9 folders) (LD-5) — AR-8
- [x] **LT-9** 29 request tests against stub upstreams (LD-1) — AR-8
- [x] **LT-10** Vestigial Postgres / empty Migrator (LD-4) — AR-9

## Active / next

- [x] **LT-16** Extract the geofence-breach derivation behind `GET /api/alerts`
  into a pure `detect_geofence_breaches` (+ `cabinet_buildings`) function and
  unit-test every branch of the boundary-crossing rule (D-12). Previously the
  logic was reachable only through Postgres-gated request tests; it now has 6
  in-crate `#[cfg(test)]` tests (cross-building breach, same-building
  suppression, `None`-endpoint in-transit/created-in-place, unknown cabinet,
  orphan-room hierarchy, mixed-log filtering). No behaviour change. Lib tests
  8→14.
- [x] **LT-11** OpenAPI / JSON Schema document for every endpoint (P1) — AR-4.
  *(2026-09-03)* [`openapi.yaml`](../openapi.yaml) already existed and was
  substantially complete, but the checkbox was never ticked; audited it
  against the live route table rather than assuming it was current.
  Verified all 29 route operations across the 10 controllers (matched
  method-by-method against every `routes()` fn in `src/controllers/`)
  are present with request/response schemas, and that every response
  struct in `src/responses/mod.rs` (`Patient`, `Place`, `Cabinet`,
  `Folder`, `Volume`, `Move`, `Worker`, the `List<T>` envelope,
  `ErrorBody`, `ValidationBody`) has a matching component schema. Found
  and fixed one real drift while auditing: `POST /api/auth/logout` had
  no `security: []` override, so it inherited the document's top-level
  `security` requirement — but the handler (`src/controllers/auth.rs`)
  never checks for a session and always returns `204` (its own doc
  comment: "logout is idempotent (clearing an absent session is a
  no-op)"), matching the middleware's public-path bypass for
  `/api/auth/*` (`src/initializers/auth.rs`). The document now declares
  it public, same as `/healthz` and the other two pre-session auth
  routes. No route lacks a schema; the soft-fail 200-always endpoints
  (`GET /api/stats`, `GET /api/patients/{nhs}`) were checked against
  their handlers and correctly carry no `503` response.
- [ ] **LT-12** SSE on `/api/moves` (P1)
- [ ] **LT-13** Soft-delete cabinets, refusing while occupied (P2)
- [ ] **LT-14** CSV / FHIR export of the audit log (P2)
- [ ] **LT-15** NHS Spine PDS lookup on folder registration (P3)

- [ ] **LT-17** Nested `.github/workflows/{quality,security}.yml` in this
  crate are wired into neither `.github/workflows/` nor `.woodpecker.yml`
  at the repo root *(verified: `grep -rl case-folder ../../.github/workflows/
  ../../.woodpecker.yml` finds nothing)* — root CI already runs
  `fmt`/`clippy`/`test`/`deny` against this crate via
  `scripts/ci-crates.sh`'s generic Cargo.toml discovery, so these files
  never execute and only look like coverage. Cross-referenced at the
  cross-edition level as root **T-17** (same finding, decide once).
  **Acceptance:** delete the two files, or add a one-line comment at the
  top of each explaining why they're kept as a deliberate local-only
  convenience despite never running in CI.
- [x] **LT-18** `openapi.yaml`'s `bearerAuth` scheme declares
  `bearerFormat: JWT`, but `src/auth/mod.rs::bearer_token` /
  `identity_from_headers` treat the bearer value as the raw opaque
  session id, never as a decoded JWT *(verified by reading both files —
  the only real JWT in this service is the short-lived magic-link token,
  which is accepted only at `POST /api/auth/verify`, never as a bearer
  credential elsewhere)*. Cross-referenced as root **T-18**.
  **Acceptance:** `bearerFormat` is corrected or removed, matching the
  actual credential shape. *(resolved 2026-09-05.)*
  - **Resolved.** Removed the misdeclared `bearerFormat: JWT` line from
    `openapi.yaml`'s `bearerAuth` scheme and added a comment explaining
    the credential is the same opaque session id `sessionCookie`
    carries. No code change — this was a docs-only correctness fix
    (`openapi.yaml` is a static reference doc, not loaded or served by
    any code path). YAML re-validated (`python3 -c "import yaml;
    yaml.safe_load(open('openapi.yaml'))"`) after the edit.
- [x] **LT-19** *(resolved 2026-09-05.)* [`testing.md`](testing.md)'s
  unit-test inventory (`src/nhs.rs` 8 + `src/controllers/alerts.rs` 6 =
  14, and `cargo test` passes are described only in those terms) had
  drifted since **T-AUTH** landed: `src/auth/mod.rs` carries 7
  `#[test]`s and `src/auth/crypto.rs` carries 2 more, neither module
  mentioned in the doc — *(verified: `grep -rn '#\[test\]' src |
  xargs -I{} …` counts 23 unit tests total in `src/`, against the 14
  the doc enumerated)*. The same "doc says a count, code has moved
  past it" pattern **LT-16** fixed once for `testing.md`/`README.md`'s
  request-test count. **Resolution:** `testing.md`'s summary table
  and `README.md`'s `cargo test` comment both corrected from `21` to
  `23` (the table had already grown a "`src/auth/mod.rs` — session
  lifecycle" section for 7 of the 9 auth tests since this task was
  written, but was still one module and 2 tests short); added the
  missing "`src/auth/crypto.rs` — HS256 magic-link token signing (2
  tests)" section naming `hs256_signs_and_verifies_round_trip` and
  `hs256_rejects_a_wrong_secret`. Verified: `cargo test --lib` — 23
  passed, 0 failed, matching the corrected count exactly; `cargo fmt
  --check` / `cargo clippy --all-targets -- -D warnings` clean
  (doc-only change, but rechecked per the three-part discipline).

## Production gates

- [ ] **LT-G1** Auth (CIS2 / OIDC) + ABAC via Loco auth layer (P0)
- [ ] **LT-G2** Append-only chained-signature audit storage on a worker (P0)
- [ ] **LT-G3** API versioning via `Accept` mediatype (P0)
- [ ] **LT-G4** `auto_migrate: false`, no dangerous truncate/recreate, `--release`, secrets manager, reverse-proxy TLS/HSTS — see [regulatory.md](regulatory.md)

## Recipe: add an endpoint

1. Add route + method to [routes.md](routes.md); add a use case to
   [root requirements](../../spec/requirements.md) if new.
2. Add a `Serialize` struct in `src/responses/mod.rs` if a new shape.
3. Create/extend the controller; register it in `mod.rs` + `app.rs`.
4. Add request tests in `tests/requests/<resource>.rs` (happy + one failure).
5. Run the CI gates in [testing.md](testing.md).
