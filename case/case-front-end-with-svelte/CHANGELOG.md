# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [Unreleased]

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
