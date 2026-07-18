# Contact Relationship Management — SvelteKit front-end

The browser client for the
[Loco JSON API sibling](../contact-relationship-management-service-with-rust/):
sales, marketing, and support views over the relationship
lifecycle — lead queue, deal Kanban, forecasts, campaign funnels,
nurture sequences, ticket queue with SLA countdowns, knowledge
base, and KPI dashboards.

> ⚠️ **Demo software.** Not a production CRM; synthetic data only.
> See [spec/regulatory](../spec/regulatory.md).

**Status: implemented (CRM-T17/T18, 2026-07-18).** svelte-check
clean; 5 vitest + 4 Playwright tests pass (`page.route`-stubbed —
runs without the Rust service). Quick start: `pnpm install &&
pnpm dev` (expects the Loco sibling on :5150).

## Stack

SvelteKit 2 · Svelte 5 runes · TypeScript strict · SPA mode with a
same-origin BFF proxy (session cookie → short-lived PASETO; no
token in browser JS) · 13-locale i18n · vitest + Playwright
(`page.route`-stubbed).

## Views

| Area | Views |
|---|---|
| Sales | contact/account timeline, lead queue (score + breakdown), deal Kanban, forecast table |
| Marketing | campaign list + funnel + ROI, segment preview, nurture editor, consent history |
| Support | ticket queue with SLA countdowns, ticket detail, KB editor + search |
| Analytics | dashboards: win rate, pipeline by stage, SLA health, CLV, activity feed |
