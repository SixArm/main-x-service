# AGENTS.md — Care Pathway Front-End

Operator UI for the [Care Pathway Service](../care-pathway-service-with-loco):
care-pathway CRUD + matching + name search + merge + audit trail +
recent-activity + cookie-session / BFF auth (PASETO; pivot in progress).

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
4. **Minimal deps.** No data grid / design system — plain inputs + the
   `app.css` utilities.
5. **No envelope.** The service is loco.rs and returns **raw JSON**;
   `src/lib/api/client.ts` is the lean wrapper (get/post/put/delete).

## Layout

```
src/
├── lib/
│   ├── config.ts                 PUBLIC_API_BASE_URL (:5150) + VITE_AUTH_FRONTEND_URL (:5173) + signInUrl()
│   ├── auth.svelte.ts            reactive bearer-token store (mxi_access_token) + captureTokenFromHash
│   ├── api/
│   │   ├── client.ts             lean fetch wrapper (+ ApiError); attaches Bearer from auth store
│   │   ├── types.ts              CarePathway + ConditionCode + CodeSystem + CareSetting + IdentifierScheme + PathwayIdentifier + PathwayRef + ScoredRef + MergeResult + AuditEntry + PathwayEvent
│   │   └── care-pathways.ts      CarePathwayRepository (CRUD + search + checkDuplicates + merge + audit + recentEvents)
│   └── components/CarePathwayForm.svelte
└── routes/
    ├── +layout.svelte / +layout.ts   nav + SPA toggle + session affordance (Sign in / paste / Sign out)
    ├── +page.svelte              list + name-search box + recent-activity toggle
    ├── new/+page.svelte          create
    ├── [pid]/+page.svelte        detail + delete + check-duplicates + merge + audit-trail toggle
    └── [pid]/edit/+page.svelte   edit
```

## API consumption

| UI action | Endpoint |
|---|---|
| List | `GET /api/care-pathways` |
| Search | `GET /api/care-pathways/search?q=` |
| Recent activity | `GET /api/care-pathways/events/recent` → `PathwayEvent[]` |
| Create | `POST /api/care-pathways` |
| Detail | `GET /api/care-pathways/{pid}` |
| Edit | `PUT /api/care-pathways/{pid}` |
| Delete | `DELETE /api/care-pathways/{pid}` |
| Check duplicates | `POST /api/care-pathways/check-duplicates` |
| Merge duplicate | `POST /api/care-pathways/merge` (body `{main_pid, duplicate_pid, reason?}`) |
| Audit trail | `GET /api/care-pathways/{pid}/audit` → `AuditEntry[]` |

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

**Target model (BFF).** Sign-in via the central authentication-service
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
are decommissioned). **Pivot in progress** — the listed `auth.svelte.ts`
client-held-token store keyed on `localStorage["mxi_access_token"]` /
`captureFromLocation` handoff is the current runtime; the BFF + cookie +
CSRF code follow-up is tracked in spec §13.

Configure with `PUBLIC_API_BASE_URL` and `VITE_AUTH_FRONTEND_URL`
(see `.env.example`).
