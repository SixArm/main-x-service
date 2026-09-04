# Tasks (delivery checklist)

> Part of the [Case Tracking specification](index.md). The living
> delivery board. Each task traces to a requirement in
> [requirements.md](requirements.md) and a decision in
> [design.md](design.md). Per-edition task lists live in each
> subproject's `spec/tasks.md`.

## Status legend

- [x] done · [~] in progress · [ ] not started

## Delivered (baseline — both editions in repo)

- [x] **T-1** Five upstream client interfaces + HTTP + stub impls (D-1) — FR-1..FR-9
- [x] **T-2** Snapshot-on-write for folders and move events (D-2) — NFR-2, NFR-3
- [x] **T-3** Derived folder status (D-3) — FR-9
- [x] **T-4** JSON API: folders, moves, places, patients, workers, stats, healthz (D-4) — FR-1..FR-9
- [x] **T-5** Modulus 11 validator on both editions (D-6) — NFR-1
- [x] **T-6** Stub mode + seed task (D-5) — NFR-7
- [x] **T-7** Resilience policy: 503 default + stats/patient soft-fail (D-8) — NFR-3
- [x] **T-8** SvelteKit reference client: dashboard, folders, patients, places, move, history (D-4)
- [x] **T-9** Loco request tests + Svelte Playwright e2e against stub mode (D-5) — NFR-7
- [x] **T-10** Accessibility baseline: skip link, single h1, labelled fields (NFR-5)

## Active / next

- [x] **T-AUTH** Email magic-link auth: stateless signed tokens, session
  cookie, `/api/auth/*` endpoints, `/api/*` guard, allowlist identity,
  log mailer, dev proxy (D-9) — FR-10..12, NFR-8, NFR-9
- [x] **T-CLICK** Click-through views (D-10): worker→folders, place→presence
  history, event→detail. New endpoints `GET /api/workers/{id}`,
  `GET /api/places/{id}/history`, `GET /api/moves/{id}`; new pages
  `/workers[/{id}]`, `/cabinets/{id}`, `/rooms/{id}`, `/history/{id}`.
  Folder→history and patient→folders already existed — FR-13, FR-14, FR-15
- [x] **T-VOL** Volumes (D-11): movable single-patient folder bundles stored
  as `thing_type=Volume` Things; folder gains `volume_id`. Endpoints
  `GET/POST /api/volumes`, `GET/PATCH /api/volumes/{id}`,
  `POST /api/volumes/{id}/folders`, `DELETE /api/volumes/{id}/folders/{fid}`,
  `POST /api/volumes/{id}/move`; pages `/volumes[/new][/{id}]` — FR-16..19
- [x] **T-IFIT** iFIT-inspired software features (D-12): geofence alerts
  (`GET /api/alerts`), reports view, scan-to-move, role display + audit-by-worker.
  Pages `/alerts`, `/reports`, `/scan`. Hardware (RFID/BLE/GPS/GIS) out of
  scope — FR-20..23
- [x] **T-11** OpenAPI 3.1 document for every endpoint:
  [`case-folder-service-with-rust/openapi.yaml`](../case-folder-service-with-rust/openapi.yaml) (P1) — NFR-6
- [x] **T-12** Client wire types generated from `openapi.yaml` via
  `openapi-typescript` (`npm run gen:api` → `src/lib/api/schema.d.ts`); the
  client aliases the generated schemas, keeping the camelCase domain types +
  mappers (P1) — NFR-6
- [x] **T-13** Automated axe accessibility scans (`tests/e2e/a11y.spec.ts`,
  9 routes, fails on serious/critical) (P1) — NFR-5
- [x] **T-14** vitest unit tests (`npm run test:unit`, 48 tests across 9
  files as of the DOC-7 audit — `nhs.ts` + all exported `client.ts`
  mappers + the `cache.svelte.ts` store (ST-13c) + `i18n.test.ts` +
  `routes/layout.test.ts` + components) + ESLint flat config
  (`npm run lint`, clean) for the Svelte client (P1)
- [x] **T-15** Harden the Modulus-11 core invariant on both editions
  (D-6) — NFR-1. Fixed two factual errors in
  [nhs-number.md](nhs-number.md)'s worked-example table (the `013 628 2963`
  row mis-stated its `Sum mod 11`/`Check` columns); added a genuine
  `check == 10 → invalid` example (`999 000 0140`); both editions now test
  that branch plus grouped/bare-form parity and the documented invalid
  numbers. Rust `nhs.rs` 6→8 lib tests; Svelte `nhs.test.ts` adds 4
  assertions.
- [x] **T-16** Geofence-breach derivation made pure + unit-tested (D-12,
  FR-20). The cross-building alert logic behind `GET /api/alerts` was
  extracted from the Loco controller into a pure `detect_geofence_breaches`
  function (plus a `cabinet_buildings` hierarchy resolver) and now has 6
  in-crate tests covering every branch of the boundary-crossing rule
  (cross-building breach, same-building suppression, `None`-endpoint
  in-transit/created-in-place, unknown cabinet, orphan-room hierarchy,
  mixed-log filtering). Previously reachable only via Postgres-gated request
  tests. Service lib tests 8→14; no behaviour change. Also corrected
  service-edition doc drift: `testing.md`/`README.md` unit count (6→14) and
  request-test count (29→49).

## Cross-edition hygiene (found 2026-09-04)

- [ ] **T-17** Dead nested CI: `case-folder-service-with-rust/.github/workflows/`
  (`quality.yml`, `security.yml`) is not referenced by either root pipeline
  — *(verified: `grep -rl case-folder .github/workflows/ .woodpecker.yml`
  finds nothing)*. Root `scripts/ci-crates.sh` already discovers this crate
  (any dir with a `Cargo.toml`) and runs `fmt`/`clippy`/`test`/`deny`
  against it via the family stages, so the nested workflows are pure
  duplication that never actually runs on GitHub (this repo has no Actions
  runner configured to pick up a nested `.github/`) — a contributor
  checking this path's Actions tab sees green-looking jobs that never
  executed. `patient-flow-service-with-rust`'s PF-T1 already faced the same
  question and explicitly deferred nested workflows; case-folder's were
  simply left in place unresolved. **Acceptance:** either delete both
  files (root CI already covers the same checks) or, if kept deliberately
  as a documented redundant local-CI convenience, say so in a comment at
  the top of each file and in this app's `spec/index.md`/`AGENTS.md` —
  either way the ambiguity is gone.
- [ ] **T-18** `openapi.yaml`'s `bearerAuth` scheme claims
  `bearerFormat: JWT`, but the value it actually accepts is the **opaque
  session id** (the same string as the `cts_session` cookie) — *(verified:
  `case-folder-service-with-rust/src/auth/mod.rs::bearer_token` strips the
  `Bearer ` prefix and passes the raw string straight to session lookup;
  nothing JWT-shaped is parsed on that path, only the `verify`-response's
  short-lived magic token is a real JWT and it is never accepted as a
  bearer credential)*. A client following the declared `bearerFormat`
  would reasonably assume it can mint or inspect a JWT, which is wrong and
  could mislead an integrator. **Acceptance:** `bearerFormat` reads
  something accurate (e.g. omitted, or `Opaque session id`), and
  [`api-contract.md`](../case-folder-service-with-rust/spec/api-contract.md)
  is checked for the same claim.

- [ ] **T-19** [audit-integrity.md](audit-integrity.md)'s hash-chain +
  signature design (driving **T-G2**) reinvents a scheme the family
  already built and audited: `integrity/integrity-mac-rust-crate`
  provides exactly this — HMAC-SHA256 MACs with HKDF-SHA256 per-(service,
  domain) subkeys, plus the `prev_hash`/`hash` audit-row chaining that
  person, worker, care-pathway and case already embed *(verified:
  `grep -n integrity-mac case-folder-service-with-rust/Cargo.toml` finds
  no dependency, and `audit-integrity.md`'s design section names no
  existing crate)*. Reusing it means case-folder's move-event chain gets
  the same production-grade key handling (key-file-over-env precedence,
  root-key zeroization, placeholder refusal) other services already have
  proven, instead of a bespoke implementation earning its own security
  review from scratch. **Acceptance:** `audit-integrity.md` either adopts
  `integrity-mac` as its dependency (updating the design section to match
  the crate's actual API) or records a stated reason it doesn't apply
  here (e.g. the table-less-aggregator constraint in [D-1](design.md))
  before T-G2 implementation starts.

## Production gates (blocked on deployment decisions)

- [~] **T-G1** Auth + ABAC across API and client (P0) — magic-link auth
  **done** (T-AUTH); remaining: **CIS2 / OIDC** identity + ABAC — **sketched**
  in [authorization.md](authorization.md); implementation pending
- [~] **T-G2** Append-only, hash-chained, signed audit log (P0) — **sketched**
  in [audit-integrity.md](audit-integrity.md); implementation pending
- [~] **T-G3** Same-origin deployment + re-enable SSR (P0) — **sketched**
  in [deployment.md](deployment.md); implementation pending
- [ ] **T-G4** Per-user move attribution from auth context (replaces free-text
  `movedBy`) — see [authorization.md](authorization.md) §4
- [ ] **T-G5** Secrets manager, `auto_migrate: false`, TLS/HSTS, CSP — see
  [deployment.md](deployment.md)

## How to add work

1. Add or update a requirement in [requirements.md](requirements.md).
2. Record the design decision in [design.md](design.md).
3. Add a `T-` row here, tracing to those IDs.
4. Update the affected topic files and the relevant subproject `spec/`.
5. Implement, test, and tick the box.
