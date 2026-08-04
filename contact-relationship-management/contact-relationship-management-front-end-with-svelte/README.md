# Contact Relationship Management — SvelteKit front-end

The browser client for the
[Loco JSON API sibling](../contact-relationship-management-service-with-rust/):
sales, marketing, and support views over the relationship
lifecycle — lead queue, deal Kanban, forecasts, campaign funnels,
nurture sequences, ticket queue with SLA countdowns, knowledge
base, and KPI dashboards.

> ⚠️ **Demo software.** Not a production CRM; synthetic data only.
> See [spec/regulatory](../spec/regulatory.md).

**Status: implemented (CRM-T17/T18, T19, T20; 2026-07-18 through
2026-07-20).** svelte-check clean; 5 vitest + 12 Playwright tests
pass (`page.route`-stubbed — runs without the Rust service). Quick
start: `pnpm install && pnpm dev` (expects the Loco sibling on
:5150).

## Environment variables

Server-side only (`src/lib/server/config.ts`; never reach the browser
bundle) — see [`.env.example`](.env.example).

| Variable | Default | Purpose |
|---|---|---|
| `CRM_API_URL` | `http://localhost:5150` | The Loco sibling's base URL |
| `AUTH_API_URL` | `http://localhost:5150` | The authentication service (session → PASETO exchange, magic-link) |

## Stack

SvelteKit 2 · Svelte 5 runes · TypeScript strict · SPA mode with a
same-origin BFF proxy (session cookie → short-lived PASETO; no
token in browser JS) · 13-locale i18n · Lily Design System (headless + ThemePicker + LocalePicker) · vitest + Playwright
(`page.route`-stubbed).

## Views

| Area | Views |
|---|---|
| Sales | contact/account timeline, lead queue + `/leads/board` (score + breakdown), deal `/deals` Kanban + pipeline selector + funnel strip, forecast table |
| Marketing | campaign list + funnel + ROI, segment preview, nurture editor, consent history |
| Support | ticket queue + `/tickets/board` with SLA countdowns, ticket detail, KB editor + search |
| Analytics | dashboards: win rate, pipeline by stage, SLA health, CLV, activity feed |
| Insights (CRM-T19) | `/followups` (overdue aging + calendar), `/executive` (period pack + hygiene findings + forecast trend), `/dpo` (consent coverage + duplicate hygiene) |
| Engagement (CRM-T20) | `/engagement` (cadence, workload, member health), `/partners` (stakeholder register + grid, partnership register, membership renewals) |
