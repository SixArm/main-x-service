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
4. **Dependency-light where possible.** `@svar-ui/svelte-grid` (the
   `/organizations` index grid), `@svar-ui/svelte-kanban` (the `/review`
   board), and Lily Design System (`ThemePicker`/`LocalePicker` in the
   layout chrome) are real, used dependencies; forms stay plain inputs +
   the `app.css` utilities. (Drift between front-ends is accepted
   family-wide.)
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
│   │   ├── types.ts              Organization + OrgIdentifier + PostalAddress + OrgRef + ScoredRef; IdentifierScheme + DETERMINISTIC_SCHEMES + ALL_SCHEMES
│   │   ├── build.ts              pure form->payload core: buildOrganization + splitList/blankToUndef + excludeSelf
│   │   └── organizations.ts      OrganizationRepository (CRUD + checkDuplicates)
│   ├── server/                   BFF-only (never bundled to the browser): auth.ts (magic-link + session→PASETO exchange), session.ts (cookie), config.ts (ORGANIZATION_API_URL / AUTH_API_URL)
│   └── components/OrganizationForm.svelte
├── routes/
│   ├── +layout.svelte / +layout.ts / +layout.server.ts   nav + session panel
│   ├── +page.svelte              list
│   ├── signin/ · verify/         per-app magic-link sign-in (BFF server routes)
│   ├── api/proxy/[...path]/+server.ts   BFF proxy → organization service (injects the PASETO bearer)
│   ├── new/+page.svelte          create
│   ├── [pid]/+page.svelte        detail + delete + check-duplicates
│   ├── [pid]/edit/+page.svelte   edit
│   └── merge/+page.svelte        merge a duplicate into a survivor + recent merge history
tests/
├── unit/                         vitest (client, build, organizations, i18n, layout)
└── e2e/smoke.spec.ts             Playwright (four routes, API stubbed)
```

## Session / SSO

**BFF model (current).** The browser holds no token: sign-in establishes
a server-side **cookie session** (`__Host-mxi_session`, httpOnly), the
browser talks only to this front-end's own SvelteKit server (BFF), and
the BFF exchanges the session for a short-lived **PASETO v4.public**
token and calls the organization service server-side. Mutating requests
are CSRF-protected; there is no `localStorage` and no `mxi_access_token`.
Service-side enforcement (`ORGANIZATION_REQUIRE_AUTH`) is off by default.

Source of truth:
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
(RS256 JWT + JWKS and the cross-origin `#access_token` fragment handoff
are decommissioned). The former `auth.svelte.ts` client-held-token store
/ `captureFromLocation` handoff has been removed — the runtime is the
BFF: `src/lib/server/` exchanges the session for the PASETO and the
`/api/proxy` route calls the service server-side.

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

Configure the BFF's upstream URLs with the server-side env vars
`ORGANIZATION_API_URL` (organization service) and `AUTH_API_URL`
(authentication service) — see `src/lib/server/config.ts`; both default
to `http://localhost:5150`.
