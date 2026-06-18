# authentication-front-end-with-svelte — documentation index

Operator UI for passwordless magic-link sign up / sign in / sign out,
consuming the [Authentication Service](../authentication-service-with-loco).

## Start here

| Doc | Purpose |
|---|---|
| [spec/index.md](./spec/index.md) | **Single source of truth** (§1–§18). |
| [AGENTS.md](./AGENTS.md) | Conventions, `src/` tree, API consumption map. |
| [README.md](./README.md) | Routes, quick start, configuration. |
| [CHANGELOG.md](./CHANGELOG.md) | Release history. |

> **Session model:** httpOnly cookie + BFF — see
> [`AGENTS/share/authentication-sessions.md`](../../AGENTS/share/authentication-sessions.md).
> No token in the browser; this supersedes the prior bearer-token SPA
> model.

## Flow

```text
choose language   ──>  en | cy  (sidebar <select>; persisted to localStorage)
/signup or /signin  ──>  (BFF) POST signup | magic-link  {email, locale}
                            │  (link logged to the auth service console in dev;
                            │   `locale` makes the email language match the UI)
open the magic link ──>  /verify?token=…  ──(server route)──>  GET /api/auth/magic-link/{token}
                            │  auth service sets Set-Cookie: __Host-mxi_session
                            │  (BFF relays it to the browser; no token in JS)
                            ├─ allowlisted return_to? ──> redirect to return_to (no credential)
                            └─ otherwise              ──> /
/                   ──>  (server load) GET /api/auth/me   ·   sign out -> POST /api/auth/signout
```

The whole UI is bilingual (English + Welsh / Cymraeg). The language
`<select>` lives in the sidebar; switching it re-renders every string and
persists the choice, and the choice is sent as the `locale` hint so the
magic-link email arrives in the same language. Welsh support is a
deliberate UK public-sector Welsh-language-duty choice.

### Cross-origin `return_to` (worked example)

An operator app on `https://organization.example.com` (allowlisted via
`VITE_RETURN_TO_ALLOWLIST`) links a user here to sign in, then is sent
back afterwards. **No credential travels in the redirect** — each origin
is signed in via its own httpOnly session cookie:

```text
1. operator app  ──>  https://auth.example.com/signin?return_to=https://organization.example.com/
2. /signin keeps  return_to  (allowlisted) across the email round-trip
3. user opens the emailed magic link ──> /verify?token=…  (server route)
4. auth service sets __Host-mxi_session; /verify redirects the browser to:
        https://organization.example.com/        (plain navigation, no #access_token=)
```

A `return_to` whose origin is **not** allowlisted is ignored (open-redirect
control); `/verify` then just lands on `/`.
