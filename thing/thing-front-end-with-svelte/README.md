# thing-front-end-with-svelte

SvelteKit front-end for the **[Thing Service](../thing-service-with-loco/)** in the Main X Index. Built on Svelte 5 (runes), SVAR Svelte DataGrid, and Lily Design System Svelte Headless primitives.

## What's here

| Route | Purpose |
| --- | --- |
| `/` | Dashboard — service health + recent audit activity |
| `/things` | List & search (full-text, fuzzy, phonetic) with SVAR DataGrid |
| `/things/new` | Create thing; surfaces 409 duplicate candidates |
| `/things/[id]` | Detail view — identity, identifiers, alternate names, same-as URLs, images |
| `/things/[id]/edit` | Edit |
| `/things/[id]/audit` | Per-thing audit log |
| `/things/match` | Match check — score a hypothetical record against the index |
| `/things/merge` | Merge two things (main + duplicate) |
| `/review` | Stored duplicate-review board |
| `/signin` | Per-app magic-link sign-in (BFF auth page) |
| `/verify` | Magic-link verification (BFF auth page) |

## Stack

- **SvelteKit 2** + **Svelte 5** (runes API)
- **@svar-ui/svelte-grid** + **@svar-ui/svelte-filter** (DataGrid + FilterBar; migrated off the legacy `wx-svelte-*` packages 2026-07-19)
- **Lily Design System Svelte Headless** (consumed via `file:` dependency)
- **TypeScript** strict mode
- **Vitest** for unit tests, **Playwright** for e2e

## Prerequisites

- Node.js 20+
- `pnpm` (or `npm`)
- A running Thing Service — see [`../thing-service-with-loco/README.md`](../thing-service-with-loco/README.md). Default: `http://localhost:5150` (loco dev default; see Configuration below).
- A running Authentication Service for sign-in — see [`../../authentication/authentication-service-with-loco/README.md`](../../authentication/authentication-service-with-loco/README.md). Default: `http://localhost:5150` (a distinct instance/port in a real deployment).

## Quick start

```bash
cp .env.example .env
pnpm install
pnpm dev
```

Open <http://localhost:5173>.

## Configuration

The browser calls the same-origin BFF proxy at `/api/proxy` — there is no public API env var (see `src/lib/config.ts`). The server-side BFF reads:

| Variable | Default | Purpose |
| --- | --- | --- |
| `THING_API_URL` | `http://localhost:5150` | Thing Service base URL — the proxy injects a server-exchanged PASETO and forwards |
| `AUTH_API_URL` | `http://localhost:5150` | Authentication Service base URL — magic-link login + session→PASETO exchange |

Set in `.env`. Both are read server-side in `src/lib/server/config.ts` and are never exposed to the client bundle.

## Testing

```bash
pnpm test         # vitest unit tests (no live service required)
pnpm test:e2e     # playwright smoke tests (no live service required)
pnpm check        # svelte-check type-check
```

The unit tests mock `fetch`. The Playwright suite asserts the page shells render even when the API is down (the page shows a banner; layout still mounts).

## Project layout

```
src/
  app.html
  app.css                  - shared CSS variables + utility classes
  app.d.ts
  hooks.server.ts          - resolves locals.sessionId from the BFF cookie
  lib/
    config.ts              - same-origin BFF proxy base (/api/proxy)
    i18n.svelte.ts          - 13-locale string catalog + translate()
    server/                 - BFF-only, never bundled into the browser
      config.ts             - THING_API_URL / AUTH_API_URL
      session.ts             - __Host-mxi_session cookie helpers
      auth.ts                 - magic-link + session->PASETO exchange calls
    api/
      types.ts             - Thing, ThingIdentifier, MatchResult, … (mirrors the Rust models)
      client.ts            - ApiClient + ApiError (envelope-aware fetch)
      things.ts           - ThingRepository (CRUD + search + match + merge + audit + review-queue)
    forms/
      form.svelte.ts       - createForm rune-based store
      LabeledField.svelte
      FieldError.svelte
      FieldRow.svelte
    components/
      SearchBox.svelte
      ThingGrid.svelte    - SVAR DataGrid binding
      ThingIdentifierInput.svelte
      ThingForm.svelte
      MatchResultsList.svelte
  routes/
    +layout.svelte         - top nav bar (hamburger on narrow viewports); theme/locale pickers
    +layout.server.ts       - exposes signedIn to the layout
    +page.svelte           - dashboard
    things/
      +page.svelte         - list
      new/+page.svelte
      match/+page.svelte
      merge/+page.svelte
      [id]/
        +page.svelte       - detail
        edit/+page.svelte
        audit/+page.svelte
    review/+page.svelte     - stored duplicate-review board (SVAR Kanban)
    signin/+page.svelte     - BFF magic-link request
    verify/+page.svelte     - BFF magic-link consume (sets the session cookie)
    api/proxy/[...path]/+server.ts - BFF reverse proxy (session -> PASETO -> Thing Service)
tests/
  unit/
    client.test.ts          - ApiClient envelope + error tests
    things.test.ts          - ThingRepository wrapping tests
    thing-form.test.ts       - FR-4 create/edit form validation
    merge-validation.test.ts - FR-9 merge guard
    i18n.test.ts             - 13-locale key-parity test
    layout.test.ts           - layout smoke
  e2e/
    things.spec.ts          - smoke tests (5)
```

## Lily Design System

The Lily file: dependency resolves to `~/git/lilydesignsystem/lily-design-system/lily-design-system-svelte-headless`. Components import via deep path:

```svelte
import Button from "lily-design-system-svelte-headless/src/lib/components/Button/Button.svelte";
```

The Lily **theme selector** (45 shared themes at `/assets/themes/`) and **locale selector** (13 locales) are wired live in the layout shell — `src/routes/+layout.svelte` imports and renders `ThemePicker` and `LocalePicker`. Lily Headless is available for further primitives as the design system stabilises.

## SVAR DataGrid

`wx-svelte-grid` is GPL-3.0 in its free tier. **If this front-end ships in a commercial product, evaluate the SVAR Pro/Enterprise license before adopting.** See `spec.md §16 Open questions`.

## Status

MVP scaffold. See [`spec.md`](spec/index.md) for the canonical work queue (§13 Tasks).
