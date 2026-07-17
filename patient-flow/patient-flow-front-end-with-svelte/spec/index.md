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

## Delivery

PF-T15 (scaffold + routes) and PF-T16 (tests) in
[../../spec/tasks.md](../../spec/tasks.md), after the service
phases reach the whiteboard reads (PF-T10).
