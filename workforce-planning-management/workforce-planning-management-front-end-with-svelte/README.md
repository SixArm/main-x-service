# Workforce Planning Management — SvelteKit front-end

The browser client for the
[Loco JSON API sibling](../workforce-planning-management-service-with-rust/):
HR, manager, and employee self-service views over the full
employment lifecycle — hiring boards, onboarding, time and leave,
rotas with working-time and ergonomic-issue panels, the employee
record and org chart, wellbeing prompts and the anonymous pulse,
reviews and 360° appraisals, notifications, reasonable adjustments,
training and learning, succession, payroll runs, salary
benchmarking, and the privacy/retention admin area.

> ⚠️ **Demo software.** Not a production HR system; synthetic data
> only. See [spec/regulatory](../spec/regulatory.md).

**Status: implemented (WPM-T18–T36, 2026-07-18 → 2026-07-25).**
svelte-check clean; 10 vitest + 9 Playwright specs pass
(`page.route`-stubbed — runs without the Rust service). Quick
start: `pnpm install && pnpm dev` (expects the Loco sibling on
:5150; `pnpm test` / `pnpm exec playwright test`).

## Stack

SvelteKit 2 · Svelte 5 runes · TypeScript strict · SPA mode with a
same-origin BFF proxy (session cookie → short-lived PASETO; no token
in browser JS) · 13-locale i18n · Lily Design System (headless + ThemePicker + LocalePicker) · vitest + Playwright
(`page.route`-stubbed).

## Views

| Area               | Views                                                                       |
| ------------------ | --------------------------------------------------------------------------- |
| Talent acquisition | requisition board (SVAR Kanban), application pipeline, candidate pool, onboarding tracker |
| Workforce          | team approvals, department rota, working-time flags, ergonomic issues       |
| HR core            | employee list (SVAR grid) + profile (masked salary unless entitled), org chart, benefits |
| Wellbeing          | `/wellbeing`: entitlement rules + aggregate uptake & conversion + pulse results; `/privacy`: retention report + sweep |
| Development        | review panel, `/learning` (skills matrix, analytics, paths), `/mentorship`, succession + gap report |
| Payroll            | run screen (draft → calculated → approved), payslips, benchmarking table    |
| Self-service (profile) | my record + payslips + leave + reviews, wellbeing prompts, pulse card, notifications, my 360 requests, 360 panel, ergonomics checklist, reasonable adjustments, "Download my data", erase (terminated only) |

The employee **profile page is the self-service hub** — most
per-person features surface there as panels; masked fields render as
first-class masked states (an em dash / "Hidden"), never as errors
or fake zeros.
