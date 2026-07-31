# authentication-front-end-with-svelte

Operator UI for the [Authentication Service](../authentication-service-with-loco):
passwordless email magic-link **sign up / sign in / sign out**.

SvelteKit 2 · Svelte 5 (runes) · TypeScript strict · **BFF** (server-side
session) · bilingual (English + Welsh / Cymraeg).

> **Session model.** Login establishes a **server-side session**; the
> browser holds only the httpOnly `__Host-mxi_session` cookie — no token in
> JS. See the canonical design doc
> [`AGENTS/share/authentication-sessions.md`](../../AGENTS/share/authentication-sessions.md).
> This **supersedes the prior bearer-token / `localStorage` SPA model**
> (and its `#access_token=` cross-origin handoff).

## Routes

| Route | Purpose |
|---|---|
| `/` | Account dashboard (current user, sign out) — server load reads the session cookie |
| `/signup` | Create an account → emailed a magic link |
| `/signin` | Request a magic link for an existing account |
| `/verify?token=…` | Consume the magic link server-side → session cookie set → redirect home |
| `/admin/attributes` | ABAC attribute-assignment admin UI (`?pid=…`) — view / replace a user's attributes; gated on an `access=admin` caller |

## Prerequisites

- Node 20+ and pnpm
- A running [Authentication Service](../authentication-service-with-loco)

## Quick start

```bash
cp .env.example .env     # PUBLIC_API_BASE_URL=http://localhost:5150
pnpm install
pnpm dev                 # http://localhost:5173
```

Sign up, then look at the **auth service console** — in development the
magic link is logged there (no SMTP). Open it to land on `/verify`, which
exchanges the token server-side; the auth service sets the
`__Host-mxi_session` cookie and you are signed in.

## Language (i18n)

The UI is bilingual: **English** (`en`) and **Welsh / Cymraeg** (`cy`) —
the latter a deliberate UK public-sector Welsh-language-duty choice. Pick
a language via the Lily `LocalePicker` in the top-bar layout (a Lily
`ThemePicker` sits beside it for theme choice); the locale persists to
`localStorage["mxi.auth.locale"]` and re-renders every string live. It is
also sent as a `locale` hint on sign-up / sign-in so the **magic-link
email** arrives in the same language. There is no i18n library — just a
small per-locale catalog and a reactive store in
`src/lib/i18n.svelte.ts`.

## Configuration

| Var | Default | Purpose |
|---|---|---|
| `PUBLIC_API_BASE_URL` | `http://localhost:5150` | Auth service REST base URL (no trailing slash). Called **server-side** by the BFF. |
| `VITE_RETURN_TO_ALLOWLIST` | _(empty)_ | Comma-separated operator-app origins (exact `scheme://host[:port]`) the post-verify redirect may target. Unset/empty ⇒ same-origin only. An **open-redirect** control — no credential travels in the redirect. |

## How it works

This front-end is a **Backend-For-Frontend (BFF)**: its SvelteKit
**server** (`hooks.server.ts` / `+page.server.ts` / `+server.ts`) is the
only party that talks to the auth service, and the **session** lives in an
httpOnly cookie the browser cannot read.

- Verifying a magic link is handled server-side; the auth service
  establishes a server-side session and returns
  `Set-Cookie: __Host-mxi_session=…` (HttpOnly · Secure · `SameSite=Lax` ·
  `__Host-` prefix), which the BFF relays to the browser.
- The dashboard load and sign-out run on the server, reading the cookie
  and calling `GET /me` / `POST /signout` (the latter revokes the session
  and clears the cookie).
- **No token in browser JS.** The `localStorage` access token,
  `mxi_access_token` federation key, and `Authorization: Bearer` from the
  browser are all gone.
- **CSRF**: browser→BFF mutating requests carry a per-session CSRF token,
  validated server-side.

Full design (session table, cookie attributes, CSRF, cross-service PASETO,
rollout): [`AGENTS/share/authentication-sessions.md`](../../AGENTS/share/authentication-sessions.md).

### Cross-origin `return_to`

An operator app on a different origin links here to sign in and is sent
back afterwards — but **no credential is handed off** (each origin is
signed in via its own session cookie):

1. The operator app links here as
   `/signin?return_to=<absolute operator-app URL>` (or `/signup?...`).
2. If `origin(return_to)` is allowlisted (`VITE_RETURN_TO_ALLOWLIST`, or
   our own origin), it is preserved across the magic-link email
   round-trip. A non-allowlisted value is ignored.
3. After `/verify` signs the user in, the browser is redirected to
   `return_to` — a plain navigation, **no `#access_token=` fragment**.

The allowlist (`src/lib/auth/return-to.ts`) is the open-redirect control.

## Testing

```bash
pnpm run check     # svelte-check (strict, 0 errors expected)
pnpm run build
pnpm run test      # vitest (unit)
pnpm run test:e2e  # playwright
```

## Project layout

See [AGENTS.md](./AGENTS.md) for the `src/` tree and conventions, and
[spec/index.md](./spec/index.md) for the specification.
