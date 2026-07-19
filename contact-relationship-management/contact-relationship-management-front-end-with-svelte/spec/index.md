# Contact Relationship Management front-end — edition spec

Stack-specific specification for the Svelte edition. The
**cross-cutting spec at [`../../spec/`](../../spec/index.md) is the
single source of truth**; this file adds only what is specific to
this edition, and grows topic files (routes, components, i18n) as
CRM-T17/T18 land.

## Stack

SvelteKit 2 · Svelte 5 **runes only** · TypeScript strict · SPA
mode + same-origin BFF proxy · vitest + Playwright. Copy-adapt from
the sibling family front-ends (drift-accepted; the
project-portfolio-management front-end's Kanban board, i18n, and
`money()` are the closest source). BFF auth per
[../../spec/auth.md](../../spec/auth.md).

## Edition-specific decisions (so far)

- **Kanban board** is the PPM task-board pattern adapted to deals
  (stage columns, drag = stage-move mutation, `kanban_position`).
- **SLA countdowns** tick client-side from the served
  `*_due_at` deadlines — display only; breach truth comes from the
  API.
- **Masked amounts** (deal values, forecasts, ROI) render as a
  first-class masked state, never zeros or errors.
- **13-locale i18n from the start** with the parity test.
- **No client-held tokens**: mutations via server routes (session
  cookie + CSRF).

## Edition-specific implementation notes (as landed)

- **Copy source**: the HCM front-end (BFF proxy, session seam, SPA
  mode, i18n pattern; 45-key catalogue here).
- **Deal board**: stage columns from the pipeline's stage rows;
  the forward-move button drives `POST /deals/{pid}/stage` (a lost
  target carries a reason); the forecast strip re-reads
  `GET /forecast` after every move so the number is never client
  math.
- **Honest KPIs**: win rate renders `value` with its
  numerator/denominator and shows a no-data state on `null`; ROI
  likewise; masked/absent money renders an em dash.
- **Testing**: vitest (money honesty, i18n parity, API path map);
  Playwright over `page.route` stubs mirroring the service contract
  (unstubbed = 404-loud).

- **Lily Design System** (2026-07-19): the chrome uses the Lily
  **ThemeSelect** (45-theme catalogue incl. the NHS design-system
  themes; stylesheets via the `static/assets/themes` symlink; choice
  persisted to `mxi.crm.theme`) and **LocaleSelect** (wired to the i18n
  store, `applyDir` off — the app's own effect owns `lang`/`dir`);
  the **Lily headless** primitives are available as a dependency.

## Delivery

CRM-T17/T18 **delivered 2026-07-18** — see
[../../spec/tasks.md](../../spec/tasks.md).
