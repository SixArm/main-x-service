# worker-front-end-with-svelte

SvelteKit front-end for the **[Worker Service](../worker-service-with-loco/)** in the Main X Index. Built on Svelte 5 (runes), SVAR Svelte DataGrid, and Lily Design System Svelte Headless primitives.

## What's here

| Route | Purpose |
| --- | --- |
| `/` | Dashboard — service health + recent audit activity |
| `/workers` | List & search (full-text, fuzzy, phonetic) with SVAR DataGrid |
| `/workers/new` | Create worker; surfaces 409 duplicate candidates |
| `/workers/[id]` | Detail view — identity, identifiers, addresses, telecom, emergency contacts |
| `/workers/[id]/edit` | Edit |
| `/workers/[id]/audit` | Per-worker audit log |
| `/workers/match` | Match check — score a hypothetical record against the index |
| `/workers/merge` | Merge two workers (main + duplicate) |
| `/review` | Stored duplicate-review board — drag-to-decide |
| `/expiry` | Credential-expiry calendar |
| `/signin` | Per-app magic-link sign-in (BFF auth page) |
| `/verify` | Magic-link verification (BFF auth page) |

## Stack

- **SvelteKit 2** + **Svelte 5** (runes API)
- **SVAR Svelte** (`@svar-ui/svelte-grid` + `svelte-filter` for the workers grid; `svelte-calendar`/`svelte-kanban` for `/expiry`/`/review`; `svelte-gantt`/`svelte-filemanager` installed, no routes yet)
- **Lily Design System Svelte Headless** (consumed via `file:` dependency)
- **TypeScript** strict mode
- **Vitest** for unit tests, **Playwright** for e2e

## Prerequisites

- Node.js 20+
- `pnpm` (or `npm`)
- A running Worker Service — see [`../worker-service-with-loco/README.md`](../worker-service-with-loco/README.md). Default: `http://localhost:5150`.
- A running Authentication Service for `/signin`/`/verify` — see [`../../authentication/authentication-service-with-loco/README.md`](../../authentication/authentication-service-with-loco/README.md). Default: `http://localhost:5150`.

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
| `WORKER_API_URL` | `http://localhost:5150` | Worker Service base URL — the proxy injects a server-exchanged PASETO and forwards |
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
  hooks.server.ts          - BFF: reads the session cookie into event.locals
  lib/
    config.ts              - same-origin BFF proxy base (/api/proxy)
    server/                - BFF-only (never bundled into the browser)
      config.ts            - WORKER_API_URL, AUTH_API_URL
      session.ts           - __Host-mxi_session cookie name + attributes
      auth.ts              - magic-link + session->PASETO calls to the auth service
    api/
      types.ts             - Worker, HumanName, MatchResult, EntityLink, … (mirrors the Rust models)
      client.ts            - ApiClient + ApiError (envelope-aware fetch)
      workers.ts           - WorkerRepository (CRUD + search + match + merge + audit + links + review-queue)
      links.ts             - client-side mirror of the service's cross-service-link validation
    forms/
      form.svelte.ts       - createForm rune-based store
      LabeledField.svelte
      FieldError.svelte
      FieldRow.svelte
    components/
      SearchBox.svelte
      WorkerGrid.svelte    - SVAR DataGrid binding
      HumanNameInput.svelte
      WorkerForm.svelte
      MatchResultsList.svelte
      LinksPanel.svelte    - cross-service entity_links (list/assert/withdraw)
  routes/
    +layout.svelte         - top nav bar (hamburger on narrow viewports) + theme/locale pickers
    +page.svelte           - dashboard
    api/proxy/[...path]/+server.ts        - BFF reverse proxy (injects the PASETO)
    signin/+page.svelte, +page.server.ts  - per-app magic-link request
    verify/+page.server.ts - consumes the magic link, sets the session cookie
    review/+page.svelte    - stored duplicate-review board (drag-to-decide)
    expiry/+page.svelte    - credential-expiry calendar
    workers/
      +page.svelte         - list
      new/+page.svelte
      match/+page.svelte
      merge/+page.svelte
      [id]/
        +page.svelte       - detail (incl. the LinksPanel)
        edit/+page.svelte
        audit/+page.svelte
tests/
  unit/
    client.test.ts            - ApiClient envelope + error tests
    workers.test.ts           - WorkerRepository wrapping tests
    links-validation.test.ts  - client-side link validation vs the Rust `validate_edge` cases
    i18n.test.ts              - 13-locale key-parity + RTL tests
    layout.test.ts            - app-shell smoke test
  e2e/
    workers.spec.ts        - smoke tests (incl. the links panel)
```

## Lily Design System

The Lily file: dependency resolves to `~/git/lilydesignsystem/lily-design-system/lily-design-system-svelte-headless`. Components import via deep path:

```svelte
import Button from "lily-design-system-svelte-headless/src/lib/components/Button/Button.svelte";
```

The Lily **theme selector** (41 of the 45 shared theme stylesheets under `static/assets/themes/` are wired into the picker's `THEMES` list — `adobe-spectrum`, `mozilla-protocol`, `united-kingdom-government-digital-service`, and `united-states-web-design-system` are present as files but not yet offered) and **locale selector** (13 locales) are wired live in the layout shell — `src/routes/+layout.svelte` imports and renders `ThemePicker` and `LocalePicker`. Lily Headless is available for further primitives as the design system stabilises.

## SVAR DataGrid

`wx-svelte-grid` is GPL-3.0 in its free tier. **If this front-end ships in a commercial product, evaluate the SVAR Pro/Enterprise license before adopting.** See `spec.md §16 Open questions`.

## Status

MVP scaffold. See [`spec.md`](spec/index.md) for the canonical work queue (§13 Tasks).
