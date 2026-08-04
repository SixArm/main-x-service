# AGENTS.md — Authentication Front-End

Entry point for AI coding agents (and humans) working in
`authentication-front-end-with-svelte`: the operator UI for the
[Authentication Service](../authentication-service-with-loco).

> Read [`spec/index.md`](./spec/index.md) — the living spec — first.

## What this is

A SvelteKit 2 / Svelte 5 (runes) app for passwordless magic-link
**sign up / sign in / sign out**, structured as a **Backend-For-Frontend
(BFF)**: the SvelteKit **server** holds the session and calls the auth
service; the browser holds only the httpOnly `__Host-mxi_session` cookie.
The UI ships the family's standard **13-locale** catalog (not just
English + Welsh — see the Glossary note in `spec/index.md` §4) via a
dependency-free i18n store (`src/lib/i18n.svelte.ts`); the chosen locale
is also sent to the service so the magic-link email language matches.

> **Session model (canonical):**
> [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md).
> No token in browser JS. **Supersedes the prior bearer-token /
> `localStorage` SPA model** (and its `#access_token=`/`return_to` cross-
> origin handoff, removed entirely — not just the credential part; see
> spec §13).

## Ground rules

1. **Runes only.** `$state` / `$derived` / `$effect` / `$props` /
   `$bindable`. No `export let`, no `$:`, no `createEventDispatcher`
   (events are callback props).
2. **BFF, not pure SPA.** The auth-bearing paths (verify, dashboard load,
   sign-out) run on the SvelteKit **server** (`hooks.server.ts` /
   `+page.server.ts` / `+layout.server.ts`) so the httpOnly session cookie
   is held server-side. The session is a **cookie**, never `localStorage`.
   CSRF protection on browser→BFF mutations is implemented
   (`agents/share/authentication-sessions.md` §4; `src/lib/server/session.ts`
   holds the `__Host-mxi_csrf` cookie helpers). Read-only UI may still
   render client-side, but any auth-bearing fetch goes through the BFF.
3. **TypeScript strict** (with `noUncheckedIndexedAccess`).
4. **Minimal deps.** Unlike the data-heavy sibling front-ends, this UI
   uses no data grid. Lily `ThemePicker`/`LocalePicker` ARE used in the
   layout (theme + locale chrome); the SVAR packages are declared in
   `package.json` but currently **unused in `src/`**. Add nothing further
   unless a real need appears (drift is accepted family-wide).
5. **No envelope.** The auth service is loco.rs and returns **raw JSON**
   (no `{success,data,error}` wrapper).
6. **`src/lib/api/` is dead code, not a client library to extend.**
   `client.ts` (`ApiClient`) and `auth.ts` (`AuthRepository`) are the
   pre-BFF, browser-held-token model's HTTP layer. No route imports
   either one today — the live BFF calls the auth service via
   `src/lib/server/auth.ts` / `src/lib/server/admin.ts` instead (plain
   `fetch` against `AUTH_API_URL`, never a shared client class). `api/`'s
   only remaining callers are its own unit tests
   (`tests/unit/client.test.ts`, `tests/unit/auth.test.ts`). Don't wire a
   new route to it without first checking whether it should be deleted
   instead — see spec §13.

## Layout

Layout under the BFF model (landed; the prior `session.svelte.ts` token
store, `src/lib/auth/return-to.ts`, and the fragment handoff are removed
— not just superseded, deleted, in `f66ff50f`):

```
src/
├── hooks.server.ts               BFF: read __Host-mxi_session + __Host-mxi_csrf, populate event.locals
├── app.d.ts                      App.Locals: sessionId, csrfToken
├── lib/
│   ├── config.ts                 dead: PUBLIC_API_BASE_URL + VITE_RETURN_TO_ALLOWLIST, read only by lib/api/ below
│   ├── i18n.svelte.ts            13-locale catalog + reactive locale store + t() (en source of truth; see spec §4)
│   ├── server/                   the REAL BFF, reads AUTH_API_URL (private, server-only):
│   │   ├── session.ts              SESSION_COOKIE/CSRF_COOKIE names + options, Set-Cookie parsing
│   │   ├── auth.ts                 verifyMagicLink/requestMagicLink/signup/exchangeToken/currentUser/signout
│   │   └── admin.ts                ABAC attribute GET/PUT (exchanges session for a bearer first)
│   └── api/                      DEAD — no route imports this; kept alive only by its own unit tests (see Ground rule 6)
│       ├── client.ts               ApiClient (reads config.ts's PUBLIC_API_BASE_URL)
│       ├── types.ts                LoginResponse / CurrentUser (mirror the service views; still used by server/ too)
│       └── auth.ts                 AuthRepository (signup/magic-link/verify/me/signout)
└── routes/
    ├── +layout.server.ts         resolves the signed-in user: cookie → /token exchange → GET /me (drives every page)
    ├── +layout.svelte            top nav + locale/theme pickers + signed-in badge
    ├── +page.svelte              account dashboard (data from +layout.server.ts, NOT its own load)
    ├── +page.server.ts           sign-out action ONLY (no load of its own)
    ├── signup/+page.svelte + +page.server.ts   request a magic link (new account)
    ├── signin/+page.svelte + +page.server.ts   request a magic link (existing account)
    ├── admin/attributes/         operator UI: view/replace a user's ABAC attributes (?pid=…; admin-gated; save action)
    └── verify/
        ├── +page.svelte          status UI
        └── +page.server.ts       consume ?token= server-side -> set session + CSRF cookies -> redirect to "/" (always; no return_to)
```

## API consumption

All auth-service calls run **server-side (BFF)**; the browser calls only
this app's own server routes (carrying the session cookie + a CSRF token
on mutations).

| UI action | Auth-service call (server-side) |
|---|---|
| Sign up | `POST /api/auth/signup {email, name?, locale?}` |
| Sign in | `POST /api/auth/magic-link {email, locale?}` |
| Verify (`/verify?token=…`) | `GET /api/auth/magic-link/{token}` → relay `Set-Cookie: __Host-mxi_session` (and `__Host-mxi_csrf`) |
| Dashboard load, every page | `POST /api/auth/token` (session cookie + CSRF header → bearer) then `GET /api/auth/me` (bearer) |
| Sign out | `POST /api/auth/token` (as above) then `POST /api/auth/signout` (bearer) → revoke + clear cookies |
| Manage attributes (admin) | `POST /api/auth/token` then `GET`/`PUT /api/auth/admin/users/{pid}/attributes` (bearer; requires `access=admin`) |

`GET /me` and `POST /signout` are **not** called with the session cookie
directly — every one of them first spends the session on a
`POST /api/auth/token` exchange (cookie + CSRF header → short-lived
PASETO), then calls with `Authorization: Bearer <token>`. This exchange
is cookie-authed and mutating, so the BFF echoes the session's CSRF token
(captured from `__Host-mxi_csrf` at verify, re-hosted httpOnly on this
origin) in the `X-CSRF-Token` header.

`locale` is the optional current UI locale (any of the 13 supported
codes); it makes the magic-link email language match the UI and drops
out of the body when unset (the service defaults to English). No token
is returned to or held by the browser — the credential is the httpOnly
session cookie (`agents/share/authentication-sessions.md` §3, §6).

In development the magic link is printed to the **auth service console**
(no SMTP) — confirmed live in `tutorials/03-authentication-abac.md`
(TUT-3). The link points at `{FRONTEND_URL}/verify?token=…`, where
`FRONTEND_URL` is the **auth service's own** env var (not read by this
front-end).

## Commands

```bash
pnpm install
pnpm dev          # http://localhost:5173
pnpm run check    # svelte-check (strict)
pnpm test         # vitest (unit) — passing
pnpm run build
```

Configure the auth-service base URL with `AUTH_API_URL` (see
`.env.example`; NOT `PUBLIC_API_BASE_URL` — see Ground rule 6).

`pnpm run test:e2e` (playwright) currently **fails 5 of 9 cases** — its
`page.route()` stubs intercept only browser-issued requests, but every
auth-service call moved server-side under the BFF migration, so
`AUTH_API_URL` (unset in CI/dev ⇒ `http://localhost:5150`) is hit for
real and the stub never engages. This is a pre-existing, unfixed gap
from the BFF migration (`f66ff50f`), not something introduced by this
audit; see spec §11/§13.
