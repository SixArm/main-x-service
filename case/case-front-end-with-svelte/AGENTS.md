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
4. **Minimal deps.** No data grid / design system — plain inputs + the
   `app.css` utilities.
5. **No envelope.** The service is loco.rs and returns **raw JSON**;
   `src/lib/api/client.ts` is the lean wrapper (get/post/put/delete).
6. **Auth (BFF + cookie session).** The browser holds **no token** — no
   `localStorage`, no `mxi_access_token`, no URL-fragment handoff. The
   top-bar **Sign in** redirects to the auth front-end; the magic-link
   sets an httpOnly `__Host-mxi_session` cookie. This app's SvelteKit
   server acts as a **Backend-For-Frontend**: it holds the session,
   exchanges it for a short-lived **PASETO v4.public** token, and calls
   the case service server-side; mutating calls carry a CSRF token. See
   [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
   (source of truth; RS256/JWKS decommissioned).

   > Auth pivot in progress: the current `src/lib/auth.svelte.ts` /
   > `ApiClient` runtime still uses the old client-held bearer +
   > fragment-capture flow described in the layout below; the BFF +
   > cookie + PASETO follow-up is tracked in the spec.

## Layout

```
src/
├── lib/
│   ├── config.ts                 PUBLIC_API_BASE_URL (:5150) + AUTH_FRONTEND_URL + signInUrl()
│   ├── auth.svelte.ts            reactive session store (token/setToken/clearToken) + captureTokenFromHash / captureFromLocation
│   ├── api/
│   │   ├── client.ts             lean fetch wrapper (+ ApiError; bearer from auth store)
│   │   ├── types.ts              Case + CaseIdentifier + CaseType + CaseStatus + Priority + IdentifierScheme + CaseRef + ScoredRef
│   │   └── cases.ts              CaseRepository (CRUD + checkDuplicates)
│   └── components/CaseForm.svelte
└── routes/
    ├── +layout.svelte / +layout.ts   nav + SPA toggle + SSO sign-in/out + fragment capture
    ├── +page.svelte              list
    ├── new/+page.svelte          create
    ├── [pid]/+page.svelte        detail + delete + check-duplicates
    └── [pid]/edit/+page.svelte   edit

tests/
├── unit/                         vitest: client / cases / auth / config / case-form
└── e2e/smoke.spec.ts             Playwright: four routes + check-duplicates self-exclusion
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

## Commands

```bash
pnpm install
pnpm dev          # http://localhost:5173
pnpm run check    # svelte-check (strict; 0/0 expected)
pnpm test         # vitest unit tests
pnpm test:e2e     # Playwright smoke tests
pnpm run build
```

Configure with `PUBLIC_API_BASE_URL` (case service) and
`VITE_AUTH_FRONTEND_URL` (SSO sign-in front-end); see `.env.example`.
