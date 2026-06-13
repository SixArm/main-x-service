# authentication-front-end-with-svelte — documentation index

Operator UI for passwordless magic-link sign up / sign in / sign out,
consuming the [Authentication Service](../authentication-service-rust-crate).

## Start here

| Doc | Purpose |
|---|---|
| [spec/index.md](./spec/index.md) | **Single source of truth** (§1–§18). |
| [AGENTS.md](./AGENTS.md) | Conventions, `src/` tree, API consumption map. |
| [README.md](./README.md) | Routes, quick start, configuration. |
| [CHANGELOG.md](./CHANGELOG.md) | Release history. |

## Flow

```text
/signup or /signin  ──>  POST signup | magic-link  {email}
                            │  (link logged to the auth service console in dev)
open the magic link ──>  /verify?token=…  ──>  GET /api/auth/magic-link/{token}
                            │  store { token, pid, name, email } in localStorage
/                   ──>  GET /api/auth/me (bearer)   ·   sign out -> POST /api/auth/signout
```
