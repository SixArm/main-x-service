# Authentication Service

The **central single sign-on provider** for the Main X Index family.
Passwordless **email magic-link** authentication establishing a
server-side **cookie session**; peer services authenticate offline via
short-lived **PASETO v4.public** tokens minted from the session.

Built on **loco.rs** — and the family's reference loco application.

> **Auth model source of truth:**
> [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md).
> The previous **RS256 JWT + JWKS** access-token model is **decommissioned**
> in favour of httpOnly cookie sessions + PASETO. The pivot has **landed
> in code**: the runtime mints Ed25519 PASETO v4.public tokens and
> publishes its key set at `/.well-known/paseto-keys`; no JWT is issued.

- Spec: [spec/index.md](./spec/index.md)
- Agent guide: [AGENTS.md](./AGENTS.md)
- Sibling UI: [authentication-front-end-with-svelte](../authentication-front-end-with-svelte)

## Endpoints

| Method | Path | Auth | Purpose |
|---|---|---|---|
| POST | `/api/auth/signup` | — | Create account, send magic link |
| POST | `/api/auth/magic-link` | — | Request magic link (sign in) |
| GET | `/api/auth/magic-link/{token}` | — | Consume link → session cookie |
| POST | `/api/auth/token` | Session | Exchange session for a short-lived PASETO v4.public bearer |
| GET | `/api/auth/me` | Session | Current user |
| POST | `/api/auth/signout` | Session | Revoke session |
| GET | `/api/auth/audit/recent` | — | System-wide authentication audit trail |
| GET | `/api/auth/account/export` | Session | GDPR right of access: the subject's data |
| GET | `/api/auth/account/audit` | Session | GDPR right of access: the subject's own audit trail |
| DELETE | `/api/auth/account` | Session | GDPR right to erasure: soft-delete + anonymise |
| GET | `/.well-known/paseto-keys` | — | Published Ed25519 public key(s) (offline verification) |
| GET | `/api-docs/openapi.json` | — | Hand-written OpenAPI 3 document |
| GET | `/swagger-ui` | — | Swagger UI page |
| GET | `/metrics.prom` | — | Prometheus metrics (text exposition; root path) |

`signup` and `magic-link` accept an optional `locale` (`en` / `cy`,
BCP-47; unknown/absent ⇒ English) selecting the **language of the
magic-link email** — the response shape is identical across locales.

## Quick start

Requires a PostgreSQL instance.

```bash
# 1. Point at a database (or use the loco config default).
export DATABASE_URL=postgres://loco:loco@localhost:5432/authentication_service_development

# 2. Run (auto-migrates in development).
cargo loco start

# 3. Sign up — the magic link is printed to the console in development.
curl -s localhost:5150/api/auth/signup -H 'content-type: application/json' \
  -d '{"email":"you@example.com","name":"You"}'

# 4. Open the logged link (or call verify directly) to establish a session:
curl -s localhost:5150/api/auth/magic-link/<TOKEN>
```

## Development notes

- **Magic links** are logged to the tracing console (the SMTP mailer is
  disabled in `config/development.yaml`). Production supplies real SMTP.
- **Signing keys**: the service publishes its **Ed25519** public key(s)
  at `/.well-known/paseto-keys` for PASETO v4.public verification. No
  key files are committed — development uses a built-in dev seed
  (`DEV_SEED`); production supplies the seed via `TOKEN_PRIVATE_KEY_SEED`
  / `TOKEN_PRIVATE_KEY_FILE` (see
  [config/keys/README.md](./config/keys/README.md) and
  [AGENTS.md](./AGENTS.md) for all env vars).
- **Queue**: Postgres-backed background jobs (repo convention).

## Testing

```bash
cargo test               # DB-free tests: crypto unit tests, route table,
                         # and the cross-crate sign→verify contract test
cargo test -- --ignored  # Postgres-backed model + request tests (magic-link surface)
cargo clippy --bins
```

The Postgres-backed tests under `tests/` are `#[ignore]`d so plain
`cargo test` stays green without a database.

## How peers authenticate requests

Peers authenticate **offline**, with no per-request hop back to this
service. A peer fetches the published **Ed25519 public key(s)** from
`/.well-known/paseto-keys` once at boot, then verifies each
`Authorization: Bearer v4.public.…` **PASETO v4.public** token locally,
checking `iss = authentication-service` and `aud = main-x-service`.

Use the sibling
[authentication-verifier](../authentication-verifier-rust-crate) crate
rather than re-implementing this: build a `Verifier` from the published
PASETO keys, then `verify(token)` per request. The cross-crate contract
test (`tests/sign_verify_contract.rs`) keeps this service and the
verifier in lock-step on the `Claims` shape and `kid` derivation.

> RS256 JWT + JWKS are **decommissioned**; the running binary issues
> PASETO v4.public only.
