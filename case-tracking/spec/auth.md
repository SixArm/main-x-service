# Authentication — email magic link

> Part of the [Case Tracking specification](index.md). Edition detail:
> [loco auth](../case-tracker-service-with-rust/spec/auth.md),
> [svelte auth](../case-tracker-front-end-with-svelte/spec/auth.md).

Authentication is **passwordless email magic link**, backed by
**stateless signed tokens** (no auth tables — consistent with the
aggregator decision [D-1](design.md)).

## Flow

```
1. POST /api/auth/request { email }
      └─ resolve identity by email (configured allowlist)
      └─ mint a short-lived MAGIC token (signed JWT, ~10 min, aud=magic)
      └─ email a link:  {frontend_url}/auth/callback?token=<MAGIC>
      └─ 200 { sent: true }            (+ magic_link in dev/test only)

2. user clicks the link → /auth/callback?token=…

3. POST /api/auth/verify { token }
      └─ validate MAGIC token (sig, exp, aud)
      └─ mint a SESSION token (signed JWT, ~24 h, aud=session)
      └─ Set-Cookie: cts_session=<SESSION>; HttpOnly; SameSite=Lax; Path=/
      └─ 200 { user: { email, name, role } }

4. subsequent requests carry the cookie (or Authorization: Bearer <SESSION>)
      └─ a guard requires a valid SESSION on /api/* (except /api/auth/*)

5. GET  /api/auth/me      → 200 { user } | 401
   POST /api/auth/logout  → clears the cookie, 204
```

## Token claims (HS256, shared `secret`)

| Claim  | Magic token            | Session token          |
| ------ | ---------------------- | ---------------------- |
| `sub`  | email                  | email                  |
| `name` | identity display name  | identity display name  |
| `role` | identity role          | identity role          |
| `aud`  | `"magic"`              | `"session"`            |
| `iat`  | issued-at (unix)       | issued-at (unix)       |
| `exp`  | `iat + magic_ttl`      | `iat + session_ttl`    |

`aud` separation means a magic token can never be replayed as a session
token and vice-versa.

## Identity

Identity is resolved from a **configured allowlist** mapping email →
`{ name, role }`. This avoids a user table and any upstream change. The
allowlist is intended to be replaced by NHS **CIS2 / OIDC** in
production (see [regulatory.md](regulatory.md)); the magic-link flow is
the demo-grade stand-in.

To avoid email enumeration, `POST /api/auth/request` always returns
`200 { sent: true }` whether or not the email is on the allowlist — a
token is only minted + emailed when it matches.

## Email delivery

A `Mailer` abstraction sends the link. The repo ships a **log mailer**
(writes the link to the server log) so the flow works with no SMTP
server — mirroring the in-process upstream stubs ([D-5](design.md)). A
real SMTP mailer is a production task. In dev/test, `expose_magic_link`
also returns the link in the `request` response so automated flows and
local clicking work without reading logs.

## Sessions & transport

- Session is an **HttpOnly cookie** (`cts_session`) so the token is
  never readable by JavaScript — satisfies the "no PII in JS storage"
  privacy posture ([regulatory.md](regulatory.md)).
- `Authorization: Bearer <session>` is also accepted (for API clients
  and server-to-server tests).
- Because the cookie is HttpOnly + `SameSite=Lax`, the browser and API
  must be **same-origin**. In dev the SvelteKit server proxies `/api` to
  the Loco API so the cookie is first-party; in production they sit
  behind one ingress. This nudges the project toward the same-origin
  production gate it already wanted.

## Configuration (per environment)

| Key                      | dev / stub                | test            | production           |
| ------------------------ | ------------------------- | --------------- | -------------------- |
| `secret`                 | dev default               | fixed test key  | **required env var** |
| `magic_link_ttl_seconds` | 600                       | 600             | 600                  |
| `session_ttl_seconds`    | 86400                     | 86400           | 86400                |
| `require_session`        | `true`                    | `false`         | `true`               |
| `expose_magic_link`      | `true`                    | `true`          | `false`              |
| `cookie_secure`          | `false` (http)            | `false`         | `true`               |
| `allowlist`              | demo emails               | test email      | from secrets / OIDC  |

`require_session: false` in `test` keeps the existing request-test suite
asserting domain behaviour without logging in; auth itself is covered by
dedicated tests and by `/api/auth/me` (which always requires a session).

## Security notes (demo vs production)

- The magic link is a bearer credential in email — short TTL mitigates,
  but production should add single-use + a denylist, which a stateless
  scheme can't do alone (acknowledged trade-off of [D-9](design.md)).
- Production must set a strong `secret` from a secrets manager, enable
  `cookie_secure`, disable `expose_magic_link`, and replace the
  allowlist with CIS2/OIDC. See [regulatory.md](regulatory.md).
