# AGENTS.md — Authentication Front-End

Entry point for AI coding agents (and humans) working in
`authentication-front-end-with-svelte`: the operator UI for the
[Authentication Service](../authentication-service-rust-crate).

> Read [`spec/index.md`](./spec/index.md) — the living spec — first.

## What this is

A SvelteKit 2 / Svelte 5 (runes) **SPA** for passwordless magic-link
**sign up / sign in / sign out**. It calls the auth service REST API and
stores the resulting RS256 JWT client-side as the federation's bearer
credential.

## Ground rules

1. **Runes only.** `$state` / `$derived` / `$effect` / `$props` /
   `$bindable`. No `export let`, no `$:`, no `createEventDispatcher`
   (events are callback props).
2. **SPA.** `src/routes/+layout.ts` sets `ssr = false` and
   `prerender = false`; the session lives in `localStorage`. Do not add
   server load functions that assume a session.
3. **TypeScript strict** (with `noUncheckedIndexedAccess`).
4. **Minimal deps.** Unlike the data-heavy sibling front-ends, this UI
   has no data grid and no Lily/SVAR dependency — just SvelteKit. Keep it
   that way unless a real need appears (drift is accepted family-wide).
5. **No envelope.** The auth service is loco.rs and returns **raw JSON**
   (no `{success,data,error}` wrapper). `src/lib/api/client.ts` is
   deliberately leaner than the enveloped clients in sibling front-ends.

## Layout

```
src/
├── lib/
│   ├── config.ts                 PUBLIC_API_BASE_URL (:5150) + VITE_RETURN_TO_ALLOWLIST
│   ├── api/
│   │   ├── client.ts             lean fetch wrapper (+ bearer, ApiError)
│   │   ├── types.ts              LoginResponse / CurrentUser (mirror the service views)
│   │   └── auth.ts               AuthRepository (signup/magic-link/verify/me/signout)
│   └── auth/
│       ├── session.svelte.ts     token + profile, persisted to localStorage (runes); mirrors token to mxi_access_token
│       └── return-to.ts          cross-origin SSO handoff: return_to allowlist + pure redirect decision
└── routes/
    ├── +layout.svelte            nav + signed-in badge
    ├── +layout.ts                SPA toggle
    ├── +page.svelte              account dashboard + sign out
    ├── signup/+page.svelte       request a magic link (new account)
    ├── signin/+page.svelte       request a magic link (existing account)
    └── verify/+page.svelte       consume ?token= -> store session -> /
```

## API consumption

| UI action | Service call |
|---|---|
| Sign up | `POST /api/auth/signup {email, name?}` |
| Sign in | `POST /api/auth/magic-link {email}` |
| Verify (`/verify?token=…`) | `GET /api/auth/magic-link/{token}` → store `{token, …}` |
| Dashboard | `GET /api/auth/me` (bearer) |
| Sign out | `POST /api/auth/signout` (bearer) → clear session |

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
