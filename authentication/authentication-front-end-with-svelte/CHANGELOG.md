# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md) — single source of truth;
> [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

### Added

- **Inaugural scaffold (v0.1.0).** SvelteKit 2 / Svelte 5 (runes) SPA
  for the Authentication Service.
  - Routes: `/` (account + sign out), `/signup`, `/signin`, and
    `/verify` (consumes `?token=` → stores the access token → redirects).
  - Lean API client (`src/lib/api/client.ts`) matched to the loco
    service's **raw JSON** responses (no `{success,data,error}`
    envelope), with bearer-token support and an `ApiError` type.
  - `AuthRepository` wrapping signup / magic-link / verify / me /
    signout; `LoginResponse` / `CurrentUser` types mirroring the service
    views.
  - Runes-based client session (`src/lib/auth/session.svelte.ts`):
    access token + cached profile persisted to `localStorage`.
  - SPA mode via `+layout.ts` (`ssr = false`, `prerender = false`);
    `adapter-auto`.
  - Deliberately dependency-light: no SVAR DataGrid and no Lily Design
    System (this UI has no tables) — accepted drift from the sibling
    front-ends. `pnpm run check` clean; production build succeeds.
- **Test suites (T-11).** vitest unit tests
  (`tests/unit/client.test.ts` + `tests/unit/auth.test.ts`, 16) covering
  the `ApiClient` (URL join, JSON body, bearer-token attachment for
  `/me` + `/signout`, empty-body handling, `ApiError` classification,
  non-JSON fallback) and the `AuthRepository` (exact path/verb/body for
  signup, magic-link request, verify with URL-encoded token, `me`,
  signout); playwright smoke tests (`tests/e2e/smoke.spec.ts`, 7) that
  stub the auth API and load every route (sign-up / sign-in / verify /
  signed-in + signed-out home). `playwright.config.ts` runs against
  `vite preview`.

### Fixed

- `src/app.html` meta description named the *Course Service* — corrected
  to the Authentication Service (scaffold copy-paste artifact).

### Configuration

- `PUBLIC_API_BASE_URL` (default `http://localhost:5150`).
