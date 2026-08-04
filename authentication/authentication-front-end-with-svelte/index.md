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
> [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md).
> No token in the browser; this supersedes the prior bearer-token SPA
> model.

## Flow

```text
choose language   ──>  one of 13 (top-bar Lily LocalePicker; persisted to localStorage)
/signup or /signin  ──>  (BFF) POST signup | magic-link  {email, locale}
                            │  (link logged to the auth service console in dev;
                            │   `locale` makes the email language match the UI)
open the magic link ──>  /verify?token=…  ──(server route)──>  GET /api/auth/magic-link/{token}
                            │  auth service sets Set-Cookie: __Host-mxi_session (+ __Host-mxi_csrf)
                            │  (BFF relays it to the browser; no token in JS)
                            └─ redirect  ──> /   (always — no return_to)
/  (any page, via +layout.server.ts) ──> POST /api/auth/token (cookie+CSRF → bearer) → GET /api/auth/me
   sign out -> POST /api/auth/token (as above) → POST /api/auth/signout (bearer)
```

The whole UI ships the family's standard 13-locale catalog. The Lily
`LocalePicker` lives in the **top navigation bar** (no sidebar — see the
Layout shell rule in `spec/index.md` §5); switching it re-renders every
string and persists the choice, and the choice is sent as the `locale`
hint so the magic-link email arrives in the same language. Welsh support
is a deliberate UK public-sector Welsh-language-duty choice.

### No cross-origin handoff (historical)

An earlier revision let an operator app on a different origin link here
(`/signin?return_to=<url>`), sign a user in, and be redirected back with
a bearer token in a URL fragment, gated by an allowlist. **That entire
mechanism is removed** — not restated as credential-free, gone — because
every sibling front-end is now its own independent BFF (its own
`/signin`, its own session cookie against the auth service directly), so
there is nothing to hand back to. `/verify` unconditionally redirects to
`/`. See `spec/index.md` §13 for when/why.
