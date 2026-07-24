# AGENTS.md — working agreements

A pocket guide for human and AI collaborators working in this
subproject. Read this **before** opening a PR.

## What this project is

A **SvelteKit browser client** for the
[Loco JSON API sibling](../workforce-planning-management-service-with-rust/):
requisition and application boards, the onboarding tracker, team
calendar and rota, employee profiles and the org chart, review and
training panels, the payroll run screen, benchmarking, and the HR
dashboard — plus the employee **self-service** views (my record, my
payslips, my leave). The Svelte app owns no data; every page
round-trips through the API.

> ⚠️ Demo software, not a production HR system. See
> [regulatory](../spec/regulatory.md).

## Ground rules

1. **Spec first.** The cross-cutting spec at
   [`../spec/`](../spec/index.md) is the single source of truth —
   especially [auth](../spec/auth.md) (personas + masking) and
   [architecture](../spec/architecture.md). Task queue:
   [`../spec/tasks.md`](../spec/tasks.md) (WPM-T18/T19).
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
