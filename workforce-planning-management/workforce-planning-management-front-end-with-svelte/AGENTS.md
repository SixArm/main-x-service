# AGENTS.md — working agreements

A pocket guide for human and AI collaborators working in this
subproject. Read this **before** opening a PR.

## What this project is

A **SvelteKit browser client** for the
[Loco JSON API sibling](../workforce-planning-management-service-with-rust/):
requisition and application boards, the onboarding tracker, the rota
with working-time and ergonomic-issue panels, employee profiles and
the org chart, review and training panels, `/wellbeing` (entitlement
rules, uptake, pulse results), `/learning` and `/mentorship`,
`/privacy` (retention report + sweep), the payroll run screen,
benchmarking, and the HR dashboard. The **employee profile is the
self-service hub**: my record / payslips / leave / reviews, wellbeing
prompts, the pulse card, notifications, 360° panels and "my 360
requests", the ergonomics checklist, reasonable adjustments,
subject-access download, and the erase action. The Svelte app owns no
data; every page round-trips through the API, and masked fields
render as first-class masked states, never errors or fake zeros.

> ⚠️ Demo software, not a production HR system. See
> [regulatory](../spec/regulatory.md).

## Ground rules

1. **Spec first.** The cross-cutting spec at
   [`../spec/`](../spec/index.md) is the single source of truth —
   especially [auth](../spec/auth.md) (personas + masking) and
   [architecture](../spec/architecture.md). Task queue:
   [`../spec/tasks.md`](../spec/tasks.md) (WPM-T18/T19 + the
   front-end halves of T20–T36).
2. **Family front-end conventions.** SvelteKit 2, **Svelte 5 runes
   only** (no legacy stores/`$:`), TypeScript strict, SPA mode.
   Drift between front-ends is accepted — copy-adapt from a sibling
   (the project-portfolio-management front-end's operational views
   and i18n are the closest source); no shared package.
3. **BFF auth.** The SvelteKit server holds the cookie session and
   exchanges it for short-lived PASETO tokens; **no token in browser
   JS, no localStorage credentials**. Mutations go through server
   routes with CSRF protection.
4. **Masking honesty.** Salary, payslip amounts, and review content
   may arrive masked; render the masked state as a first-class path,
   never "•••" as an error.
5. **Money display.** Amounts arrive as minor units + currency; one
   `money()` formatter, locale-aware; never float arithmetic in the
   client.
6. **13-locale i18n from the start** (the PPM lesson) with the
   full-coverage parity test.
7. **Tests.** vitest for the API client path map + `money()` + core
   components; Playwright over a `page.route`-stubbed API mirroring
   the endpoint contract (unstubbed = 404-loud).

## Running

```bash
pnpm install
pnpm dev           # expects the Loco sibling (stub mode) on its default port
pnpm test          # vitest
pnpm exec playwright test
```
