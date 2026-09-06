# Tasks (Svelte edition delivery)

> Part of the [Svelte edition specification](index.md). Cross-edition
> board: [root tasks](../../spec/tasks.md). Tasks trace to
> [requirements.md](requirements.md) (UR-) and [design.md](design.md) (SD-).

## Status legend

- [x] done · [~] in progress · [ ] not started

## Delivered (in repo)

- [x] **ST-1** API client with snake↔camel mappers + `ApiError` (SD-2) — UR-6
- [x] **ST-2** Rune-reactive cache singleton + setters/upserters/mutations (SD-1, SD-5) — UR-7
- [x] **ST-3** Dashboard (KPIs, recent moves, cabinet utilisation, FolderGrid) (SD-1) — UR-2
- [x] **ST-4** Folders: list/search, detail+history, create with validation (SD-1, SD-4) — UR-1, UR-4
- [x] **ST-5** Patients: list, detail with snapshot-fallback warning (SD-4) — UR-5
- [x] **ST-6** Places: buildings/rooms/cabinets list/show/create (SD-1) — UR-1
- [x] **ST-7** Move workflow: live NHS lookup + worker/cabinet pickers (SD-1, SD-2) — UR-3
- [x] **ST-8** History: audit log with free-text filter (SD-1) — UR-1
- [x] **ST-9** `+error.svelte` hard-fail handling (404/503) (SD-4) — UR-8
- [x] **ST-10** Accessibility baseline (skip link, single h1, labelled fields) (SD-6) — UR-9
- [x] **ST-11** Locale + theme pickers via Lily helpers; NHS themes (SD-6)
- [x] **ST-12** Playwright e2e suite against stub-mode API — 14 spec
  files, 73 `test()` cases (smoke, dashboard, folders, patients, places,
  move, history, errors, volumes, clickthrough, auth, ifit, wiring, a11y) (—)

## Active / next

- [x] **ST-13** vitest unit tests for `nhs.ts` + `client.ts` mappers (P1) — UR-4, UR-6.
  `nhs.test.ts` covers normalise/format/Modulus-11; `client.test.ts` covers
  every exported mapper (`toPatient`, `toFolder`, `toMove`, `toBuilding`,
  `toRoom`, `toCabinet`, `toWorker`, `toStats`) + `ApiError`. Also removed a
  redundant `role="separator"` on `Separator.svelte` (`<hr>` carries the
  implicit role), so `npm run check` is back to 0 errors / 0 warnings.
- [x] **ST-13b** Modulus-11 hardening (root **T-15**). `nhs.test.ts` adds the
  `check === 10 → invalid` branch (`999 000 0140`), the documented invalid
  number `614 309 0431`, leading-zero normalisation, empty input, and
  grouped/bare parity; `nhs.ts` reorders the `check === 10` guard to mirror
  the Rust edition (no behaviour change). vitest unit count 24→26.
- [x] **ST-13c** vitest unit tests for the cache singleton
  (`cache.svelte.test.ts`, P1) — UR-7. Covers the rune store's setters /
  `clearUser` / `upsertFolder` (insert vs replace), the four synchronous
  lookups, and `cabinetLocation`'s three-step resolution
  (`containerPath` → derived `Building — Room` → `? — ?` fallbacks). With
  `api.*` mocked it also asserts the cache side effects of `recordMove`
  (prepend + in-place folder relocation, including the in-transit and
  folder-not-cached branches), `addFolder`, and `addBuilding/Room/Cabinet`.
  Also fixed a `cache-api.md` drift: `cabinetLocation` returns "In transit"
  for an unknown id, not only `null`. vitest unit count 26→43.
- [x] **ST-14** ESLint + svelte-eslint (P1). `eslint.config.js` (flat
  config) + `eslint-plugin-svelte` + `svelte-eslint-parser`; `npm run
  lint` (`eslint .`) is clean. Cross-referenced in root **T-14**; this
  row was left unticked after that landed — reconciled 2026-08-29.
- [x] **ST-15** `@axe-core/playwright` scans (P1) — UR-9.
  `tests/e2e/a11y.spec.ts` scans 9 primary routes (`/`, `/patients`,
  `/folders`, `/volumes`, `/workers`, `/cabinets`, `/alerts`, `/reports`,
  `/scan`) for serious/critical violations.
- [x] **ST-16** Codegen client types from API OpenAPI/JSON Schema (P1) —
  UR-6. `npm run gen:api` runs `openapi-typescript
  ../case-folder-service-with-rust/openapi.yaml -o
  src/lib/api/schema.d.ts`; `client.ts` imports `components` from the
  generated schema and aliases its wire types. Verified 2026-08-29:
  regenerating produces no diff (schema is current). Cross-referenced
  in root **T-12**; this row was left unticked after that landed —
  reconciled 2026-08-29.

- [x] **ST-17** *(resolved 2026-09-06.)* No Prettier at all:
  `package.json` had no `prettier` or `prettier-plugin-svelte`
  devDependency, no `format`/`format:check` script, and no
  `.prettierrc`/`prettier.config.*` file, so this app's `npm run lint`
  (ESLint only) was the one place in the family relying on ESLint
  alone for style.
  - **Resolved.** Added `prettier` + `prettier-plugin-svelte` as
    devDependencies; `format` (`prettier --write src`) and
    `format:check` (`prettier --check src`) scripts, **scoped to
    `src`** like every sibling front-end's own scripts (not `.` —
    running unscoped over the whole project also reformats
    `spec/*.md`, `pnpm-lock.yaml`, and config files nobody intended to
    touch, discovered by trying it first and reverting); a new
    `.prettierrc` (`tabWidth: 4`, `singleQuote: true`,
    `prettier-plugin-svelte`) matching this project's **existing**
    style, chosen specifically so adopting the tool needed a real
    first-time reformat rather than an arbitrary style change.
  - **The first-time reformat is part of this same change** (85 files
    under `src` had never been Prettier-formatted): verified
    mechanical-only (trailing commas, line wraps, quote
    normalisation — no logic touched) by diffing several files by
    hand and by `npm run check` / `npm run test:unit` / `npm run lint`
    all staying green afterwards (50/50 unit tests, 0 errors, 0
    ESLint issues).
  - `AGENTS.md` §"CI gate" now lists `npm run format:check` alongside
    `npm run check` and `npm run lint`.
  - **Acceptance met:** `npm run format:check` is clean; `npm run
    lint`/`npm run check` unaffected.
- [x] **ST-18** *(resolved 2026-09-06.)* `UserView.role` (from `GET
  /api/auth/me`) is rendered in three places (`+layout.svelte`,
  `move/+page.svelte`, `workers/+page.svelte`) but there was no test
  asserting the signed-in user's own role renders correctly in the nav
  utility row. Once **T-G1**'s CIS2/OIDC roles land this becomes an
  ABAC signal worth trusting the UI reflects correctly.
  - **Resolved.** Two new `vitest`/`@testing-library/svelte` cases in
    `src/routes/layout.test.ts` (extending the existing hamburger-toggle
    describe block, which already seeded a stub user but never asserted
    on the role text): one seeds `role: 'clerk'` and asserts the
    `.auth-status` element's text contains `Test Operator(clerk)`; the
    other seeds `role: null` and asserts the name renders with no `(`
    at all. Verified to fail (assert the missing text) with the
    template's `{#if user.role}` branch stripped, and pass with it
    restored.
  - **Acceptance met:** `npm run test:unit` — 50/50 (was 47); `npm run
    check` clean.

- [ ] **ST-19** `src/lib/api/schema.d.ts` is a **committed, generated**
  file (`npm run gen:api` runs `openapi-typescript` against the sibling
  crate's `openapi.yaml`) with no CI step re-generating and diffing it —
  *(verified: `grep -n "gen:api\|schema.d.ts" AGENTS.md` finds the
  script documented as a manual recipe step only; the "CI gate" section
  lists just `npm run check` + `npm run test:e2e`)*. **ST-16**'s
  "regenerating produces no diff" claim was a one-off manual check on
  2026-08-29, not an enforced invariant — a future `openapi.yaml` edit
  (e.g. this pass's **LT-18** `bearerFormat` fix) can drift silently
  from the checked-in types with nothing failing. **Acceptance:** a CI
  step runs `npm run gen:api` and fails the build on a non-empty `git
  diff` against the committed file, added to the CI gate list in
  [`AGENTS.md`](../AGENTS.md).

## Production gates

- [ ] **ST-G1** Auth (CIS2 / OIDC) threaded through `api.*` (P0) — see [regulatory.md](regulatory.md)
- [ ] **ST-G2** Same-origin deployment + re-enable SSR (P0)
- [ ] **ST-G3** Per-user `movedBy` from auth context (replaces free-text)
- [ ] **ST-G4** CSP, IG review of SVAR font CDN

## Recipe: add a page

1. Add a use case + route row in [routes.md](routes.md).
2. Create `+page.ts` (load+cache) and `+page.svelte`.
3. Add a cache method + document it in [cache-api.md](cache-api.md) if
   new mutation behaviour is needed.
4. Add a nav link in `+layout.svelte`; run `npm run check`.
5. New API surface? Add it to the
   [Loco spec](../../case-folder-service-with-rust/spec/routes.md) first.
