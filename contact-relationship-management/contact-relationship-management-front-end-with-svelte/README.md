# Contact Relationship Management — SvelteKit front-end

The browser client for the
[Loco JSON API sibling](../contact-relationship-management-service-with-rust/):
sales, marketing, and support views over the relationship
lifecycle — lead queue, deal Kanban, forecasts, campaign funnels,
nurture sequences, ticket queue with SLA countdowns, knowledge
base, and KPI dashboards.

> ⚠️ **Demo software.** Not a production CRM; synthetic data only.
> See [spec/regulatory](../spec/regulatory.md).

**Status: not started.** The cross-cutting specification landed
2026-07-18; this edition is CRM-T17/T18 in
[../spec/tasks.md](../spec/tasks.md).

## Planned stack

SvelteKit 2 · Svelte 5 runes · TypeScript strict · SPA mode with a
same-origin BFF proxy (session cookie → short-lived PASETO; no
token in browser JS) · 13-locale i18n · vitest + Playwright
(`page.route`-stubbed).

## Planned views

| Area | Views |
|---|---|
| Sales | contact/account timeline, lead queue (score + breakdown), deal Kanban, forecast table |
| Marketing | campaign list + funnel + ROI, segment preview, nurture editor, consent history |
| Support | ticket queue with SLA countdowns, ticket detail, KB editor + search |
| Analytics | dashboards: win rate, pipeline by stage, SLA health, CLV, activity feed |
