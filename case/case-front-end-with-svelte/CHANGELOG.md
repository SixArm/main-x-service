# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [Unreleased]

### Added

- 2026-07-19 — SVAR component seams: **@svar-ui/svelte-calendar**,
  **@svar-ui/svelte-kanban**, **@svar-ui/svelte-gantt**, and
  **@svar-ui/svelte-filemanager** are installed (no routes yet —
  candidate features are catalogued per project; see the roadmap).

- 2026-07-19 — SVAR DataGrid + Filter: new **/cases** index route: the case list in the SVAR DataGrid
  with a FilterBar (client-side title filter); row selection opens
  the detail route.

- 2026-07-19 — Lily Design System: the hand-rolled locale `<select>` is replaced by the Lily
  **LocaleSelect** (wired to the i18n store; `applyDir` off), and
  the **Lily headless** component library is now a dependency
  alongside the existing ThemeSelect.

### Fixed

- Prettier formatting drift across `src/` (left behind by recent
  BFF/auth-era edits) broke the `pnpm lint` (`prettier --check src`)
  gate. Reformatted with `pnpm format`; no behavioural change —
  `svelte-check` and the vitest suite are unchanged and green.

### Changed

- **Auth pivot.** The family
  authentication model moved from **client-held RS256 JWT bearer tokens**
  (fragment handoff + `localStorage["mxi_access_token"]`) to a
  **Backend-For-Frontend (BFF) + httpOnly cookie session + CSRF**, with
  the BFF exchanging the session for a short-lived **PASETO v4.public**
  token for server-side service calls — see
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
  as the source of truth; RS256/JWKS are decommissioned. Human-facing
  docs (README / index / AGENTS) describe the BFF + cookie model;
  the browser holds no token. The runtime implements the BFF
  (`src/hooks.server.ts`, `src/lib/server/*`, the `/api/proxy/[...path]`
  server route); the old client-held bearer flow (`auth.svelte.ts`,
  fragment capture) is removed.
- **Doc harmonization pass.** Refreshed `AGENTS.md` to match the spec's
  auth/SSO layer: added `src/lib/auth.svelte.ts` and the `tests/` tree to
  the layout, noted `config.ts` now also exports `AUTH_FRONTEND_URL` /
  `signInUrl()`, added a bearer-token / SSO ground rule, and documented
  `VITE_AUTH_FRONTEND_URL` alongside `PUBLIC_API_BASE_URL`. Added an
  SSO token-handoff worked example to `index.md` so the navigation aid
  reflects the implemented sign-in flow (spec §6.7-6.8, §8). No code
  change; vitest 40 green, `pnpm run check` 0/0.

### Added

- **Cross-origin SSO token handoff (consumer side)** (family contract
  [`agents/share/jwt-enforcement.md`](../../agents/share/jwt-enforcement.md),
  "Token acquisition handoff").
  - `src/lib/auth.svelte.ts` gains `captureTokenFromHash(hash)` — a pure
    parser pulling `access_token` out of a `#…access_token=<jwt>…` URL
    fragment (with/without leading `#`, URL-decoded, `null` otherwise) —
    and a browser-only `captureFromLocation()` that reads
    `window.location.hash`, stores any token, then strips the fragment
    via `history.replaceState` so the bearer credential never lingers in
    the address bar / history.
  - The layout runs `captureFromLocation()` once in `onMount` before any
    route makes an API call.
  - `src/lib/config.ts` gains `AUTH_FRONTEND_URL` (from
    `VITE_AUTH_FRONTEND_URL`, default `http://localhost:5173`) and
    `signInUrl(origin?, basePath?)`, building
    `${AUTH_FRONTEND_URL}/signin?return_to=<encoded origin + base>`
    (trailing slash trimmed; origin / base injectable for tests).
  - Layout sidebar now shows a primary **Sign in** link (redirects to the
    auth front-end) when signed out; the manual paste field is demoted to
    a dev-only `<details>`. **Sign out** unchanged.
  - Tests: `auth.test.ts` adds the `captureTokenFromHash` cases
    (well-formed, multi-param, no leading `#`, URL-decode, empty/`#`,
    no-token, garbage → `null`); new `config.test.ts` covers `signInUrl`
    (encoded `return_to`, base path, trailing-slash safety). vitest 31
    green; Playwright smoke suite stays green.

- **Session bearer-token attachment** (family contract
  [`agents/share/jwt-enforcement.md`](../../agents/share/jwt-enforcement.md)).
  - New reactive session store `src/lib/auth.svelte.ts` (`token` /
    `setToken` / `clearToken`), hydrated from the family-shared
    `localStorage["mxi_access_token"]` and guarded for SSR / `vite
    preview` / vitest where `localStorage` is absent.
  - `ApiClient` now reads the session token by default and attaches
    `Authorization: Bearer <token>` on every request when present; a
    per-call `token` (string or `null`) still overrides, and a
    `tokenSource` seam keeps it unit-testable.
  - Minimal session affordance in the layout sidebar: paste / clear the
    token (issued by the central authentication-service magic-link flow),
    so operator traffic passes the service's blanket JWT enforcement
    (`CASE_REQUIRE_AUTH`) once activated.
  - Tests: vitest `auth.test.ts` (no-token default, round-trip, guarded
    write-through under the shared key) + new `ApiClient` cases
    (store-default header, per-call `null` override). Playwright smoke
    suite stays green.

### Added (scaffold)

- **Inaugural scaffold (v0.1.0).** SvelteKit 2 / Svelte 5 (runes) SPA
  for the Case Service (governmental case management), copy-adapted from
  care-pathway-front-end-with-svelte (same loco raw-JSON client).
  - Routes: `/` (list), `/new` (create), `/[pid]` (detail + delete +
    check-duplicates), `/[pid]/edit` (edit).
  - Lean API client (get/post/put/delete); `CaseRepository`.
  - `types.ts` mirrors `case_matcher::Case` (the service DTO),
    including `CaseType`, `CaseStatus`, `Priority`, and
    `IdentifierScheme`, plus the `ALL_*` dropdown arrays.
  - `CaseForm` editing title (required), case number, agency id/name,
    case type / status / priority dropdowns, opened date, comma-list
    inputs (alternate titles / subjects / keywords / same-as /
    languages), and identifier rows (scheme + value).
  - SPA mode; dependency-light (no SVAR/Lily). `pnpm run check` clean
    (0/0); production build succeeds.
  - **Test suites.** vitest unit tests (`tests/unit/`, 16) for the
    `ApiClient` and `CaseRepository` — verb/path/body/bearer-token,
    error classification, and a regression pinning the
    `check-duplicates` path. Playwright smoke tests (`tests/e2e/`, 4)
    load the four routes with the API stubbed via `page.route`; they
    run against the production build (`vite preview`) to dodge the
    `vite dev` cold-start module-load race. `playwright.config.ts`
    included.

### Configuration

- `PUBLIC_API_BASE_URL` (default `http://localhost:5150`).
