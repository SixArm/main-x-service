# Patient Flow front-end — edition spec

Stack-specific specification for the Svelte edition. The
**cross-cutting spec at [`../../spec/`](../../spec/index.md) is the
single source of truth**; this file adds only what is specific to
this edition, and grows topic files (routes, components,
ui-conventions, i18n) as PF-T15/T16 land.

## Stack

SvelteKit 2 · Svelte 5 **runes only** · TypeScript strict · vitest +
Playwright. Copy-adapt from the sibling family front-ends
(drift-accepted; the portfolio front-end's operational views are the
closest source). BFF auth per [../../spec/auth.md](../../spec/auth.md).

## Edition-specific decisions (so far)

- **Kiosk/touch mode** is a route variant
  (`/wards/{pid}/kiosk`) — no separate app: chrome-less layout,
  large tap targets, optional masked rendering (no patient names)
  for screens visible to visitors, auto-refresh with visible
  `as_of`.
- **Polling**: ETag/`updated_since` polling in v1 (interval
  configurable per deployment); SSE when the service roadmap item
  lands.
- **Bed-card component** renders the full contract in
  [../../spec/whiteboard.md](../../spec/whiteboard.md); its state ×
  flags matrix is the primary vitest surface.
- **No client-held tokens**: mutations via server routes (session
  cookie + CSRF); read loads server-side where masking applies.

## Edition-specific implementation notes (as landed)

- **SPA mode** (`ssr = false`, family convention): every route loads
  client-side through the same-origin BFF proxy
  (`src/routes/api/proxy/[...path]/+server.ts`), which strips
  cookies, stamps `Accepts-version: 1.0`, and carries the marked
  PF-T18 seam for the session→PASETO exchange.
- **Copy source**: the case front-end (dependency-light, no data
  grid) rather than portfolio — a whiteboard is custom CSS cards,
  not a grid. Zero runtime dependencies.
- **Layout**: `src/lib/api/{types,client,flow}.ts`,
  `src/lib/components/{BedCard,WardBoard}.svelte`, routes per the
  README table; kiosk mode is a body-class + CSS variant, not a
  separate app.
- **Testing**: vitest + jsdom for the `BedCard` matrix;
  Playwright against `vite preview` with the API stubbed via
  `page.route` mirroring the endpoint contract (unstubbed calls 404
  loudly, so contract drift fails the suite).

## Delivery

PF-T15/T16 and PF-T18 (BFF session + PASETO exchange; ETag-aware
board polling) **delivered 2026-07-18** — see
[../../spec/tasks.md](../../spec/tasks.md). Open queue: none; later
ideas live in the cross-cutting [roadmap](../../spec/roadmap.md).
