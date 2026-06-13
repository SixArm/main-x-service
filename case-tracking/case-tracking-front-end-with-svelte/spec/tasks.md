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
- [x] **ST-12** 50 Playwright e2e tests against stub-mode API (—)

## Active / next

- [ ] **ST-13** vitest unit tests for `nhs.ts` + `client.ts` mappers (P1) — UR-4, UR-6
- [ ] **ST-14** ESLint + svelte-eslint (P1)
- [ ] **ST-15** `@axe-core/playwright` scans in CI (P1) — UR-9
- [ ] **ST-16** Codegen client types from API OpenAPI/JSON Schema (P1) — UR-6

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
   [Loco spec](../../case-tracker-service-with-rust/spec/routes.md) first.
