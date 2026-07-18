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

## Delivery

This edition is CRM-T17/T18 in
[../../spec/tasks.md](../../spec/tasks.md). Nothing implemented yet.
