# Authentication Service

The **central single sign-on provider** for the Main X Index family.
Passwordless **email magic-link** authentication issuing **RS256 JWT**
access tokens that peer services verify offline via JWKS.

Built on **loco.rs** — and the family's reference loco application.

- Spec: [spec/index.md](./spec/index.md)
- Agent guide: [AGENTS.md](./AGENTS.md)
- Sibling UI: [authentication-front-end-with-svelte](../authentication-front-end-with-svelte)

## Endpoints

| Method | Path | Auth | Purpose |
|---|---|---|---|
| POST | `/api/auth/signup` | — | Create account, send magic link |
| POST | `/api/auth/magic-link` | — | Request magic link (sign in) |
| GET | `/api/auth/magic-link/{token}` | — | Consume link → access token |
| GET | `/api/auth/me` | Bearer | Current user |
| POST | `/api/auth/signout` | Bearer | Revoke session |
| GET | `/.well-known/jwks.json` | — | Public keys (offline verification) |

## Quick start

Requires a PostgreSQL instance.

```bash
# 1. Point at a database (or use the loco config default).
export DATABASE_URL=postgres://loco:loco@localhost:5432/authentication-service_development

# 2. Run (auto-migrates in development).
cargo loco start

# 3. Sign up — the magic link is printed to the console in development.
curl -s localhost:5150/api/auth/signup -H 'content-type: application/json' \
  -d '{"email":"you@example.com","name":"You"}'

# 4. Open the logged link (or call verify directly) to get a token:
curl -s localhost:5150/api/auth/magic-link/<TOKEN>
```

## Development notes

- **Magic links** are logged to the tracing console (the SMTP mailer is
  disabled in `config/development.yaml`). Production supplies real SMTP.
- **JWT keys**: a dev RSA keypair is committed under `config/keys/` so
  the app runs out of the box. Production supplies its own via
  `JWT_PRIVATE_KEY_FILE` / `JWT_PUBLIC_KEY_FILE` (or the `*_PEM` inline
  variants). See [AGENTS.md](./AGENTS.md) for all env vars.
- **Queue**: Postgres-backed background jobs (repo convention).

## Testing

```bash
cargo test --lib    # DB-free unit tests for the RS256/JWKS module
cargo clippy --bins
```

Loco's request tests under `tests/` require a Postgres instance.

## How peers verify tokens

Fetch `/.well-known/jwks.json` once, then verify each `Authorization:
Bearer <jwt>` locally with RS256, checking `iss = authentication-service`
and `aud = main-x-service`. No call back to this service is needed on the
hot path.
