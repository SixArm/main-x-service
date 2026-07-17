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

**Status: specification round — implementation queued** (PF-T15/T16
in [../spec/tasks.md](../spec/tasks.md), after the service phases).

## Routes (target)

| Route | View |
|---|---|
| `/wards/{pid}/whiteboard` | ward whiteboard — bed cards, tap actions |
| `/wards/{pid}/kiosk` | same board, chrome-less touchscreen mode (large targets, masked option) |
| `/stays/{pid}` | stay detail — journey, Red2Green run, flags, audit slice |
| `/at-a-glance` | per-ward + site capacity tiles |
| `/bed-requests` | demand queue + allocation |
| `/locate` | patient locate search |
| `/audits` | ward-scoped handover trail |

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
