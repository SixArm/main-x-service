# Human Capital Management front-end — edition spec

Stack-specific specification for the Svelte edition. The
**cross-cutting spec at [`../../spec/`](../../spec/index.md) is the
single source of truth**; this file adds only what is specific to
this edition, and grows topic files (routes, components, i18n) as
HCM-T18/T19 land.

## Stack

SvelteKit 2 · Svelte 5 **runes only** · TypeScript strict · SPA
mode + same-origin BFF proxy · vitest + Playwright. Copy-adapt from
the sibling family front-ends (drift-accepted; the
project-portfolio-management front-end's operational views, i18n,
and `money()` are the closest source). BFF auth per
[../../spec/auth.md](../../spec/auth.md).

## Edition-specific decisions (so far)

- **Personas shape navigation, policy shapes data**: the same routes
  serve employee/manager/HR/payroll; what renders depends on what
  the API returns (masked fields render as first-class masked
  states, not errors).
- **13-locale i18n from the start** with the parity test.
- **Money**: minor units + ISO-4217 in, one locale-aware `money()`
  out; no client-side float arithmetic.
- **No client-held tokens**: mutations via server routes (session
  cookie + CSRF).

## Delivery

This edition is HCM-T18/T19 in
[../../spec/tasks.md](../../spec/tasks.md). Nothing implemented yet.
