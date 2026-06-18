# AGENTS.md — Organization Front-End

Operator UI for the [Organization Service](../organization-service-with-loco):
organization CRUD + matching.

> Read [`spec/index.md`](./spec/index.md) first — the living spec.

## What this is

A SvelteKit 2 / Svelte 5 (runes) **SPA**. It calls the organization
service REST API, whose request/response body is the
`organization_matcher::Organization` shape itself.

## Ground rules

1. **Runes only** (`$state`/`$derived`/`$effect`/`$props`/`$bindable`).
   No `export let`, no `$:`, events are callback props.
2. **SPA.** `+layout.ts` sets `ssr = false` / `prerender = false`.
3. **TypeScript strict** (`noUncheckedIndexedAccess`).
4. **Minimal deps.** No data grid / design system — plain inputs + the
   `app.css` utilities. (Drift from the SVAR/Lily front-ends is accepted
   family-wide.)
5. **No envelope.** The service is loco.rs and returns **raw JSON**;
   `src/lib/api/client.ts` is the lean wrapper (get/post/put/delete).

## Layout

```
src/
├── lib/
│   ├── config.ts                 PUBLIC_API_BASE_URL (:5150) + VITE_AUTH_FRONTEND_URL (:5173) + signInUrl()
│   ├── auth.svelte.ts            bearer-token store (mxi_access_token) + captureTokenFromHash / captureFromLocation (SSO handoff)
│   ├── api/
│   │   ├── client.ts             lean fetch wrapper (+ ApiError, auto-attaches Bearer from auth)
│   │   ├── types.ts              Organization + OrgIdentifier + PostalAddress + OrgRef + ScoredRef; IdentifierScheme + DETERMINISTIC_SCHEMES + ALL_SCHEMES
│   │   ├── build.ts              pure form->payload core: buildOrganization + splitList/blankToUndef + excludeSelf
│   │   └── organizations.ts      OrganizationRepository (CRUD + checkDuplicates)
│   └── components/OrganizationForm.svelte
├── routes/
│   ├── +layout.svelte / +layout.ts   nav + Session panel + SPA toggle
│   ├── +page.svelte              list
│   ├── new/+page.svelte          create
│   ├── [pid]/+page.svelte        detail + delete + check-duplicates
│   └── [pid]/edit/+page.svelte   edit
tests/
├── unit/                         vitest (client, auth, config, organizations, build)
└── e2e/smoke.spec.ts             Playwright (four routes, API stubbed)
```

## Session / SSO

**Target model (BFF).** The browser holds no token: sign-in establishes
a server-side **cookie session** (`__Host-mxi_session`, httpOnly), the
browser talks only to this front-end's own SvelteKit server (BFF), and
the BFF exchanges the session for a short-lived **PASETO v4.public**
token and calls the organization service server-side. Mutating requests
are CSRF-protected; there is no `localStorage` and no `mxi_access_token`.
Service-side enforcement (`ORGANIZATION_REQUIRE_AUTH`) is off by default.

Source of truth:
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
(RS256 JWT + JWKS and the cross-origin `#access_token` fragment handoff
are decommissioned). **Pivot in progress** — the listed `auth.svelte.ts`
client-held-token store / `captureFromLocation` handoff is the current
runtime; the BFF + cookie + CSRF code follow-up is tracked in spec §13.

## API consumption

| UI action | Endpoint |
|---|---|
| List | `GET /api/organizations` |
| Create | `POST /api/organizations` |
| Detail | `GET /api/organizations/{pid}` |
| Edit | `PUT /api/organizations/{pid}` |
| Delete | `DELETE /api/organizations/{pid}` |
| Check duplicates | `POST /api/organizations/check-duplicates` |

## Commands

```bash
pnpm install
pnpm dev          # http://localhost:5173
pnpm run check    # svelte-check (strict; 0/0 expected)
pnpm run build
pnpm test         # vitest unit suite
pnpm test:e2e     # Playwright smoke (production build)
```

Configure the API base URL with `PUBLIC_API_BASE_URL` and the central
auth front-end with `VITE_AUTH_FRONTEND_URL` (see `.env.example`).
