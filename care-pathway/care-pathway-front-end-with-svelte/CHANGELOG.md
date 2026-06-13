# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [Unreleased]

### Added

- **Test suites (T-5).** vitest unit tests (`tests/unit/`, 16) for the
  `ApiClient` and `CarePathwayRepository` — verb/path/body/bearer-token,
  error classification, and a regression pinning the `check-duplicates`
  path. Playwright smoke tests (`tests/e2e/`, 4) load the four routes
  with the API stubbed via `page.route`; they run against the
  production build (`vite preview`) to dodge the `vite dev` cold-start
  module-load race. `playwright.config.ts` added.

### Fixed

- Copy-paste artifacts from the scaffold source: `client.ts` header
  said "Authentication Service"; `app.html` description said "Course
  Service" — both now read "Care Pathway Service".

### Added (scaffold)

- **Inaugural scaffold (v0.1.0).** SvelteKit 2 / Svelte 5 (runes) SPA
  for the Care Pathway Service, copy-adapted from
  organization-front-end-with-svelte (same loco raw-JSON client).
  - Routes: `/` (list), `/new` (create), `/[pid]` (detail + delete +
    check-duplicates), `/[pid]/edit` (edit).
  - Lean API client (get/post/put/delete); `CarePathwayRepository`.
  - `types.ts` mirrors `care_pathway_matcher::CarePathway` (the service
    DTO), including `CodeSystem`, `CareSetting`, and `IdentifierScheme`.
  - `CarePathwayForm` editing scalars, care setting, target condition
    codes (system + code rows), interventions/keywords, and identifiers.
  - SPA mode; dependency-light (no SVAR/Lily). `pnpm run check` clean
    (0/0); production build succeeds.

### Configuration

- `PUBLIC_API_BASE_URL` (default `http://localhost:5150`).
