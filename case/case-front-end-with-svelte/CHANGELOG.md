# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [Unreleased]

### Added

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
