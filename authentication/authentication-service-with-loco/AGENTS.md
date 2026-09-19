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

**Enterprise identity federation (EV-2, opt-in).** Behind the `oidc`
Cargo feature (off by default — a build without it pulls in no extra
HTTP/JWT stack at all), this crate can act as an OIDC **relying
party**: `GET /api/auth/oidc/login` / `callback` let a deployment's own
IdP sign a human in, establishing the **exact same** session §3 already
gives a magic-link redemption — same table, same cookie, same PASETO
minting. See
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
§7a for the design and `src/oidc.rs` / `src/controllers/oidc.rs` for
the implementation. **SAML 2.0 is not implemented** — a deliberately
separate, larger piece of work given its XML-DSig signature-verification
surface, tracked as the remaining half of EV-2 rather than rushed
alongside OIDC in the same pass.

| Question | Answer |
|---|---|
| Framework | loco.rs 1.0.1 (real `Hooks`/`AppContext` boot, loco controllers, loco config, `sea-orm-migration` 2.0). |
| Auth model | Passwordless magic link → server-side cookie session. No passwords are ever checked. |
| Tokens | Cross-service: short-lived PASETO v4.public (Ed25519); public key(s) published at `/.well-known/paseto-keys` for offline verification. |
| Build | `cargo build` |
| Test | `cargo test` (DB-free: unit + contract tests); `cargo test -- --ignored` for the Postgres-backed model/request tests. |
| Lint | `cargo clippy --bins` |
| Run | `cargo loco start` (needs Postgres; see README). Works here via this crate's own `.cargo/config.toml` alias (`loco = "run --"`) when run from inside this directory — no `cargo-loco` shim needs to be installed. Same for `cargo loco db migrate`, `cargo loco task …`. If it ever doesn't resolve (e.g. invoked via `--manifest-path` from elsewhere, which does not pick up this local alias), `cargo run -- start` / `-- db migrate` / `-- task …` is the equivalent. |

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
| GET | `/api/auth/audit/recent` | Admin | System-wide authentication audit trail (newest 100). `401` no/invalid token, `403` unless `access=admin` (SEC-A2 — the rows carry emails, an enumeration oracle if left open). |
| GET | `/api/auth/account/export` | Session | GDPR right of access: the subject's data (`users` + `sessions` + `auth_events`). |
| GET | `/api/auth/account/audit` | Session | GDPR right of access: the subject's own audit trail. |
| DELETE | `/api/auth/account` | Session | GDPR right to erasure: soft-delete + anonymise + revoke sessions + audit. |
| GET | `/api/auth/admin/users/{pid}/attributes` | Admin | Show a user's ABAC subject attributes. `403` unless the caller carries `access=admin`. |
| PUT | `/api/auth/admin/users/{pid}/attributes` | Admin | Replace a user's ABAC attribute map (body `{ "attributes": { … } }`); validates keys/values, writes an `attributes_assigned` audit row. |
| GET | `/api/compliance/audit/verify` | Bearer | Recompute SHA-256/SHA-3/MAC digests over `auth_events` rows; reports any row whose content no longer matches what was stored. Any authenticated caller (not admin-gated — see note). |
| GET | `/api/auth/oidc/login` | — | **EV-2, `oidc` Cargo feature only.** Redirect to the configured IdP's authorization endpoint (PKCE + state + nonce). Accepts an optional `?return_url=` (the same allow-listed per-app knob `MagicLinkParams::return_url` gives the magic-link flow), carried through the flow cookie to the callback. `404` when the feature is not compiled in or [`AUTH_OIDC_ISSUER_URL`](#configuration-env) is unset. |
| GET | `/api/auth/oidc/callback` | — | **EV-2, `oidc` feature only.** Exchange the code, verify the ID token, map claims into `users.attributes`, then **bridge** to the front end: mint a single-use magic-link token for the now-verified user and redirect to `{frontend}/verify?token=…` — the front end's existing `/verify` BFF route (`GET /api/auth/magic-link/{token}`) does the actual session establishment, since `__Host-mxi_session` is host-locked to this service's own origin and cannot be set usefully on a cross-origin redirect. See `src/controllers/oidc.rs`'s module doc for why. `403` when no local account exists and JIT provisioning is off (the default). |
| GET | `/.well-known/paseto-keys` | — | Published Ed25519 public key(s) for offline PASETO verification. |
| GET | `/api-docs/openapi.json` | — | Hand-written OpenAPI 3 document. |
| GET | `/swagger-ui` | — | Swagger UI page (CDN assets) rendering the doc. |
| GET | `/metrics.prom` | — | Prometheus metrics (text exposition; root path, no `/api` prefix). |

> **`/api/compliance/audit/verify` requires a bearer, decided PRO-P23**
> — unlike sibling loco-idiomatic crates (e.g. case-service, behind
> `CASE_REQUIRE_AUTH`), this crate has no blanket `/api/*` guard, so
> the handler gates itself directly (`AuthUser`, `401` without a valid
> token), the same per-handler pattern every other route above uses.
> It is deliberately **not** admin-gated like `/api/auth/audit/recent`
> — the report leaks no PII (row counts and ids only), so the gate is
> about cost, not disclosure: the handler recomputes real digests over
> up to 10,000 DB rows on every call, which is CPU/DB work an
> unauthenticated caller could otherwise trigger for free. See
> `spec/index.md` §16.

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
│   ├── compliance.rs      GET /api/compliance/audit/verify — keyed integrity verification (bearer-required, not admin-gated; see spec §16)
│   ├── docs.rs            /api-docs/openapi.json + /swagger-ui
│   ├── paseto_keys.rs     published key endpoint (/.well-known/paseto-keys — Ed25519 public key set)
│   ├── metrics.rs         /metrics.prom (Prometheus text exposition)
│   └── oidc.rs            EV-2, `oidc` feature only: GET /api/auth/oidc/{login,callback} — discovery, PKCE, token exchange, ID-token verify (openidconnect crate), bridges to the front end via a magic-link token (auth.rs::verify does the actual session establishment on the front end's own origin)
├── compliance/            mac.rs (HMAC-SHA256 via the shared integrity-mac crate) + audit_integrity.rs (SHA-256/SHA-3/MAC digest + verify over auth_events)
├── metrics.rs            Prometheus registry + auth-specific counters
├── i18n.rs               dependency-light email copy catalog (en / cy)
├── oidc.rs                EV-2, `oidc` feature only: OidcConfig::from_env (federation config) + map_claims (IdP claim → ABAC attrs, vocabulary-gated) — pure, DB-free
├── openapi.rs             hand-written OpenAPI 3 document
├── rate_limit.rs          per-email sliding-window magic-link issuance limiter
├── secret_hash.rs         SHA-256 hash-at-rest for bearer-equivalent secrets (magic-link token / session jid / CSRF token) — SEC-A9
├── models/
│   ├── users.rs           magic-link user model (+ create_passwordless, GDPR erase + find_active_by_pid, ABAC attributes_map/attrs)
│   ├── sessions.rs        opaque cookie session issue/revoke; session_data copies ABAC attrs at establishment; revoke_all_for_user for erasure (per the auth-sessions design)
│   └── _entities/         generated SeaORM entities
├── mailers/auth.rs        magic-link mailer (prod)
├── tasks/attributes.rs    `user_attributes` CLI task — operator ABAC attribute assignment (set/show/unset/clear users.attributes)
├── migration/             in-crate migrator: m20220101_000001_users, _000002_sessions, _000003_auth_events, _000004_users_deleted_at, _000005_auth_rate_limits, _000006_users_attributes, _000007_sessions_data, _000008_sessions_ttls, _000009_hash_credentials_at_rest, m20260728_000001_add_auth_event_mac, m20260906_000001_sessions_source_ip, m20260906_000002_auth_events_source_ip
├── version.rs             header-based API versioning (`Accepts-version`, `require_version_mw`, layered in `after_routes`)
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
| `AUTH_INTEGRITY_MAC_KEY` | — | HMAC-SHA256 key for the `auth_events` integrity MAC (`GET /api/compliance/audit/verify`). Unset ⇒ no MAC written; affected rows report `mac_absent`, not a mismatch. |
| `AUTH_INTEGRITY_MAC_KEY_FILE` | — | Path form of the above; takes precedence over the inline var. |
| `AUTH_INTEGRITY_MAC_KEY_ID` | — | Key id stamped into new MACs, for rotation. |
| `AUTH_INTEGRITY_MAC_KEYS_RETIRED` | — | Comma-separated retired key material, still verifiable, no longer used to sign. |
| `FRONTEND_URL` | `http://localhost:5173` | Base for the magic link in emails/logs; also the redirect target after a successful OIDC sign-in. |
| `DATABASE_URL` | loco config default | Postgres connection. |
| `AUTH_OIDC_ISSUER_URL` | — | **EV-2** (`oidc` Cargo feature, off by default). The IdP's issuer URL, used for OIDC discovery. Unset ⇒ `/api/auth/oidc/*` is `404` — federation is opt-in per deployment even with the feature compiled in. |
| `AUTH_OIDC_CLIENT_ID` | — | This service's registered OAuth2 client id at the IdP. |
| `AUTH_OIDC_CLIENT_SECRET` | — | This service's client secret, inline. `AUTH_OIDC_CLIENT_SECRET_FILE` takes precedence when both are set. |
| `AUTH_OIDC_CLIENT_SECRET_FILE` | — | Path form of the above. |
| `AUTH_OIDC_REDIRECT_URL` | — | This service's own callback URL, registered with the IdP (`.../api/auth/oidc/callback`). |
| `AUTH_OIDC_CLAIM_MAP` | — | Inline JSON claim-name → attribute-key map (e.g. `{"groups":"dept"}`); every mapped key/value still passes through `AUTH_ATTRIBUTE_VOCABULARY` exactly as a CLI/admin-API assignment would. `AUTH_OIDC_CLAIM_MAP_FILE` takes precedence when both are set. |
| `AUTH_OIDC_CLAIM_MAP_FILE` | — | Path form of the above. |
| `AUTH_OIDC_JIT_PROVISIONING` | off | `1`/`true` ⇒ a first successful federated sign-in for an unknown email auto-creates a passwordless account. Default **off** — the safer of §7a's two documented leans (an IdP should not silently control account creation); a deployment opts in explicitly. |

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
