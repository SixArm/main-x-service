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
magic links**. Verifying a magic link establishes a server-side
**cookie session** (httpOnly `__Host-mxi_session`); cross-service auth is
a short-lived **PASETO v4.public** token minted from that session. Every
other Main X service verifies those tokens **offline** against the
published Ed25519 public key(s) at `/.well-known/paseto-keys` — no shared
secret, no per-request introspection. Peers do that by embedding the
sibling [authentication-verifier](../authentication-verifier-rust-crate)
library; `tests/sign_verify_contract.rs` pins the shared `Claims` shape
and `kid` derivation across the two crates.

It is also the **sourcing side of the family's ABAC authorization**
(shared
[`agents/share/authorization-attributes.md`](../../agents/share/authorization-attributes.md)):
`users.attributes` holds a string→strings subject-attribute map (e.g.
`{"access": ["write"]}`), session establishment copies it into
`sessions.data.attrs`, and token minting stamps it into the PASETO
**`attrs`** claim, which peers evaluate with the verifier crate's shared
`abac` policy engine. Attribute *assignment* is an operator action with
two surfaces: the `user_attributes` loco CLI task
(`src/tasks/attributes.rs`) and the admin HTTP API
(`src/controllers/admin.rs`, gated by `access=admin`); both write an
`attributes_assigned` `auth_events` audit row. `scope`/`roles` are
deprecated for authorization.

> **Auth model source of truth:**
> [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md).
> The old **RS256 JWT + JWKS** model is **decommissioned** in favour of
> cookie sessions + PASETO. The pivot has **landed in code**: the runtime
> mints Ed25519 PASETO v4.public tokens and publishes its key set at
> `/.well-known/paseto-keys`; no JWT is issued anywhere.

It is also the family's **reference loco.rs application**: the existing
service crates only *declare* `loco-rs` but actually run hand-rolled
Axum. They will be converted to real loco using this crate as the
template (see root `AGENTS.md`).

| Question | Answer |
|---|---|
| Framework | loco.rs 1.0.1 (real `Hooks`/`AppContext` boot, loco controllers, loco config, `sea-orm-migration` 2.0). |
| Auth model | Passwordless magic link → server-side cookie session. No passwords are ever checked. |
| Tokens | Cross-service: short-lived PASETO v4.public (Ed25519); public key(s) published at `/.well-known/paseto-keys` for offline verification. |
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
| GET | `/api/auth/magic-link/{token}` | — | Consume the link → server-side session + `__Host-mxi_session` cookie. |
| POST | `/api/auth/token` | Session + CSRF | Exchange a valid session for a short-lived PASETO v4.public bearer (~5 min), carrying the session's ABAC `attrs` claim. Requires the `X-CSRF-Token` header to match the session's synchroniser token (`403` on mismatch). |
| GET | `/api/auth/me` | Session | Current user (rejects revoked + GDPR-erased accounts). |
| POST | `/api/auth/signout` | Session | Revoke the current session. |
| GET | `/api/auth/audit/recent` | — | System-wide authentication audit trail (newest 100). |
| GET | `/api/auth/account/export` | Session | GDPR right of access: the subject's data (`users` + `sessions` + `auth_events`). |
| GET | `/api/auth/account/audit` | Session | GDPR right of access: the subject's own audit trail. |
| DELETE | `/api/auth/account` | Session | GDPR right to erasure: soft-delete + anonymise + revoke sessions + audit. |
| GET | `/api/auth/admin/users/{pid}/attributes` | Admin | Show a user's ABAC subject attributes. `403` unless the caller carries `access=admin`. |
| PUT | `/api/auth/admin/users/{pid}/attributes` | Admin | Replace a user's ABAC attribute map (body `{ "attributes": { … } }`); validates keys/values, writes an `attributes_assigned` audit row. |
| GET | `/.well-known/paseto-keys` | — | Published Ed25519 public key(s) for offline PASETO verification. |
| GET | `/api-docs/openapi.json` | — | Hand-written OpenAPI 3 document. |
| GET | `/swagger-ui` | — | Swagger UI page (CDN assets) rendering the doc. |
| GET | `/metrics.prom` | — | Prometheus metrics (text exposition; root path, no `/api` prefix). |

To avoid account enumeration, `signup` and `magic-link` always return
`200` regardless of whether the email exists. They are also
**rate-limited per email** (`src/rate_limit.rs`: `MAX_REQUESTS` = 5 per
`WINDOW` = 5 min, Postgres-backed sliding window via the
`auth_rate_limits` table + per-key advisory lock, shared across
instances); over the cap they
return `429` and issue no token / send no mail, without leaking account
existence.

`signup` and `magic-link` also accept an optional `locale` (`en` / `cy`)
that selects only the **language** of the magic-link email (English +
Welsh, per the Welsh Language (Wales) Measure 2011); the response shape
is unchanged across locales. Copy lives in the dependency-light
`src/i18n.rs` catalog — extend `SUPPORTED_LOCALES` + `magic_link_email`
to add a locale. See spec §6.11 / §12.

---

## Golden rules

1. **Loco-idiomatic.** New endpoints are loco controllers registered in
   `app.rs`; new tables are `sea-orm-migration` migrations registered in
   `src/migration/mod.rs` with a matching entity under
   `src/models/_entities/`.
2. **Asymmetric public-key tokens only.** Cross-service token
   signing/verification lives in `src/auth`: PASETO v4.public
   (Ed25519). Do not reintroduce loco's symmetric HS256 helper
   for cross-service tokens — peers rely on the published public key(s)
   at `/.well-known/paseto-keys`. (RS256 JWT + JWKS are decommissioned.)
3. **No password flow.** This is passwordless. The `users.password`
   column exists only to satisfy `NOT NULL`; it holds an unusable random
   hash (`create_passwordless`).
4. **Don't leak account existence.** Keep the always-`200` responses on
   the unauthenticated endpoints.
5. **Dev has no SMTP.** Magic links are logged to the tracing console in
   development (mailer disabled). Production supplies SMTP via config.
6. **Keys come from the edges.** The Ed25519 signing seed loads from
   env in production (`TOKEN_PRIVATE_KEY_SEED` inline base64url, or
   `TOKEN_PRIVATE_KEY_FILE`); with neither set, a built-in dev seed
   (`DEV_SEED` in `src/auth/mod.rs`) keeps local runs and tests working
   offline — **dev only**. No key files are committed.
7. **Key rotation is a key set.** `auth::AuthKeys` holds one *primary*
   signing key plus zero or more *additional* verify-only public keys
   (`TOKEN_ADDITIONAL_PUBLIC_KEYS`). Signing always uses the
   primary; verification selects by the token footer `kid`; the
   published key set at `/.well-known/paseto-keys` advertises all keys.
   To rotate with zero downtime, follow the runbook in
   `config/keys/README.md` (spec §8.4). Unset
   additional vars ⇒ the single-key behaviour is unchanged.

---

## Layout

```
src/
├── app.rs                 loco Hooks: routes, workers, truncate, seed
├── bin/main.rs            loco CLI entrypoint
├── auth/mod.rs            PASETO v4.public signing + verification + key-set publication + bearer extractor (Ed25519; built-in DEV_SEED for dev)
├── cookie.rs              __Host-mxi_session cookie helpers (set / clear / parse)
├── csrf.rs                CSRF synchroniser token (generate / __Host-mxi_csrf cookie / constant-time compare) for POST /token
├── controllers/
│   ├── auth.rs            signup / magic-link / verify / me / signout / audit + GDPR account export/audit/erasure
│   ├── admin.rs           ABAC attribute assignment over HTTP (GET/PUT /api/auth/admin/users/{pid}/attributes; access=admin gated)
│   ├── docs.rs            /api-docs/openapi.json + /swagger-ui
│   ├── paseto_keys.rs     published key endpoint (/.well-known/paseto-keys — Ed25519 public key set)
│   └── metrics.rs         /metrics.prom (Prometheus text exposition)
├── metrics.rs            Prometheus registry + auth-specific counters
├── i18n.rs               dependency-light email copy catalog (en / cy)
├── openapi.rs            hand-written OpenAPI 3 document
├── rate_limit.rs         per-email sliding-window magic-link issuance limiter
├── secret_hash.rs        SHA-256 hash-at-rest for bearer-equivalent secrets (magic-link token / session jid / CSRF token) — SEC-A9
├── models/
│   ├── users.rs           magic-link user model (+ create_passwordless, GDPR erase + find_active_by_pid, ABAC attributes_map/attrs)
│   ├── sessions.rs        opaque cookie session issue/revoke; session_data copies ABAC attrs at establishment; revoke_all_for_user for erasure (per the auth-sessions design)
│   └── _entities/         generated SeaORM entities
├── mailers/auth.rs        magic-link mailer (prod)
├── tasks/attributes.rs    `user_attributes` CLI task — operator ABAC attribute assignment (set/show/unset/clear users.attributes)
├── migration/             in-crate migrator: m20220101_000001_users, _000002_sessions, _000003_auth_events, _000004_users_deleted_at, _000005_auth_rate_limits, _000006_users_attributes, _000007_sessions_data, _000008_sessions_ttls, _000009_hash_credentials_at_rest
└── views/auth.rs          LoginResponse / CurrentResponse
config/                    development/production/test yaml (keys/ holds only a README — no committed key files)
```

## Configuration (env)

| Var | Default | Purpose |
|---|---|---|
| `TOKEN_PRIVATE_KEY_SEED` | — | Primary Ed25519 signing seed, 32 bytes base64url (no pad). Takes precedence over the file var. |
| `TOKEN_PRIVATE_KEY_FILE` | — | Path to a file holding the same base64url seed. |
| *(neither set)* | built-in `DEV_SEED` | Development-only stable keypair; never rely on it in production. |
| `TOKEN_ADDITIONAL_PUBLIC_KEYS` | — | Comma-separated base64url 32-byte Ed25519 **verify-only** public keys (rotated-out keys whose tokens are still live). See key rotation above. |
| `TOKEN_ISSUER` | `authentication-service` | `iss` claim + key-set issuer. |
| `TOKEN_AUDIENCE` | `main-x-service` | `aud` claim. |
| `TOKEN_EXPIRATION` | `300` | Access-token lifetime (seconds) — deliberately short; the cookie session is the durable thing. |
| `AUTH_SESSION_IDLE_TTL_SECS` | `1800` (30 min) | Sliding idle session TTL — bumped on each `/me`; session expires once idle. |
| `AUTH_SESSION_ABSOLUTE_TTL_SECS` | `43200` (12 h) | Hard absolute session ceiling set at issuance, never extended. |
| `AUTH_ATTRIBUTE_VOCABULARY` | — | Optional inline-JSON allow-set of ABAC attribute keys→values (`{ "access": ["read","write","admin"], "dept": ["cardiology"], "svc": [] }`; empty list ⇒ any value). Enforced on assignment (CLI + admin) to catch typos. Unset ⇒ unrestricted. |
| `AUTH_ATTRIBUTE_VOCABULARY_FILE` | — | Path form of the above (used when the inline var is unset). |
| `FRONTEND_URL` | `http://localhost:5173` | Base for the magic link in emails/logs. |
| `DATABASE_URL` | loco config default | Postgres connection. |

## When you are unsure

The spec wins. If the spec is silent, propose an update in
[`spec/index.md`](./spec/index.md) rather than guessing.

## Container image

`Dockerfile` (multi-stage, Debian 13 slim runtime) builds this crate's
production image. **Build context must be the repository root**, not
this directory — this crate's sibling path dependencies
(`integrity-mac`, `authentication-verifier`) live outside
`authentication/authentication-service-with-loco/`:

```sh
podman build -f authentication/authentication-service-with-loco/Dockerfile \
  -t authentication-service .   # run from the repository root
```

Verified end-to-end (2026-08-03): builds clean, boots against a real
Postgres (with `TOKEN_PRIVATE_KEY_SEED` and `JWT_SECRET` supplied — see
Configuration above; `TOKEN_PRIVATE_KEY_SEED` is required in
production per SEC-A1's fail-closed guard), and `GET /_health` returns
`200`. This exercise found and fixed a real bug:
`config/production.yaml`'s `mailer.smtp.auth.user`/`password` used an
unquoted Tera `{{ get_env(name="…", default="") }}` call, which renders
as YAML `null` (not `""`) when the env var is unset — loco's
`SmtpAuth` fields are `String`, not `Option<String>`, so this failed
config parsing at boot with "invalid type: unit value, expected a
string". This crate's `.gitignore` also excluded
`config/production.yaml` entirely (a loco scaffold default nobody had
removed), which is why the bug had never been caught — the file never
left this machine, so no other checkout could exercise it. Both are
fixed (the file is now tracked; see the `.gitignore` for the
reasoning). No signing key is ever baked into the image; supply it at
`podman run -e TOKEN_PRIVATE_KEY_SEED=…` or a secret-mounted file. See
`.containerignore` at the repository root (excludes every crate's
`target/`, or the build context would try to copy hundreds of GB of
build artifacts). The wired multi-service `examples/compose/` stacks
(DEP-1) that build on this are not yet written.
