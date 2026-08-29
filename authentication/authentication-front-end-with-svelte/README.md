# authentication-front-end-with-svelte

Operator UI for the [Authentication Service](../authentication-service-with-loco):
passwordless email magic-link **sign up / sign in / sign out**.

SvelteKit 2 · Svelte 5 (runes) · TypeScript strict · **BFF** (server-side
session) · 13-locale UI (English, Welsh / Cymraeg, + 11 more).

> **Session model.** Login establishes a **server-side session**; the
> browser holds only the httpOnly `__Host-mxi_session` cookie — no token in
> JS. See the canonical design doc
> [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md).
> This **supersedes the prior bearer-token / `localStorage` SPA model**
> (and its `#access_token=`/`return_to` cross-origin handoff — removed
> entirely, not just the credential part).

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
cp .env.example .env     # AUTH_API_URL=http://localhost:5150
pnpm install
pnpm dev                 # http://localhost:5173
```

Sign up, then look at the **auth service console** — in development the
magic link is logged there (no SMTP). Open it to land on `/verify`, which
exchanges the token server-side; the auth service sets the
`__Host-mxi_session` cookie and you are signed in.

## Language (i18n)

The UI ships the family's standard **13-locale** catalog (English, Welsh
`cy`, Spanish, French, German, Arabic, Russian, Hindi, Mandarin, Bengali,
Portuguese, Indonesian, Urdu) — Welsh a deliberate UK public-sector
Welsh-language-duty choice, the other eleven matching the sibling
front-ends' coverage. Pick a language via the Lily `LocalePicker` in the
top-bar layout (a Lily `ThemePicker` sits beside it for theme choice);
the locale persists to `localStorage["mxi.auth.locale"]` and re-renders
every string live, including right-to-left layout for Arabic/Urdu. It is
also sent as a `locale` hint on sign-up / sign-in so the **magic-link
email** arrives in the same language. There is no i18n library — just a
per-locale catalog and a reactive store in `src/lib/i18n.svelte.ts`
(`pnpm test` pins full 13-locale key coverage).

## Configuration

| Var | Default | Purpose |
|---|---|---|
| `AUTH_API_URL` | `http://localhost:5150` | Auth service REST base URL (no trailing slash). Read **server-side only**, by the BFF (`src/lib/server/auth.ts`, `src/lib/server/admin.ts`). This is the var that actually configures the running app. |

`PUBLIC_API_BASE_URL` and `VITE_RETURN_TO_ALLOWLIST` used to appear here
too, but fed the pre-BFF `src/lib/config.ts` → `src/lib/api/{client,auth}.ts`
layer, which no route ever imported. That code (and the two vars) was
deleted 2026-08-29 (PRO-P24) — see `AGENTS.md` Ground rule 6.

## How it works

This front-end is a **Backend-For-Frontend (BFF)**: its SvelteKit
**server** (`hooks.server.ts` / `+page.server.ts` / `+server.ts`) is the
only party that talks to the auth service, and the **session** lives in an
httpOnly cookie the browser cannot read.

- Verifying a magic link is handled server-side; the auth service
  establishes a server-side session and returns
  `Set-Cookie: __Host-mxi_session=…` (HttpOnly · Secure · `SameSite=Lax` ·
  `__Host-` prefix), which the BFF relays to the browser.
- The dashboard load and sign-out run on the server: reading the cookie,
  exchanging it for a short-lived bearer (`POST /token`), and calling
  `GET /me` / `POST /signout` with that bearer (the latter revokes the
  session and clears the cookies).
- **No token in browser JS.** The `localStorage` access token,
  `mxi_access_token` federation key, and `Authorization: Bearer` from the
  browser are all gone.
- **CSRF**: browser→BFF mutating requests carry a per-session CSRF token,
  validated server-side.

Full design (session table, cookie attributes, CSRF, cross-service PASETO,
rollout): [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md).

### No cross-origin handoff

Earlier revisions of this app issued a shared bearer token and handed it
to another operator-app origin via an allowlisted `?return_to=` +
`#access_token=` URL fragment. **That handoff is removed, not just its
credential-carrying part** — `/verify` always redirects to `/` on this
app's own origin; there is no `return_to` parameter, no allowlist
consulted, and `src/lib/auth/return-to.ts` no longer exists (deleted in
`f66ff50f`). Under the family-wide BFF pattern
(`agents/share/authentication-sessions.md` §6) every sibling front-end is
its **own** independent BFF with its own `/signin` route and its own
session cookie against the auth service directly, so there is no longer
a "come here to sign in, then bounce back" flow to support. See
`spec/index.md` §13 for the history.

## Testing

```bash
pnpm run check     # svelte-check (strict, 0 errors expected) — passing
pnpm run build     # passing
pnpm run test      # vitest (unit) — passing, 17 tests / 3 files
pnpm run test:e2e  # playwright — 6/6 PASSING
```

`tests/e2e/smoke.spec.ts` no longer stubs the auth API via `page.route()`
— that only intercepts requests the **browser** makes, but every
auth-service call happens server-side (the BFF's own `fetch`), so the
stub could never see any of them (a silent gap since the BFF migration).
`playwright.config.ts` instead starts a second `webServer`,
`tests/e2e/mock-auth-server.mjs` — a small Node HTTP stub — and points
`AUTH_API_URL` at it, so the SvelteKit server's real outbound calls hit
something that actually answers them. See `spec/index.md` §11/§13.

## Project layout

See [AGENTS.md](./AGENTS.md) for the `src/` tree and conventions, and
[spec/index.md](./spec/index.md) for the specification.
