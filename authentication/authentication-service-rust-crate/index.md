# Authentication Service — documentation index

The central single sign-on provider for the Main X Index family:
passwordless email magic-link auth, RS256 JWT, JWKS for offline
verification. Built on loco.rs.

## Start here

| Doc | Purpose |
|---|---|
| [spec/index.md](./spec/index.md) | **Single source of truth** (§1–§18). |
| [AGENTS.md](./AGENTS.md) | How to work in this crate; API surface; env vars. |
| [README.md](./README.md) | User-facing intro + quick start. |
| [CHANGELOG.md](./CHANGELOG.md) | Release history. |

## Worked flow

```text
signup/signin  ──>  POST /api/auth/signup | /api/auth/magic-link  {email}
                       │  (magic link logged to console in dev)
click link     ──>  GET  /api/auth/magic-link/{token}
                       │  -> { token (RS256 JWT), pid, name, email }
use the token  ──>  GET  /api/auth/me            Authorization: Bearer <jwt>
sign out       ──>  POST /api/auth/signout       Authorization: Bearer <jwt>

peers verify   ──>  GET  /.well-known/jwks.json  (fetch once, verify offline)
```

## Relationship to the family

This is the first **real** loco.rs crate in the repo. The other service
crates declare `loco-rs` but run hand-rolled Axum; they will be converted
to idiomatic loco using this crate as the reference. See the root
[AGENTS.md](../../AGENTS.md).
