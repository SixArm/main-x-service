# event-front-end-with-svelte

SvelteKit front-end for the **[Event Service](../event-service-with-loco/)** in the Main X Index. Built on Svelte 5 (runes), SVAR Svelte DataGrid, and Lily Design System Svelte Headless primitives.

## What's here

| Route | Purpose |
| --- | --- |
| `/` | Dashboard — service health + recent audit activity |
| `/events` | List & search (full-text + fuzzy toggle + date / status / type filters) with SVAR DataGrid |
| `/events/new` | Create event; surfaces 409 duplicate candidates |
| `/events/[id]` | Detail view — identity (time window, status, type, attendance mode), locations, organizers, performers, identifiers, offers |
| `/events/[id]/edit` | Edit |
| `/events/[id]/audit` | Per-event audit log |
| `/events/match` | Match check — score a hypothetical record against the index |
| `/events/merge` | Merge two events (main + duplicate) |
| `/calendar` | SVAR Calendar over the event time-window — drag an event to a new slot to reschedule it (writes back via the normal update endpoint) |
| `/signin` | Magic-link sign-in (BFF flow against the auth service) |
| `/verify` | Magic-link verification landing page |

## Stack

- **SvelteKit 2** + **Svelte 5** (runes API)
- **SVAR Svelte DataGrid** (`wx-svelte-grid`, `wx-svelte-core`)
- **Lily Design System Svelte Headless** (consumed via `file:` dependency)
- **TypeScript** strict mode
- **Vitest** for unit tests, **Playwright** for e2e

## Prerequisites

- Node.js 20+
- `pnpm` (or `npm`)
- A running Event Service — see [`../event-service-with-loco/README.md`](../event-service-with-loco/README.md). Default: `http://localhost:8080`.

## Quick start

```bash
cp .env.example .env
pnpm install
pnpm dev
```

Open <http://localhost:5173>.

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `EVENT_API_URL` | `http://localhost:5150` | Event Service base URL (server-side only) |
| `AUTH_API_URL` | `http://localhost:5150` | Authentication Service base URL (server-side only) |

Set in `.env`. Both are **server-side** variables read in `src/lib/server/config.ts` — they are never bundled into the browser. The browser talks only to the app's own origin: entity-API calls go through the same-origin `/api/proxy` BFF route, which forwards them to the Event Service with a server-injected PASETO.

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
    config.ts              - API_BASE_URL (same-origin /api/proxy BFF)
    api/
      types.ts             - Event, Location, Party, Offer, Identifier, MatchResult, … (mirrors the Rust models)
      client.ts            - ApiClient + ApiError (envelope-aware fetch)
      events.ts           - EventRepository (CRUD + search + match + merge + audit)
    forms/
      form.svelte.ts       - createForm rune-based store
      LabeledField.svelte
      FieldError.svelte
      FieldRow.svelte
    components/
      SearchBox.svelte
      EventGrid.svelte    - SVAR DataGrid binding
      EventForm.svelte
      MatchResultsList.svelte
  routes/
    +layout.svelte         - sidebar nav
    +page.svelte           - dashboard
    events/
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
    events.test.ts        - EventRepository wrapping tests
  e2e/
    events.spec.ts        - smoke tests
```

## Lily Design System

The Lily file: dependency resolves to `~/git/lilydesignsystem/lily-design-system/lily-design-system-svelte-headless`. Components import via deep path:

```svelte
import Button from "lily-design-system-svelte-headless/src/lib/components/Button/Button.svelte";
```

Lily's `ThemeSelect` and `LocaleSelect` components are live in `src/routes/+layout.svelte` — theme choice persists via ThemeSelect's own storage key, and LocaleSelect drives the i18n store (which sets `lang`/`dir` on `<html>`).

## SVAR DataGrid

`wx-svelte-grid` is GPL-3.0 in its free tier. **If this front-end ships in a commercial product, evaluate the SVAR Pro/Enterprise license before adopting.** See `spec.md §16 Open questions`.

## Status

MVP scaffold. See [`spec.md`](spec/index.md) for the canonical work queue (§13 Tasks).
