# Human Capital Management — SvelteKit front-end

The browser client for the
[Loco JSON API sibling](../human-capital-management-service-with-rust/):
HR, manager, and employee self-service views over the full
employment lifecycle — hiring boards, onboarding, time and leave,
rotas, the employee record and org chart, reviews, training,
succession, payroll runs, and salary benchmarking.

> ⚠️ **Demo software.** Not a production HR system; synthetic data
> only. See [spec/regulatory](../spec/regulatory.md).

**Status: implemented (HCM-T18/T19, 2026-07-18).** svelte-check
clean; 5 vitest + 4 Playwright tests pass (`page.route`-stubbed —
runs without the Rust service). Quick start: `pnpm install &&
pnpm dev` (expects the Loco sibling on :5150; `pnpm test` /
`pnpm exec playwright test`).

## Stack

SvelteKit 2 · Svelte 5 runes · TypeScript strict · SPA mode with a
same-origin BFF proxy (session cookie → short-lived PASETO; no token
in browser JS) · 13-locale i18n · vitest + Playwright
(`page.route`-stubbed).

## Views

| Area | Views |
|---|---|
| Talent acquisition | requisition board, application pipeline, candidate pool, onboarding tracker |
| Workforce | my time, team approvals, leave calendar, department rota |
| HR core | employee profile (masked salary unless entitled), org chart, benefits |
| Development | review panel, goals, training enrollments, succession + gap report |
| Payroll | run screen (draft → calculated → approved), payslips, benchmarking table |
| Self-service | my record, my payslips, my leave, my reviews |
