# Workforce Planning Management front-end — edition spec

Stack-specific specification for the Svelte edition. The
**cross-cutting spec at [`../../spec/`](../../spec/index.md) is the
single source of truth**; this file adds only what is specific to
this edition, and grows topic files (routes, components, i18n) as
WPM-T18/T19 land.

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

## Edition-specific implementation notes (as landed)

- **Copy source**: the patient-flow front-end (BFF proxy, session
  flow, SPA mode) + the PPM front-end's i18n pattern (the catalogue
  has grown to ~125 keys × 13 locales with the T20–T36 areas; the
  parity test pins the exact set — every new key lands in all 13
  locale blocks).
- **Layout**: `src/lib/{i18n.svelte.ts,api/{client,types,wpm}.ts,
components/OrgTree.svelte,server/*}`, routes per the README table;
  the proxy strips cookies, stamps `Accepts-version: 1.0`, and
  carries the session→PASETO exchange seam.
- **Masked money**: `money(null, …)` renders an em dash and the
  employee list/profile render `common.masked` — never a fake 0.
- **Testing**: vitest (money honesty, i18n parity, the API path
  map); Playwright against `vite preview` with `page.route` stubs
  mirroring the service contract (unstubbed calls 404 loudly).

- **Lily Design System** (2026-07-19): the chrome uses the Lily
  **ThemePicker** (45-theme catalogue incl. the NHS design-system
  themes; stylesheets via the `static/assets/themes` symlink; choice
  persisted to `mxi.wpm.theme`) and **LocalePicker** (wired to the i18n
  store, `applyDir` off — the app's own effect owns `lang`/`dir`);
  the **Lily headless** primitives are available as a dependency.

- **The profile as self-service hub** (T23–T36): per-person features
  land as panels on `/employees/[pid]` (wellbeing prompts, pulse,
  notifications, 360s + rater requests, ergonomics, adjustments,
  subject-access, erase) rather than as new top-level routes; new
  top-level routes exist only for genuinely cross-person areas
  (`/wellbeing`, `/privacy`, `/learning`, `/mentorship`).

## Delivery

WPM-T18/T19 **delivered 2026-07-18**; the front-end halves of
WPM-T20–T36 **delivered 2026-07-20 → 2026-07-25** — see
[../../spec/tasks.md](../../spec/tasks.md). Suites: 10 vitest,
9 Playwright.
