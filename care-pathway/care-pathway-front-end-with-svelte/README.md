# care-pathway-front-end-with-svelte

Operator UI for the [Care Pathway Service](../care-pathway-service-with-loco):
care-pathway **CRUD + matching + merge + audit trail + registry
insights + instance tracking (board/Gantt)**.

SvelteKit 2 · Svelte 5 (runes) · SVAR DataGrid/Kanban/Gantt/Filter · Lily Design System · TypeScript strict · SPA.

## Routes

| Route | Purpose |
|---|---|
| `/` | Registry: SVAR DataGrid + FilterBar (client-side name filter) |
| `/new` | Create |
| `/[pid]` | Detail + instances + delete + check-duplicates + merge + audit-trail toggle |
| `/[pid]/edit` | Edit |
| `/insights` | Five read-only registry lenses (directory / coverage / variants / providers / languages) |
| `/board` | Instance Kanban for one pathway (drag = `POST /api/instances/{pid}/status`) |
| `/gantt` | Instance timeline Gantt for one pathway |
| `/sequence` | Intervention sequence Gantt (SVAR) — the selected pathway's interventions as ordered bars on an **ordinal** axis (a sequence view, not a schedule; the model carries order only, no durations or dates) |
| `/signin` | Magic-link sign-in (BFF flow against the auth service) |
| `/verify` | Magic-link verification landing page |

> A list-page name-search box and a "recent activity" event-stream
> toggle shipped in earlier versions and are not present in the current
> registry page; see `spec/index.md` §6.1/§13.

Auth (BFF): **Sign in** via the central authentication-service
magic-link establishes a server-side **cookie session**
(`__Host-mxi_session`, httpOnly); the browser holds **no token** and
talks only to this front-end's own SvelteKit server (BFF), which
exchanges the session for a short-lived **PASETO v4.public** token and
calls the care pathway service server-side. Mutating requests are
CSRF-protected; there is no `localStorage` and no `mxi_access_token`.
Source of truth:
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
(RS256 JWT + JWKS and the `#access_token` fragment handoff
decommissioned). The runtime is the BFF: sign-in via the app's own
`/signin` + `/verify` routes, API calls via the same-origin `/api/proxy`
route, which injects the server-exchanged PASETO.

## Prerequisites

- Node 20+ and pnpm
- A running [Care Pathway Service](../care-pathway-service-with-loco)

## Quick start

```bash
pnpm install
pnpm dev                 # http://localhost:5173
```

## Configuration

| Var | Default | Purpose |
|---|---|---|
| `CARE_PATHWAY_API_URL` | `http://localhost:5150` | Care pathway service REST base URL (read server-side by the BFF proxy; see `src/lib/server/config.ts`). |
| `AUTH_API_URL` | `http://localhost:5150` | Authentication service base URL (BFF-side magic-link + session→PASETO exchange); see [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md). |

## How it works

The care-pathway record body **is** the `care_pathway_matcher::CarePathway`
shape (name, pathway code, provider, care setting, target condition codes
(ICD/SNOMED), interventions, keywords, identifiers, sameAs). The form
edits these; `check-duplicates` posts the current record and lists stored
matches with their scores. The detail page offers a per-row **Merge into
this record** action (`POST /merge`) and a per-pathway audit-trail
toggle (`GET /{pid}/audit`). `/insights` renders five read-only,
server-derived lenses over the registry; `/[pid]`, `/board`, and
`/gantt` surface a pathway's enrolled **instances** (people/subjects on
the pathway), including a drag-to-move Kanban of instance status.

## Testing

```bash
pnpm run check     # svelte-check (strict, 0 errors / 0 warnings)
pnpm run build
pnpm test          # vitest unit suite
pnpm test:e2e      # Playwright smoke (runs against `vite preview`)
```

## License

Dual-licensed under MIT OR Apache-2.0.
