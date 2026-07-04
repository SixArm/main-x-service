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
The UI is bilingual (English + Welsh / Cymraeg) via a dependency-free i18n
store (`src/lib/i18n.svelte.ts`); the chosen locale is also sent to the
service so the magic-link email language matches.

> **Session model (canonical):**
> [`AGENTS/share/authentication-sessions.md`](../../AGENTS/share/authentication-sessions.md).
> No token in browser JS. **Supersedes the prior bearer-token /
> `localStorage` SPA model** (and its `#access_token=` handoff).

## Ground rules

1. **Runes only.** `$state` / `$derived` / `$effect` / `$props` /
   `$bindable`. No `export let`, no `$:`, no `createEventDispatcher`
   (events are callback props).
2. **BFF, not pure SPA.** The auth-bearing paths (verify, dashboard load,
   sign-out) run on the SvelteKit **server** (`hooks.server.ts` /
   `+page.server.ts` / `+server.ts`) so the httpOnly session cookie is
   held server-side. The session is a **cookie**, never `localStorage`.
   Add CSRF protection to browser→BFF mutations
   (`AGENTS/share/authentication-sessions.md` §4). Read-only UI may still
   render client-side, but any auth-bearing fetch goes through the BFF.
3. **TypeScript strict** (with `noUncheckedIndexedAccess`).
4. **Minimal deps.** Unlike the data-heavy sibling front-ends, this UI
   has no data grid and no Lily/SVAR dependency — just SvelteKit. Keep it
   that way unless a real need appears (drift is accepted family-wide).
5. **No envelope.** The auth service is loco.rs and returns **raw JSON**
   (no `{success,data,error}` wrapper). `src/lib/api/client.ts` is
   deliberately leaner than the enveloped clients in sibling front-ends.

## Layout

Layout under the BFF model (landed; the prior `session.svelte.ts` token
store + fragment handoff are
removed):

```
src/
├── hooks.server.ts               BFF: read/validate __Host-mxi_session, populate event.locals
├── lib/
│   ├── config.ts                 PUBLIC_API_BASE_URL (:5150) + VITE_RETURN_TO_ALLOWLIST
│   ├── i18n.svelte.ts            bilingual (en/cy) catalog + reactive locale store + t()
│   ├── server/                   server-only: csrf token issue/validate, session-cookie helpers
│   ├── api/
│   │   ├── client.ts             lean fetch wrapper (server-side; attaches cookie/bearer, ApiError)
│   │   ├── types.ts              LoginResponse / CurrentUser (mirror the service views)
│   │   └── auth.ts               AuthRepository (signup/magic-link/verify/me/signout)
│   └── auth/
│       └── return-to.ts          return_to allowlist (open-redirect control) + plain redirect decision
└── routes/
    ├── +layout.svelte            nav + signed-in badge
    ├── +page.svelte              account dashboard (data from +page.server.ts)
    ├── +page.server.ts           dashboard load: read cookie → GET /me ; sign-out action
    ├── signup/+page.svelte       request a magic link (new account)
    ├── signin/+page.svelte       request a magic link (existing account)
    └── verify/
        ├── +page.svelte          status UI
        └── +page.server.ts       consume ?token= server-side -> set session cookie -> redirect
```

## API consumption

All auth-service calls run **server-side (BFF)**; the browser calls only
this app's own server routes (carrying the session cookie + a CSRF token
on mutations).

| UI action | Auth-service call (server-side) |
|---|---|
| Sign up | `POST /api/auth/signup {email, name?, locale?}` |
| Sign in | `POST /api/auth/magic-link {email, locale?}` |
| Verify (`/verify?token=…`) | `GET /api/auth/magic-link/{token}` → relay `Set-Cookie: __Host-mxi_session` |
| Dashboard | `GET /api/auth/me` (session cookie) |
| Sign out | `POST /api/auth/signout` (session cookie) → revoke + clear cookie |

`locale` is the optional current UI locale (`en`/`cy`); it makes the
magic-link email language match the UI and drops out of the body when
unset (the service defaults to English). No token is returned to or held
by the browser — the credential is the httpOnly session cookie
(`AGENTS/share/authentication-sessions.md` §3, §6).

In development the magic link is printed to the **auth service console**
(no SMTP). The link points at `{FRONTEND_URL}/verify?token=…`.

## Commands

```bash
pnpm install
pnpm dev          # http://localhost:5173
pnpm run check    # svelte-check (strict)
pnpm run build
```

Configure the API base URL with `PUBLIC_API_BASE_URL` (see
`.env.example`).
