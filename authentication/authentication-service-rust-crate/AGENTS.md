# AGENTS.md — Authentication Service

Entry point for AI coding agents (and humans) working in the
`authentication-service` crate — the **central single sign-on provider**
for the Main X Index family.

> If you read one file, read [`spec/index.md`](./spec/index.md): the
> living specification. This guide tells you **how to work**; the spec
> tells you **what to build**.

---

## What this crate is

A **loco.rs** service that authenticates users via **passwordless email
magic links** and issues **RS256 JWT** access tokens. Every other Main X
service verifies those tokens **offline** against the public keys at
`/.well-known/jwks.json` — no shared secret, no per-request
introspection. Peers do that by embedding the sibling
[authentication-verifier](../authentication-verifier-rust-crate) library;
`tests/sign_verify_contract.rs` pins the shared `Claims` shape and `kid`
derivation across the two crates.

It is also the family's **reference loco.rs application**: the existing
service crates only *declare* `loco-rs` but actually run hand-rolled
Axum. They will be converted to real loco using this crate as the
template (see root `AGENTS.md`).

| Question | Answer |
|---|---|
| Framework | loco.rs 0.16 (real `Hooks`/`AppContext` boot, loco controllers, loco config, `sea-orm-migration`). |
| Auth model | Passwordless magic link. No passwords are ever checked. |
| Tokens | RS256 JWT; public keys published as JWKS for offline verification. |
| Build | `cargo build` |
| Test | `cargo test` (DB-free: unit + contract tests); `cargo test -- --ignored` for the Postgres-backed model/request tests. |
| Lint | `cargo clippy --bins` |
| Run | `cargo loco start` (needs Postgres; see README). |

---

## API surface

| Method | Path | Auth | Purpose |
|---|---|---|---|
| POST | `/api/auth/signup` | — | Create a passwordless account, issue a magic link. |
| POST | `/api/auth/magic-link` | — | Request a magic link for an existing account (sign in). |
| GET | `/api/auth/magic-link/{token}` | — | Consume the link → RS256 access token + session. |
| GET | `/api/auth/me` | Bearer | Current user (rejects revoked + GDPR-erased accounts). |
| POST | `/api/auth/signout` | Bearer | Revoke the current session. |
| GET | `/api/auth/audit/recent` | — | System-wide authentication audit trail (newest 100). |
| GET | `/api/auth/account/export` | Bearer | GDPR right of access: the subject's data (`users` + `sessions` + `auth_events`). |
| GET | `/api/auth/account/audit` | Bearer | GDPR right of access: the subject's own audit trail. |
| DELETE | `/api/auth/account` | Bearer | GDPR right to erasure: soft-delete + anonymise + revoke sessions + audit. |
| GET | `/.well-known/jwks.json` | — | Public keys for offline token verification. |
| GET | `/api-docs/openapi.json` | — | Hand-written OpenAPI 3 document. |
| GET | `/swagger-ui` | — | Swagger UI page (CDN assets) rendering the doc. |

To avoid account enumeration, `signup` and `magic-link` always return
`200` regardless of whether the email exists. They are also
**rate-limited per email** (`src/rate_limit.rs`: `MAX_REQUESTS` = 5 per
`WINDOW` = 5 min, Postgres-backed sliding window via the
`auth_rate_limits` table + per-key advisory lock, shared across
instances); over the cap they
return `429` and issue no token / send no mail, without leaking account
existence.

---

## Golden rules

1. **Loco-idiomatic.** New endpoints are loco controllers registered in
   `app.rs`; new tables are `sea-orm-migration` migrations registered in
   `src/migration/mod.rs` with a matching entity under
   `src/models/_entities/`.
2. **RS256 only.** Token signing/verification lives in `src/auth`. Do
   not reintroduce loco's symmetric HS256 helper for access tokens; peer
   services rely on the JWKS.
3. **No password flow.** This is passwordless. The `users.password`
   column exists only to satisfy `NOT NULL`; it holds an unusable random
   hash (`create_passwordless`).
4. **Don't leak account existence.** Keep the always-`200` responses on
   the unauthenticated endpoints.
5. **Dev has no SMTP.** Magic links are logged to the tracing console in
   development (mailer disabled). Production supplies SMTP via config.
6. **Keys come from the edges.** Private/public PEM load from env in
   production (`JWT_PRIVATE_KEY_FILE` / `JWT_PUBLIC_KEY_FILE` or the
   `*_PEM` inline variants); the committed `config/keys/*_dev.pem` are
   **dev only**.
7. **Key rotation is a key set.** `auth::AuthKeys` holds one *primary*
   signing key plus zero or more *additional* verify-only public keys
   (`JWT_ADDITIONAL_PUBLIC_KEY_FILES` / `_PEMS`). Signing always uses the
   primary; verification selects by the token header `kid`; the JWKS
   publishes all keys. To rotate with zero downtime, follow the runbook
   in `config/keys/README.md` (spec §8.4). Unset additional vars ⇒ the
   single-key behaviour is unchanged.

---

## Layout

```
src/
├── app.rs                 loco Hooks: routes, workers, truncate, seed
├── bin/main.rs            loco CLI entrypoint
├── auth/mod.rs            RS256 signing + verification + JWKS + bearer extractor
├── controllers/
│   ├── auth.rs            signup / magic-link / verify / me / signout / audit + GDPR account export/audit/erasure
│   ├── docs.rs            /api-docs/openapi.json + /swagger-ui
│   └── jwks.rs            /.well-known/jwks.json
├── openapi.rs            hand-written OpenAPI 3 document
├── rate_limit.rs         per-email sliding-window magic-link issuance limiter
├── models/
│   ├── users.rs           magic-link user model (+ create_passwordless, GDPR erase + find_active_by_pid)
│   ├── sessions.rs        session issue/revoke (jid = jwt jti); revoke_all_for_user for erasure
│   └── _entities/         generated SeaORM entities
├── mailers/auth.rs        magic-link mailer (prod)
├── migration/             in-crate migrator: m20220101_000001_users, _000002_sessions, _000003_auth_events, _000004_users_deleted_at, _000005_auth_rate_limits
└── views/auth.rs          LoginResponse / CurrentResponse
config/                    development/production/test yaml + dev RSA keys
```

## Configuration (env)

| Var | Default | Purpose |
|---|---|---|
| `JWT_PRIVATE_KEY_FILE` | `config/keys/jwt_private_dev.pem` | RSA private signing key (PEM). |
| `JWT_PUBLIC_KEY_FILE` | `config/keys/jwt_public_dev.pem` | RSA public verification key (PEM). |
| `JWT_PRIVATE_KEY_PEM` / `JWT_PUBLIC_KEY_PEM` | — | Inline PEM (takes precedence over the file vars). |
| `JWT_ADDITIONAL_PUBLIC_KEY_FILES` | — | Comma-separated paths to extra **verify-only** public keys (rotated-out keys whose tokens are still live). See key rotation below. |
| `JWT_ADDITIONAL_PUBLIC_KEY_PEMS` | — | Inline verify-only public PEMs (comma- or newline-separated). Combined with the files var. |
| `JWT_ISSUER` | `authentication-service` | `iss` claim. |
| `JWT_AUDIENCE` | `main-x-service` | `aud` claim. |
| `JWT_EXPIRATION` | `3600` | Access-token lifetime (seconds). |
| `FRONTEND_URL` | `http://localhost:5173` | Base for the magic link in emails/logs. |
| `DATABASE_URL` | loco config default | Postgres connection. |

## When you are unsure

The spec wins. If the spec is silent, propose an update in
[`spec/index.md`](./spec/index.md) rather than guessing.
