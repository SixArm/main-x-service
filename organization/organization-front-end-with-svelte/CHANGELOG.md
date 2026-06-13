# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [Unreleased]

### Added

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
- **Test suites (T-11).** vitest unit tests (`tests/unit/`, 16) for the
  `ApiClient` and `OrganizationRepository` — verb/path/body/bearer-token,
  error classification, and a regression pinning the `check-duplicates`
  path. Playwright smoke tests (`tests/e2e/`, 4) load the four routes
  with the API stubbed via `page.route`; they run against the
  production build (`vite preview`) to dodge the `vite dev` cold-start
  module-load race. `playwright.config.ts` added.

### Fixed

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
