# Patient Flow — Svelte front-end

A SvelteKit application providing the **digital ward whiteboard**
and flow-operations UI for the
[Loco JSON API sibling](../patient-flow-service-with-rust/):
interactive bed cards on ward touchscreens, hospital-at-a-glance,
bed-request board, patient locate, and clinical-handover audit
views. The app owns no data — every view round-trips through the
API.

> ⚠️ **Demo application.** Not a regulated medical record; synthetic
> data only. See [spec/regulatory](../spec/regulatory.md).

**Status: implemented (PF-T15/T16, 2026-07-18).** SPA mode with a
same-origin BFF proxy; `svelte-check` clean; 22 vitest component
tests + 7 Playwright e2e specs (API stubbed via `page.route`, no
Rust service needed). The BFF session flow (PF-T18: `/signin`
magic-link, `/verify` cookie exchange, `/signout`, proxy PASETO
injection) is wired and inert until auth activation.

## Quick start

```bash
npm install
npm run dev            # UI on :5173; proxies /api/proxy/* to the service
# point at a running service (default http://localhost:5150):
PATIENT_FLOW_API_URL=http://localhost:5150 npm run dev
npm test               # vitest (BedCard matrix)
npx playwright test    # e2e (stubbed API — no backend needed)
```

Configure with `PATIENT_FLOW_API_URL` (the service) and
`AUTH_API_URL` (the authentication service) — both server-side only,
read by `src/lib/server/config.ts`; see `.env.example`.

## Routes

| Route | View |
|---|---|
| `/wards/{pid}/whiteboard` | ward whiteboard — bed cards, tap actions |
| `/wards/{pid}/kiosk` | same board, chrome-less touchscreen mode (large targets, masked option) |
| `/stays/{pid}` | stay detail — journey, Red2Green run, flags, audit slice |
| `/at-a-glance` | per-ward + site capacity tiles |
| `/bed-requests` | demand queue + allocation |
| `/edd` | EDD / discharge-readiness calendar (SVAR Calendar, month view) — read-only overview of expected discharge dates |
| `/locate` | patient locate search |
| `/audits` | ward-scoped handover trail |
| `/signin` | magic-link sign-in (BFF flow) |
| `/signout` | sign out — clears the session |
| `/verify` | magic-link verification → cookie session |

## Stack

SvelteKit 2 · Svelte 5 runes · TypeScript strict · BFF auth (cookie
session → server-side PASETO exchange; no token in the browser).
Copy-adapt conventions from the sibling family front-ends
(drift-accepted).

## Docs

- [index.md](index.md) — documentation index
- [../spec/](../spec/index.md) — cross-cutting specification
  (see [whiteboard](../spec/whiteboard.md) for the bed-card contract)
- [spec/](spec/index.md) — this edition's stack-specific spec
- [AGENTS.md](AGENTS.md) — working agreements
