# Authentication — email magic link

> Part of the [Case Tracking specification](index.md). Edition detail:
> [loco auth](../case-folder-service-with-rust/spec/auth.md),
> [svelte auth](../case-folder-front-end-with-svelte/spec/auth.md).

Authentication is **passwordless email magic link**. The **sign-in
(magic) token is a short-lived signed JWT**; the **session is an opaque
server-side session id** (not a JWT) held in the HttpOnly `cts_session`
cookie — per [`agents/share/jwt.md`](../agents/share/jwt.md) ("JWTs must
not be used to keep users logged in"). Identity still comes from the
configured allowlist (no *user* table). The session store is **in-process
today** (an upgrade from the previous JWT-in-cookie session); a **durable
Postgres-backed `sessions` table** is the roadmap upgrade (see §Roadmap /
[roadmap.md](roadmap.md)) — it would be this app's first local table, so
it is deferred from the otherwise table-less aggregator design [D-1](design.md).

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
      └─ CREATE an opaque server-side session (random id; in-process store)
      └─ Set-Cookie: cts_session=<opaque-sid>; HttpOnly; SameSite=Lax; Path=/
      └─ 200 { user: { email, name, role } }

4. subsequent requests carry the cookie (or Authorization: Bearer <sid>)
      └─ a guard looks the sid up in the session store and requires a
         live session on /api/* (except /api/auth/*)

5. GET  /api/auth/me      → 200 { user } | 401
   POST /api/auth/logout  → REVOKE the server-side session + clear cookie, 204
```

## Magic-token claims (HS256, shared `secret`)

Only the **magic** (sign-in) token is a JWT. The session is an opaque
server-side id with **no claims** — its identity lives in the session
store, keyed by the random `cts_session` value.

| Claim  | Magic token            |
| ------ | ---------------------- |
| `sub`  | email                  |
| `name` | identity display name  |
| `role` | identity role          |
| `aud`  | `"magic"`              |
| `iat`  | issued-at (unix)       |
| `exp`  | `iat + magic_ttl`      |

The `aud = "magic"` claim means the sign-in token can only be redeemed at
`/api/auth/verify`. The session TTL (`session_ttl`) now governs the
server-side session entry's expiry rather than a token `exp`.

## Roadmap — durable session store

The session store is **in-process** today: opaque sessions are held in a
map on `AuthState`, so they do not survive a restart and are not shared
across replicas (single-instance deployments are unaffected). The upgrade
is a **Postgres-backed `sessions` table** (session id, identity, expiry,
revoked-at) — this would be the app's first local table, so it is tracked
as a deliberate follow-up rather than folded into the table-less design.

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
