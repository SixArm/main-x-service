# Patient Flow front-end — edition spec

Stack-specific specification for the Svelte edition. The
**cross-cutting spec at [`../../spec/`](../../spec/index.md) is the
single source of truth**; this file adds only what is specific to
this edition. PF-T15/T16 landed 2026-07-18 (PF-T15a followed
2026-07-19) with the edition's whole scope fitting in this one file,
so it stayed `index.md` rather than splitting into topic files.
<!-- PRO-H8, 2026-08-28: this paragraph previously promised to "grow
topic files (routes, components, ui-conventions, i18n) as PF-T15/T16
land"; per spec/tasks.md both landed in July 2026 and no topic-file
split ever followed — the split was never mechanically small enough
to be worth doing for one page's worth of content, so the promise is
corrected to match reality rather than fulfilled. Corrected during
the PRO-H8 professionalization sweep. -->

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
  not a grid. Runtime dependencies beyond the framework are the Lily
  Design System (headless primitives + the ThemePicker / LocalePicker
  helpers, `file:` deps on the sibling design-system repo, 2026-07-19)
  and, since the same day's follow-on (PF-T15a), a set of
  `@svar-ui/svelte-*` packages: **`svelte-grid`** + **`svelte-filter`**
  power the `/wards` index (FilterBar over code/ward/kind/specialty),
  and **`svelte-calendar`** powers the `/edd` month view.
  `svelte-kanban`, `svelte-gantt`, and `svelte-filemanager` are
  installed as candidate-feature seams with no route using them yet.
  The theme stylesheets are served from `static/assets/themes` (a
  symlink to the shared design-system themes). LocalePicker owns
  `lang`/`dir` (RTL for ar/ur) and persists the choice — there is
  no translation catalogue yet, so it is the i18n-ready seam.
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
board polling) **delivered 2026-07-18**; PF-T15a (Lily theme/locale
chrome, the `/wards` and `/edd` routes) followed **2026-07-19** — see
[../../spec/tasks.md](../../spec/tasks.md). Open queue: none; later
ideas live in the cross-cutting [roadmap](../../spec/roadmap.md).
- [x] **PF-T19: `/verify` crashed with a raw 500 when the authentication service was unreachable.** *(resolved 2026-09-06.)* `src/routes/verify/+page.server.ts` called `await verifyMagicLink(fetch, token)` with no `try`/`catch`. A network-level failure (the authentication service unreachable, timed out, connection reset) makes `fetch` throw rather than resolve — uncaught, that propagated out of `load` and SvelteKit rendered its generic 500 error page instead of this route's own friendly UI. The same bug class was found and fixed first in `place-front-end-with-svelte` (T-26) and `thing-front-end-with-svelte` (T-23); ported here.
  - **Resolved.** A `try`/`catch` around the call, a new `"serviceUnavailable"` error variant, and its message in `+page.svelte`.
  - **Acceptance:** `tests/unit/verify.test.ts` (new) unit-tests the `load` function directly — pinning `missingToken`, the new `serviceUnavailable` (fetch rejects), and `invalidToken` (non-ok response) branches — verified to fail with the `try`/`catch` reverted and pass with it restored. Three-part change: spec (here) + code + test.
