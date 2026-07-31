# place-front-end-with-svelte

SvelteKit front-end for the **[Place Service](../place-service-with-loco/)** in the Main X Index. Built on Svelte 5 (runes), SVAR Svelte DataGrid, and Lily Design System Svelte Headless primitives.

## What's here

| Route | Purpose |
| --- | --- |
| `/` | Dashboard — service health + recent audit activity |
| `/places` | List & search (full-text, fuzzy, phonetic) with SVAR DataGrid |
| `/places/new` | Create place; surfaces 409 duplicate candidates |
| `/places/[id]` | Detail view — identity, address, geo coordinates, identifiers, opening hours, amenities |
| `/places/[id]/edit` | Edit |
| `/places/[id]/audit` | Per-place audit log |
| `/places/match` | Match check — score a hypothetical record against the index |
| `/places/merge` | Merge two places (main + duplicate) |
| `/review` | Stored duplicate-review board |
| `/signin` | Per-app magic-link sign-in (BFF auth page) |
| `/verify` | Magic-link verification (BFF auth page) |

## Stack

- **SvelteKit 2** + **Svelte 5** (runes API)
- **SVAR Svelte DataGrid** (`wx-svelte-grid`, `wx-svelte-core`)
- **Lily Design System Svelte Headless** (consumed via `file:` dependency)
- **TypeScript** strict mode
- **Vitest** for unit tests, **Playwright** for e2e

## Prerequisites

- Node.js 20+
- `pnpm` (or `npm`)
- A running Place Service — see [`../place-service-with-loco/README.md`](../place-service-with-loco/README.md). Default: `http://localhost:8080`.

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
| `PLACE_API_URL` | `http://localhost:5150` | Place Service base URL — the proxy injects a server-exchanged PASETO and forwards |
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
  lib/
    config.ts              - same-origin BFF proxy base (/api/proxy)
    api/
      types.ts             - Place, PostalAddress, GeoCoordinates, MatchResult, … (mirrors the Rust models)
      client.ts            - ApiClient + ApiError (envelope-aware fetch)
      places.ts           - PlaceRepository (CRUD + search + match + merge + audit)
    forms/
      form.svelte.ts       - createForm rune-based store
      LabeledField.svelte
      FieldError.svelte
      FieldRow.svelte
    components/
      SearchBox.svelte
      PlaceGrid.svelte    - SVAR DataGrid binding
      PostalAddressInput.svelte
      GeoCoordinatesInput.svelte
      PlaceForm.svelte
      MatchResultsList.svelte
  routes/
    +layout.svelte         - sidebar nav
    +page.svelte           - dashboard
    places/
      +page.svelte         - list
      new/+page.svelte
      match/+page.svelte
      merge/+page.svelte
      [id]/
        +page.svelte       - detail
        edit/+page.svelte
        audit/+page.svelte
tests/
  unit/
    client.test.ts         - ApiClient envelope + error tests
    places.test.ts        - PlaceRepository wrapping tests
  e2e/
    places.spec.ts        - smoke tests
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
