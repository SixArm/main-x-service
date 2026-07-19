# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [Unreleased]

### Added

- 2026-07-19 — SVAR DataGrid + Filter: new **/organizations** index route: the organization list in the
  SVAR DataGrid (**@svar-ui/svelte-grid**) with a
  **@svar-ui/svelte-filter** FilterBar (client-side name filter);
  row selection opens the detail route.

- 2026-07-19 — Lily Design System: the hand-rolled locale `<select>` is replaced by the Lily
  **LocaleSelect** (wired to the i18n store; `applyDir` off), and
  the **Lily headless** component library is now a dependency
  alongside the existing ThemeSelect.

### Changed

- **Auth pivot — BFF + cookie session + PASETO (spec-level; code
  follow-up pending).** The family is moving off the browser-held RS256
  JWT (cross-origin `#access_token` fragment handoff,
  `localStorage["mxi_access_token"]`) to a **Backend-For-Frontend**: the
  browser holds only an httpOnly `__Host-mxi_session` cookie, the
  front-end's own SvelteKit server exchanges the session for a
  short-lived **PASETO v4.public** token and calls the organization
  service server-side, and mutating requests are CSRF-protected. RS256
  JWT + JWKS are decommissioned. Human-facing docs (README/AGENTS/index)
  updated to describe the target model; the current runtime still uses
  the older client-held-token flow and the code follow-up is tracked in
  spec §13. Source of truth:
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md).

- **Docs/tests harmonization pass.** Brought the doc set back in line
  with the implemented bearer-token + SSO handoff increments: spec §2
  now scopes in the opt-in session/SSO (was "auth out of scope"), §8
  enumerates the payload incl. `telephone`/`email` and points at the new
  `build.ts`, and §11/§13 record the suite at 49 tests across 5 files.
  AGENTS.md gained `auth.svelte.ts`, `build.ts`, `VITE_AUTH_FRONTEND_URL`/
  `signInUrl`, the `tests/` tree, a Session/SSO section, and
  `pnpm test`/`pnpm test:e2e`. README documents `telephone`/`email` and
  the test commands. index.md adds a worked SSO-handoff diagram and an
  Organization payload JSON example (incl. the `{Custom: label}`
  identifier variant).

### Added

- **Form/payload core extracted + tested.** `OrganizationForm.build()`
  and its helpers moved into a pure `src/lib/api/build.ts`
  (`buildOrganization` + `splitList`/`blankToUndef`) so the spec §8 core
  is unit-testable without mounting the component; the §6.6 self-match
  filter is now `excludeSelf` in the same module (used by the detail
  route). New `tests/unit/build.test.ts` (14) covers comma-list
  splitting, blank→null clearing, contact fields, all-or-nothing
  address, dropping empty identifier rows, and self-match exclusion.
  `tests/unit/auth.test.ts` gained `captureFromLocation` coverage
  (store-write + fragment-strip / no-op). Suite is now 49 unit tests.
- **Cross-origin SSO token handoff (consumer side).** The operator now
  obtains a token from the central authentication front-end instead of
  pasting it. `signInUrl()` (`src/lib/config.ts`, new
  `VITE_AUTH_FRONTEND_URL`) builds
  `${VITE_AUTH_FRONTEND_URL}/signin?return_to=<encoded origin+base>`; the
  layout shows a primary **Sign in** button when signed out. On app load
  the layout's `onMount` runs `captureFromLocation()` (new in
  `auth.svelte.ts`) before any API call: `captureTokenFromHash` parses
  `…#access_token=<jwt>` out of the URL fragment (URL-decoded), `setToken`
  stores it, and `history.replaceState` strips the fragment. The manual
  paste field is kept (behind a "Paste a token" disclosure) as a dev
  fallback. vitest covers `captureTokenFromHash` (extract / decode /
  null cases) and `signInUrl` (encoded `return_to`, trailing-slash
  safe). Implements the "Token acquisition handoff" section of
  `agents/share/jwt-enforcement.md`.
- **Bearer-token session.** New reactive token store
  `src/lib/auth.svelte.ts` (`setToken`/`clearToken`/`token`), hydrated
  from the family-shared `localStorage["mxi_access_token"]` key and
  guarded for SSR/preview. `ApiClient` now attaches `Authorization:
  Bearer <token>` from the store on every request when signed in and
  omits it otherwise (an explicit per-request `token` still overrides;
  pass `null` to suppress). A minimal session affordance in the layout
  sidebar lets an operator paste/clear the token ("Use token" /
  "Sign out"). The token is obtained out-of-band from the central
  authentication-service; full magic-link redirect is a follow-up.
  vitest covers store round-trip + store-driven / override / cleared
  header attachment. Implements `agents/share/jwt-enforcement.md`
  (service enforcement stays off by default).
- **Test suites (T-11).** vitest unit tests (`tests/unit/`) for the
  `ApiClient` and `OrganizationRepository` — verb/path/body/bearer-token,
  error classification, and a regression pinning the `check-duplicates`
  path. Playwright smoke tests (`tests/e2e/`, 4) load the four routes
  with the API stubbed via `page.route`; they run against the
  production build (`vite preview`) to dodge the `vite dev` cold-start
  module-load race. `playwright.config.ts` added.

### Fixed

- Prettier formatting drift across `src/` (left behind by recent
  BFF/auth-era edits) broke the `pnpm lint` (`prettier --check src`)
  gate. Reformatted with `pnpm format`; no behavioural change —
  `svelte-check` and the vitest suite are unchanged and green.
- Copy-paste artifacts from the scaffold source: `client.ts` header
  said "Authentication Service"; `app.html` description said "Course
  Service" — both now read "Organization Service".

### Added (scaffold)

- **Inaugural scaffold (v0.1.0).** SvelteKit 2 / Svelte 5 (runes) SPA
  for the Organization Service, copy-adapted from
  authentication-front-end-with-svelte (same loco raw-JSON client).
  - Routes: `/` (list), `/new` (create), `/[pid]` (detail + delete +
    check-duplicates), `/[pid]/edit` (edit).
  - Lean API client extended with `put`/`delete`; `OrganizationRepository`
    (list/get/create/update/remove/checkDuplicates).
  - `types.ts` mirrors `organization_matcher::Organization` (the service
    DTO), including `IdentifierScheme` and `PostalAddress`.
  - `OrganizationForm` editing scalars, comma-list fields, a postal
    address, and a simple identifiers editor (unit-variant schemes).
  - SPA mode (`+layout.ts`); dependency-light (no SVAR/Lily). `pnpm run
    check` clean (0/0); production build succeeds.

### Configuration

- `PUBLIC_API_BASE_URL` (default `http://localhost:5150`).
- `VITE_AUTH_FRONTEND_URL` (default `http://localhost:5173`) — base URL of
  the central authentication front-end for the SSO sign-in handoff.
