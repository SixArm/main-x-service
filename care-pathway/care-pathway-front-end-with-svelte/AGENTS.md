# AGENTS.md — Care Pathway Front-End

Operator UI for the [Care Pathway Service](../care-pathway-service-with-loco):
care-pathway CRUD + matching + merge + audit trail + registry insights +
instance tracking (Kanban board / Gantt) + cookie-session / BFF auth
(PASETO). Name search and a recent-activity view exist as repository
methods but are not wired to any route — see spec §6.1/§13.

> Read [`spec/index.md`](./spec/index.md) first — the living spec.

## What this is

A SvelteKit 2 / Svelte 5 (runes) **SPA**. It calls the care-pathway
service REST API, whose request/response body is the
`care_pathway_matcher::CarePathway` shape itself.

## Ground rules

1. **Runes only** (`$state`/`$derived`/`$effect`/`$props`/`$bindable`).
   No `export let`, no `$:`, events are callback props.
2. **SPA.** `+layout.ts` sets `ssr = false` / `prerender = false`.
3. **TypeScript strict** (`noUncheckedIndexedAccess`).
4. **Deps in real use.** All four SVAR components are used: DataGrid +
   FilterBar (`/` registry), Kanban (`/board` instance lifecycle), and
   Gantt (`/gantt` instance timeline + `/sequence` intervention sequence); plus Lily
   `ThemePicker`/`LocalePicker` (in the layout). Forms remain plain
   inputs + the `app.css` utilities.
5. **No envelope.** The service is loco.rs and returns **raw JSON**;
   `src/lib/api/client.ts` is the lean wrapper (get/post/put/delete).

## Layout

```
src/
├── hooks.server.ts               BFF session handling (reads the httpOnly session cookie)
├── lib/
│   ├── config.ts                 API_BASE_URL → same-origin BFF proxy (/api/proxy)
│   ├── api/
│   │   ├── client.ts             lean fetch wrapper (+ ApiError); no browser-held bearer — the BFF proxy injects the PASETO server-side
│   │   ├── types.ts              CarePathway + ConditionCode + CodeSystem + CareSetting + IdentifierScheme + PathwayIdentifier + PathwayRef + ScoredRef + MergeResult + AuditEntry + PathwayEvent + PathwayInstance/InstanceStatus/Urgency/InstanceDetail + the five insight response types
│   │   ├── tba.ts                TbaRepository (timeline / instance + cohort analysis / constraints / standards / flow / segment + clock recording) + the presentation helpers
│   │   └── care-pathways.ts      CarePathwayRepository (CRUD + search + checkDuplicates + merge + audit + recentEvents + insights{Directory,Coverage,Variants,Providers,Languages} + listInstances + getInstance + setInstanceStatus + caseload)
│   ├── server/                   BFF-only (never bundled to the browser): auth.ts (magic-link + session→PASETO exchange), session.ts (cookie), config.ts (CARE_PATHWAY_API_URL / AUTH_API_URL)
│   └── components/CarePathwayForm.svelte · JourneyTimeline.svelte (the timeline wall)
└── routes/
    ├── +layout.svelte / +layout.ts / +layout.server.ts   nav + session panel
    ├── +page.svelte              registry grid (SVAR DataGrid + FilterBar); rows link to /{pid}
    ├── signin/ · verify/         per-app magic-link sign-in (BFF server routes)
    ├── api/proxy/[...path]/+server.ts   BFF proxy → care-pathway service (injects the PASETO bearer)
    ├── new/+page.svelte          create
    ├── [pid]/+page.svelte        detail + instances + delete + check-duplicates + merge + audit-trail toggle
    ├── [pid]/edit/+page.svelte   edit
    ├── insights/+page.svelte     the five registry lenses as tables (directory / coverage / variants / providers / languages)
    ├── board/+page.svelte        instance Kanban (one pathway; drag = POST /api/instances/{pid}/status)
    ├── gantt/+page.svelte        instance timeline Gantt (one pathway; enrolled_on → next_review/closed/today)
    ├── sequence/+page.svelte     intervention-sequence Gantt (a pathway template's interventions)
    └── time/+page.svelte         time-based analysis: cohort ratio + NHS access-standard score + constraints + Little's Law, and one journey's timeline wall
```

## API consumption

| UI action | Endpoint |
|---|---|
| List / registry grid | `GET /api/care-pathways` |
| Search (repository method, unwired — see spec §6.1/§13) | `GET /api/care-pathways/search?q=` |
| Recent activity (repository method, unwired — see spec §6.1/§13) | `GET /api/care-pathways/events/recent` → `PathwayEvent[]` |
| Create | `POST /api/care-pathways` |
| Detail | `GET /api/care-pathways/{pid}` |
| Edit | `PUT /api/care-pathways/{pid}` |
| Delete | `DELETE /api/care-pathways/{pid}` |
| Check duplicates | `POST /api/care-pathways/check-duplicates` |
| Merge duplicate | `POST /api/care-pathways/merge` (body `{main_pid, duplicate_pid, reason?}`) |
| Audit trail | `GET /api/care-pathways/{pid}/audit` → `AuditEntry[]` |
| Insights (5 lenses) | `GET /api/care-pathways/insights/{directory,coverage,variants,providers,languages}` |
| Pathway instances | `GET /api/care-pathways/{pid}/instances` → `PathwayInstance[]` |
| Instance detail | `GET /api/instances/{pid}` → `{instance,steps,team,events,measures}` |
| Instance status move | `POST /api/instances/{pid}/status` (body `{to}`) |
| Caseload (board context) | `GET /api/instances/caseload` |
| Time-based analysis — cohort (`/time`) | `GET /api/care-pathways/{pid}/time-analysis?standard=` · `GET /api/care-pathways/{pid}/constraints` |
| Time-based analysis — journey (`/time`) | `GET /api/instances/{pid}/timeline` · `GET /api/instances/{pid}/time-analysis` |
| Access-standard catalogue + flow | `GET /api/instances/time-standards` · `GET /api/instances/flow?window_days=&pathway=` |
| Segment / clock recording (client methods; not yet wired to a route) | `POST /api/instances/{pid}/segments` · `POST /api/instances/{pid}/clock` |

## Commands

```bash
pnpm install
pnpm dev          # http://localhost:5173
pnpm run check    # svelte-check (strict; 0/0 expected)
pnpm run build
pnpm test         # vitest unit suite
pnpm test:e2e     # Playwright smoke (runs against `vite preview`)
```

## Auth

**BFF model (current).** Sign-in via the central authentication-service
magic-link establishes a server-side **cookie session**
(`__Host-mxi_session`, httpOnly). The browser holds **no token** and
talks only to this front-end's own SvelteKit server (BFF), which
exchanges the session for a short-lived **PASETO v4.public** token and
calls the care pathway service server-side. Mutating requests are
CSRF-protected; there is no `localStorage` and no `mxi_access_token`.
Service-side blanket enforcement (`CARE_PATHWAY_REQUIRE_AUTH`) is off by
default and unchanged in semantics — only the credential changes.

Source of truth:
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
(RS256 JWT + JWKS and the cross-origin `#access_token` fragment handoff
are decommissioned). The former `auth.svelte.ts` client-held-token store
keyed on `localStorage["mxi_access_token"]` / `captureFromLocation`
handoff has been removed — the runtime is the BFF: `src/lib/server/`
exchanges the session for the PASETO and the `/api/proxy` route calls
the service server-side.

Configure the BFF's upstream URLs with the server-side env vars
`CARE_PATHWAY_API_URL` (care-pathway service) and `AUTH_API_URL`
(authentication service) — see `src/lib/server/config.ts`; both default
to `http://localhost:5150`.
