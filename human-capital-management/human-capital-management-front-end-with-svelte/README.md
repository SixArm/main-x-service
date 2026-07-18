# Human Capital Management — SvelteKit front-end

The browser client for the
[Loco JSON API sibling](../human-capital-management-service-with-rust/):
HR, manager, and employee self-service views over the full
employment lifecycle — hiring boards, onboarding, time and leave,
rotas, the employee record and org chart, reviews, training,
succession, payroll runs, and salary benchmarking.

> ⚠️ **Demo software.** Not a production HR system; synthetic data
> only. See [spec/regulatory](../spec/regulatory.md).

**Status: not started.** The cross-cutting specification landed
2026-07-18; this edition is HCM-T18/T19 in
[../spec/tasks.md](../spec/tasks.md).

## Planned stack

SvelteKit 2 · Svelte 5 runes · TypeScript strict · SPA mode with a
same-origin BFF proxy (session cookie → short-lived PASETO; no token
in browser JS) · 13-locale i18n · vitest + Playwright
(`page.route`-stubbed).

## Planned views

| Area | Views |
|---|---|
| Talent acquisition | requisition board, application pipeline, candidate pool, onboarding tracker |
| Workforce | my time, team approvals, leave calendar, department rota |
| HR core | employee profile (masked salary unless entitled), org chart, benefits |
| Development | review panel, goals, training enrollments, succession + gap report |
| Payroll | run screen (draft → calculated → approved), payslips, benchmarking table |
| Self-service | my record, my payslips, my leave, my reviews |
