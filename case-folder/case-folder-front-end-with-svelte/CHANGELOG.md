# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md) — single source of truth;
> [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

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
  - **Tests**: 18 Vitest unit tests + a 14-file Playwright e2e suite
    (smoke, dashboard, folders, patients, places, move, history, errors,
    volumes, clickthrough, auth, a11y, ifit, wiring) — boots the dev
    server and runs against the API in stub mode.

### Configuration

- `VITE_API_BASE_URL` (override the API origin; default proxies to
  `http://localhost:5150`).
- The Lily Svelte helpers are consumed in-source via SvelteKit `kit.alias`
  from a sibling clone at `~/git/lilydesignsystem/...`; that repo must be
  cloned for `pnpm dev` / `pnpm build` to resolve `@lily/locale-picker`
  and `@lily/theme-picker`.

### Notes

- **Demo application.** Not a regulated medical record — do not use with
  real patient data. See [spec/regulatory.md](./spec/regulatory.md).
