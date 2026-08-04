# AGENTS.md — Case Front-End

Operator UI for the [Case Service](../case-service-with-loco):
case CRUD + matching (governmental case management).

> Read [`spec/index.md`](./spec/index.md) first — the living spec.

## What this is

A SvelteKit 2 / Svelte 5 (runes) **SPA**. It calls the case
service REST API, whose request/response body is the
`case_matcher::Case` shape itself.

## Ground rules

1. **Runes only** (`$state`/`$derived`/`$effect`/`$props`/`$bindable`).
   No `export let`, no `$:`, events are callback props.
2. **SPA.** `+layout.ts` sets `ssr = false` / `prerender = false`.
3. **TypeScript strict** (`noUncheckedIndexedAccess`).
4. **Deps in real use.** SVAR DataGrid + FilterBar (`/cases` index),
   SVAR Kanban (`/board`), and Lily `ThemePicker`/`LocalePicker` (in the
   layout) are used dependencies; forms remain plain inputs + the
   `app.css` utilities.
5. **No envelope.** The service is loco.rs and returns **raw JSON**;
   `src/lib/api/client.ts` is the lean wrapper (get/post/put/delete).
6. **Auth (BFF + cookie session).** The browser holds **no token** — no
   `localStorage`, no `mxi_access_token`, no URL-fragment handoff. This
   app has its **own** `/signin` + `/verify` magic-link flow — there is
   no cross-origin redirect to the auth front-end; the top-bar **Sign
   in** link points at this app's own `/signin`. Verifying the link sets
   an httpOnly `__Host-mxi_session` cookie. This app's SvelteKit server
   acts as a **Backend-For-Frontend**: it holds the session, exchanges it
   for a short-lived **PASETO v4.public** token, and calls the case
   service server-side; mutating calls carry a CSRF token. See
   [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
   (source of truth; RS256/JWKS decommissioned).

   > Auth pivot landed: `src/hooks.server.ts` +
   > `src/lib/server/{session,auth,config}.ts` implement the BFF
   > (session cookie → PASETO exchange → server-side proxy at
   > `/api/proxy/[...path]`); there is no client-held token.

## Layout

```
src/
├── lib/
│   ├── config.ts                 API_BASE_URL = same-origin BFF proxy (location.origin + /api/proxy)
│   ├── server/                   BFF: session cookie helpers + session→PASETO exchange + service config (CASE_API_URL, AUTH_API_URL)
│   ├── api/
│   │   ├── client.ts             lean fetch wrapper (+ ApiError; optional per-request token; getPage())
│   │   ├── types.ts              Case + CaseIdentifier + CaseType + CaseStatus + Priority + IdentifierScheme + CaseRef + ScoredRef + EntityLink + Merge*
│   │   └── cases.ts              CaseRepository (CRUD + checkDuplicates + merge/recentMerges + listLinks/createLink/deleteLink)
│   └── components/CaseForm.svelte, LinksPanel.svelte, merge-validation.ts, link-validation.ts
└── routes/
    ├── +layout.svelte / +layout.ts   top-bar nav (hamburger on narrow) + SPA toggle + SSO sign-in/out
    ├── +page.svelte               list
    ├── cases/+page.svelte         SVAR DataGrid + FilterBar index (client-side filtering)
    ├── board/+page.svelte         SVAR Kanban — drag a card to change status
    ├── new/+page.svelte           create
    ├── [pid]/+page.svelte         detail + delete + check-duplicates + "subject of this case" links panel
    ├── [pid]/edit/+page.svelte    edit
    ├── merge/+page.svelte         merge a duplicate into a survivor + recent merge history
    ├── signin / verify            this app's own magic-link request/verify (BFF session establishment)
    └── api/proxy/[...path]        BFF proxy → case service (attaches the PASETO server-side)

tests/
├── unit/                         vitest: client / cases / case-form / i18n / layout / link-validation / merge-validation
└── e2e/smoke.spec.ts             Playwright: 8 tests over the routes above + check-duplicates self-exclusion
```

## API consumption

| UI action | Endpoint |
|---|---|
| List | `GET /api/cases` |
| Create | `POST /api/cases` |
| Detail | `GET /api/cases/{pid}` |
| Edit | `PUT /api/cases/{pid}` |
| Delete | `DELETE /api/cases/{pid}` |
| Check duplicates | `POST /api/cases/check-duplicates` |
| Merge | `POST /api/cases/merge` |
| Recent merges | `GET /api/cases/merges/recent` |
| Links (list / assert / withdraw) | `GET`/`POST /api/cases/{pid}/links`, `DELETE /api/cases/{pid}/links/{id}` |

## Commands

```bash
pnpm install
pnpm dev          # http://localhost:5173
pnpm run check    # svelte-check (strict; 0/0 expected)
pnpm test         # vitest unit tests
pnpm test:e2e     # Playwright smoke tests
pnpm run build
```

Configure with `CASE_API_URL` (case service) and `AUTH_API_URL`
(authentication service) — both server-side only, read by
`src/lib/server/config.ts`; see `.env.example`.
