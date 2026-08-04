# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md) — single source of truth;
> [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

### Fixed — DOC-7 doc + e2e audit (2026-08-04)

- **Real e2e regression, not a doc bug**: `tests/e2e/smoke.spec.ts`'s
  "navigation menu links every primary route" and "aria-current marks
  the active nav link" asserted nav-link visibility without first
  opening the hamburger toggle. Commit `f66ff50f` (the cookie-session
  auth migration) changed `.navigation-menu` to always render
  `display: none` until the toggle adds `.open` — "collapsed behind the
  hamburger at every width", not just narrow ones (`src/lib/css/app.css`,
  confirmed by `git show f66ff50f -- .../app.css`) — but didn't update
  these two pre-existing tests, and nobody had run `npm run test:e2e`
  since (it needs the live Rust backend; see below). Fixed both tests to
  click `getByRole('button', { name: 'Toggle navigation' })` before
  asserting, matching the same interaction the unit test
  `routes/layout.test.ts` ("hamburger toggles nav visibility") already
  covers at the component level. Verified live against the sibling
  service in `USE_UPSTREAM_STUBS=1` mode: was 70/73 passing (this + one
  `auth.spec.ts` failure that turned out to be a local `FRONTEND_URL`
  port mismatch in my own test setup, not a real bug), now 73/73.
- **Stale test counts**: `spec/testing.md` said the e2e suite was
  "~65 `test()` cases" (its own per-file table already summed to the
  real 73 — only the prose was stale) and the vitest section said "43
  cases across 7 files", omitting `i18n.test.ts` (4 tests) and
  `routes/layout.test.ts` (1 test) entirely — live count is 48 across 9
  files. Fixed `spec/testing.md` (both counts + two new coverage
  bullets), `spec/tasks.md` (ST-12), and `AGENTS.md` (CI gate section).
  No `.env.example` bug of the TUT-1/TUT-2/DOC-4 class here: this app
  has no `.env.example` and no BFF — it's a CSR-only SPA that talks to
  `/api/*` through the Vite dev-server proxy (`LOCO_API_PROXY`,
  `VITE_API_BASE_URL`), both correctly documented in `README.md`.

### Added

- 2026-07-19 — SVAR component seams: **@svar-ui/svelte-calendar**,
  **@svar-ui/svelte-kanban**, **@svar-ui/svelte-gantt**, and
  **@svar-ui/svelte-filemanager** are installed (no routes yet —
  candidate features are catalogued per project; see the roadmap).

- 2026-07-19 — SVAR DataGrid + Filter: the folder dashboard grid migrates from `wx-svelte-grid` to
  **@svar-ui/svelte-grid**, with a **@svar-ui/svelte-filter**
  FilterBar above it filtering every column (NHS number, patient,
  folder, cabinet, status, last-moved). Legacy `wx-svelte-*` deps
  removed.

- 2026-07-19 — Lily Design System: the **Lily headless** component library is now a dependency,
  completing the Lily trio (ThemeSelect and LocaleSelect were
  already wired in the chrome).

### Fixed

- `pnpm lint` (`eslint .`) failed on two intentionally-unused
  underscore-prefixed bindings in the unit-test stubs
  (`src/lib/test-support/`). `@typescript-eslint/no-unused-vars` is now
  configured with the conventional `^_` ignore patterns (args, vars,
  caught errors, rest siblings) in `eslint.config.js`; no source change.

### Changed

- **ST-13c — cache store unit coverage.** New `cache.svelte.test.ts`
  exercises the rune cache singleton with no backend: setters / `clearUser` /
  `upsertFolder` (insert vs replace), the synchronous lookups, and
  `cabinetLocation`'s three-step resolution (`containerPath` → derived
  `Building — Room` → `? — ?` fallback). With `$lib/api/client` mocked it also
  asserts the cache side effects of `recordMove` (prepend + in-place folder
  relocation, plus the in-transit and folder-not-cached branches), `addFolder`,
  and `addBuilding/Room/Cabinet`. Fixed a `cache-api.md` drift:
  `cabinetLocation` returns "In transit" for an unknown id, not only `null`.
  vitest unit count is now 43 (was 26).
- **T-15 — Modulus-11 hardening.** `nhs.test.ts` now covers the genuine
  `check === 10 → invalid` branch (`999 000 0140`, asserted across all ten
  trailing digits), the documented invalid number `614 309 0431`, leading-zero
  normalisation, the empty-input case, and grouped/bare-form parity. Fixed a
  wrong comment that mis-described why `013 628 2963` is invalid (it fails on
  a check-digit mismatch, not because the check computes to 10). Reordered the
  `check === 10` guard in `nhs.ts` ahead of the check-digit computation to
  mirror the Rust edition; behaviour is unchanged. vitest unit count is now 26
  (was 24).
- **ST-13 complete — mapper unit coverage.** `client.test.ts` now exercises
  every exported snake→camel mapper (`toPatient`, `toFolder`, `toMove`,
  `toBuilding`, `toRoom`, `toCabinet`, `toWorker`, `toStats`) plus `ApiError`;
  vitest unit count is now 24 (was 18).
- **A11y fix.** Removed the redundant `role="separator"` from
  `Separator.svelte` — an `<hr>` already carries the implicit `separator`
  role. `npm run check` is now 0 errors / 0 warnings.

### Added

- **Inaugural scaffold (v0.1.0).** SvelteKit 2 / Svelte 5 (runes)
  client-only SPA for Case Tracking, consuming the
  [Loco JSON API sibling](../case-folder-service-with-rust) over `/api/*`.
  - **Routes**: dashboard (`/`), patients (`/patients`, `/patients/{nhs}`),
    folders (`/folders`, `/folders/new`, `/folders/{id}`), buildings,
    rooms, cabinets, volumes (`/volumes`, `/volumes/new`, `/volumes/{id}`),
    workers, move folder (`/move`), scan, reports, alerts, audit history,
    and magic-link auth (`/login`, `/auth/callback`).
  - **Hybrid load + cache** wiring: `+page.ts` loaders call the typed
    `api.*` client (`src/lib/api/client.ts`, snake→camel mapping) and push
    results into a rune-reactive cache (`src/lib/store/cache.svelte.ts`);
    components read via reactive getters; mutations round-trip and update
    the cache in place.
  - **Same-origin auth**: the dev server proxies `/api` to the Loco app so
    the session is a first-party HttpOnly cookie.
  - **NHS Number** Modulus 11 client-side pre-flight (`src/lib/store/nhs.ts`).
  - **UI**: Lily Design System (Svelte headless) primitives styled with NHS
    UK tokens; SVAR `wx-svelte-grid` (Willow theme) dashboard grid;
    addressograph box, labels print dialog, button bar.
  - **Theming & locale**: `nhs` / `nhs-high-contrast` themes and `en` / `cy`
    / `gd` locales via the Lily theme/locale pickers, persisted to
    `localStorage`.
  - **Tests**: Vitest unit tests + a 14-file Playwright e2e suite
    (smoke, dashboard, folders, patients, places, move, history, errors,
    volumes, clickthrough, auth, a11y, ifit, wiring) — boots the dev
    server and runs against the API in stub mode.

### Configuration

- `LOCO_API_PROXY` (dev-proxy target; default `http://localhost:5150`,
  read in `vite.config.ts`) keeps `/api` same-origin so the session
  cookie stays first-party. `VITE_API_BASE_URL` points the client at a
  different origin instead (bypasses the proxy; affects cookies).
- The Lily Svelte helpers are consumed in-source via SvelteKit `kit.alias`
  from a sibling clone at `~/git/lilydesignsystem/...`; that repo must be
  cloned for `npm run dev` / `npm run build` to resolve `@lily/locale-picker`
  and `@lily/theme-picker`.

### Notes

- **Demo application.** Not a regulated medical record — do not use with
  real patient data. See [spec/regulatory.md](./spec/regulatory.md).
