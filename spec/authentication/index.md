# Authentication & Authorization — monorepo-wide spec

> **SUPERSEDED MODEL — read first.** The session/credential model below
> has moved off RS256 JWT + JWKS. The **source of truth** is now
> [agents/share/authentication-sessions.md](../../agents/share/authentication-sessions.md)
> (with the principle in [agents/share/jwt.md](../../agents/share/jwt.md)).
> The new model, in brief:
> - The **human session** is a server-side **Postgres-backed cookie
>   session** (opaque `sid` in an httpOnly cookie) — *not* a token. The
>   browser holds **no token** and uses **no `localStorage`**.
> - **Cross-service** auth is a short-lived (~5 min) **PASETO v4.public**
>   token (Ed25519), verified **offline** against the issuer's published
>   Ed25519 key at **`/.well-known/paseto-keys`**. This **replaces the
>   RS256 JWT + JWKS access-token model 1:1** (`/.well-known/jwks.json`,
>   `Authorization: Bearer <jwt>` → PASETO bearer).
> - Front-ends use a **BFF** (their own SvelteKit server holds the
>   session and mints/attaches the PASETO server-side).
> - `authentication-verifier` is now a **PASETO verifier**
>   (`from_paseto_keys_value` / `_url`), same `Claims` shape.
>
> The text below still describes the prior RS256/JWKS implementation as
> shipped; **code follow-up is pending**. Where it says JWT / JWKS /
> RS256 / `localStorage`, read it as superseded per the design doc above.

> **Scope.** This is the family-wide specification for **authentication
> and authorization** across the Main X Index. It is the single place
> that describes how operators sign in once, how a token is minted, how
> every peer service verifies it offline, and how `/api/*` enforcement is
> coordinated with the front-ends. It sits *above* the per-crate specs:
>
> - The **issuing service** behaviour lives in
>   [authentication-service-with-loco/spec](../../authentication/authentication-service-with-loco/spec/index.md)
>   (and its [AGENTS](../../authentication/authentication-service-with-loco/AGENTS.md)).
> - The **peer-side verification library** lives in
>   [authentication-verifier-rust-crate/spec](../../authentication/authentication-verifier-rust-crate/spec/index.md).
> - The **blanket enforcement + SSO token-handoff contract** lives in
>   [agents/share/jwt-enforcement.md](../../agents/share/jwt-enforcement.md).
>
> Code conforms to those specs; this document harmonises them. Where a
> detail is load-bearing it is restated here, but the per-crate spec
> remains the source of truth for its crate.

---

## 1. The model: one central SSO, offline verification everywhere

The family has exactly **one** authority that authenticates humans and
mints credentials: the **authentication-service**. Everything else
*verifies* — it never authenticates a human and never issues a token.

| Property | Decision | Why |
|---|---|---|
| Sign-in | **Passwordless email magic-link**. No passwords are ever set, stored, or checked. | Removes the entire password-handling attack surface (no hashing oracle, no credential stuffing, no reset flow). |
| Session | Server-side **Postgres-backed cookie session** (opaque `sid` in an httpOnly cookie). | The session is state on the server, not a token in the browser; immediately revocable; no `localStorage` exfiltration surface. |
| Cross-service credential | Short-lived **PASETO v4.public** token (Ed25519, asymmetric, ~5 min), minted from a valid session. | Peers verify with the *public* key only; the private signing key never leaves the auth service. Replaces RS256 JWT 1:1. |
| Verification | **Offline** against the published Ed25519 key at `/.well-known/paseto-keys`. No shared secret, no per-request introspection hop. | Peers stay available and fast even if the auth service is briefly down; no token ever transits to a third party. |
| Authorization | Each service authorizes **locally** from claims. The auth service issues identity, not roles. | Keeps the SSO minimal; per-entity policy stays in the entity service. |

The auth service is also the family's **reference loco.rs application**;
the loco conversion of the peer services adopts its `src/auth.rs`
verification approach via the verifier library.

```
        magic-link (passwordless)        cookie session (httpOnly)
operator ───────────────────────▶ auth-service ──────────────────▶ front-end BFF
                                      │  PASETO v4.public (Bearer)      │  (SvelteKit server
                                      │  signs with PRIVATE ed25519 key │   holds the session,
                                      ▼  publishes PUBLIC ed25519 key   ▼   mints + attaches PASETO)
                                  /.well-known/paseto-keys              │
                                      └──── fetched/held by ───▶ peer service
                                                                 verifies OFFLINE
                                                                 (kid/iss/aud/exp)
```

### 1.1 Implemented vs follow-up (family-wide)

| Area | Status |
|---|---|
| Cookie sessions + PASETO v4.public issuance + `/.well-known/paseto-keys` publication (the new model) | **Pending** — design fixed in [authentication-sessions.md](../../agents/share/authentication-sessions.md); code follow-up. Supersedes the RS256/JWKS rows below. |
| Passwordless magic-link sign-in, RS256 issuance, JWKS publication | **Implemented (being decommissioned)** (auth service v0.1). |
| `authentication-verifier` library (offline RS256, `from_jwks_value`) → **PASETO verifier** (`from_paseto_keys_value`) | **Implemented (RS256)**, published as `authentication-verifier` 0.1; PASETO migration pending. |
| Sessions + local revocation; GDPR account export/audit/erasure | **Implemented** (auth service). |
| Per-email Postgres-backed rate limiter | **Implemented** (auth service). |
| Multi-key set + zero-downtime key rotation | **Implemented** (operator-driven; no auto-rotation scheduler). |
| Blanket `/api/*` enforcement middleware (organization, care-pathway, case) | **Implemented, default-off**; activation is an ops decision. |
| Front-end **BFF** (browser holds only the httpOnly cookie; BFF mints/attaches PASETO server-side) | **Pending** — replaces the prior SPA token-attachment + `localStorage` handoff (per [authentication-sessions.md §6](../../agents/share/authentication-sessions.md)). |
| Prior front-end token attachment + cross-origin SSO handoff (RS256, `localStorage`) | **Implemented (being decommissioned)** in the operator SPAs (per [jwt-enforcement.md](../../agents/share/jwt-enforcement.md)). |
| Published-key-over-HTTP fetch (`from_paseto_keys_url`, `fetch` feature) wired into peer boot | **Follow-up** — peers will fetch `/.well-known/paseto-keys`; HTTP fetch + refetch-on-`UnknownKid` is the next step. |
| OAuth auth-code + PKCE hardening of the handoff | **Follow-up** — acceptable as implicit-style fragment delivery for a first-party MVP with short TTLs + allowlist. |
| Auto-rotation scheduler; refresh tokens; revocation propagation to peers | **Open** (auth service §16). |

---

## 2. Token issuance flow

All issuance lives in the auth service; peers never participate. The
unauthenticated endpoints (`signup`, `magic-link`) **always return
`200`** regardless of whether the email exists — the anti-enumeration
discipline (§8).

```
POST /api/auth/signup    {email, name?}   ─┐  always 200 (no enumeration)
POST /api/auth/magic-link {email}         ─┘  rate-limited (§6) → 429 over cap
        │  a magic-link token is generated, stored on the user row,
        │  and emailed (dev: logged to the tracing console — no SMTP)
        ▼
GET  /api/auth/magic-link/{token}              redeem: validate (unexpired,
        │                                       single-use), mark email verified,
        │                                       sign an RS256 access token,
        ▼                                       INSERT a `sessions` row (jid = jti)
   { token, pid, name, email, is_verified }
        │
        ├── GET  /api/auth/me        (Bearer)  current user; 401 if revoked/erased
        └── POST /api/auth/signout   (Bearer)  stamp sessions.revoked_at
```

| Step | Endpoint | Auth | Notes |
|---|---|---|---|
| Create account | `POST /api/auth/signup` | — | Passwordless. `users.password` holds an unusable random hash to satisfy `NOT NULL`. Always `200`. |
| Request link | `POST /api/auth/magic-link` | — | For an existing account. Always `200`. |
| Redeem link | `GET /api/auth/magic-link/{token}` | — | Single-use (token cleared on consume), expires after `MAGIC_LINK_EXPIRATION_MIN` (5 min). Mints the JWT + the `sessions` row. |
| Current user | `GET /api/auth/me` | Bearer | Rejects locally-revoked sessions and GDPR-erased subjects (`401`). |
| Sign out | `POST /api/auth/signout` | Bearer | Revokes the current session (the `jti`'s row). |
| JWKS | `GET /.well-known/jwks.json` | — | Public keys for offline verification. |

**Token shape.** The access token is short-lived — default **1 hour**
(`JWT_EXPIRATION=3600`). The lifetime is deliberately short because
offline verification means a revoked token stays cryptographically valid
at *peers* until it expires (§5, §9). Claims are pinned by the
cross-crate contract test (§4).

---

## 3. JWKS + key rotation

The signing/verification material is a **key set**, modelled by
`auth::AuthKeys` in
[`src/auth/mod.rs`](../../authentication/authentication-service-with-loco/src/auth/mod.rs):
one **primary** signing key plus zero or more **additional** verify-only
public keys.

| Concept | Detail |
|---|---|
| `kid` derivation | `base64url(SHA-256(big-endian RSA modulus bytes))`, no padding. Stable across restarts; identical in token headers and the JWKS. |
| Signing | `sign_access_token` always signs with the **primary** and stamps the primary's `kid` into the JWT header. |
| Verifying | `verify_token` selects the verifying key by the token header `kid` from `{primary} ∪ {additional}`. Unknown/absent `kid` ⇒ rejected (`InvalidSignature`). |
| JWKS | Publishes **all** keys (primary first) at `/.well-known/jwks.json`. |
| De-duplication | An additional key whose `kid` duplicates the primary (or a prior key) is skipped, so the JWKS has no duplicate entries. |

### 3.1 Zero-downtime rotation runbook (summary)

Because a token signed by a key that has been *demoted* to verify-only
still verifies until it expires, rotation is downtime-free:

1. Generate a new keypair. Add the **new public** key to the additional
   set (`JWT_ADDITIONAL_PUBLIC_KEY_FILES` / `_PEMS`) on every instance so
   peers (via the JWKS) begin trusting it.
2. Promote the new keypair to **primary** (`JWT_PRIVATE_KEY_FILE` /
   `JWT_PUBLIC_KEY_FILE`), moving the **old public** key into the
   additional set.
3. After the old token TTL has elapsed (≥ 1 h), drop the old public key
   from the additional set.

Full operator procedure:
[`config/keys/README.md`](../../authentication/authentication-service-with-loco/config/keys/README.md)
(auth service spec §8.4). There is no auto-rotation scheduler yet
(follow-up).

### 3.2 Environment variables

| Var | Default | Purpose |
|---|---|---|
| `JWT_PRIVATE_KEY_FILE` | `config/keys/jwt_private_dev.pem` | RSA private signing key (PEM). |
| `JWT_PUBLIC_KEY_FILE` | `config/keys/jwt_public_dev.pem` | RSA public verification key (PEM). |
| `JWT_PRIVATE_KEY_PEM` / `JWT_PUBLIC_KEY_PEM` | — | Inline PEM; takes precedence over the file vars. |
| `JWT_ADDITIONAL_PUBLIC_KEY_FILES` | — | Comma-separated paths to extra verify-only public keys (rotated-out keys whose tokens are still live). |
| `JWT_ADDITIONAL_PUBLIC_KEY_PEMS` | — | Inline verify-only public PEMs (comma/newline-separated). Combined with the files var. |
| `JWT_ISSUER` | `authentication-service` | `iss` claim + JWKS issuer. |
| `JWT_AUDIENCE` | `main-x-service` | `aud` claim — the federation audience. |
| `JWT_EXPIRATION` | `3600` | Access-token lifetime (seconds). |
| `FRONTEND_URL` | `http://localhost:5173` | Base for the magic link in emails/logs. |

The committed `config/keys/*_dev.pem` are **dev only** — a stable
committed keypair so the dev JWKS is consistent across restarts.
Production keys come from the env edges.

---

## 4. Offline verification (the verifier library)

Peers do **not** re-implement token verification. They embed the
published
[`authentication-verifier`](../../authentication/authentication-verifier-rust-crate/spec/index.md)
crate in their `src/auth.rs`. **Target:** PASETO v4.public verification
(`from_paseto_keys_value` / `from_paseto_keys_url`) against
`/.well-known/paseto-keys`, same `Claims` shape. The RS256/JWKS API
described below (`from_jwks_value` / `from_jwks_url`) is **being
decommissioned** — see
[authentication-sessions.md §5](../../agents/share/authentication-sessions.md).

| API | Behaviour |
|---|---|
| `Verifier::from_jwks_value(&json, issuer, audience)` | Load every `kty == "RSA"` entry (needs `kid`/`n`/`e`); skip non-RSA; an **empty** key set is permitted (boots before the JWKS source is reachable; rejects with `UnknownKid`). |
| `Verifier::from_jwks_url(url, issuer, audience)` *(feature `fetch`)* | GET the JWKS, then delegate to `from_jwks_value`. **Follow-up** to wire into peer boot. |
| `verify(token) -> Result<Claims, VerifyError>` | Decode header → require `kid` → look up key → check signature + `iss`/`aud`/`exp`. |

**The shared `Claims` shape** is byte-identical between the service's
`auth::Claims` and the verifier's `Claims`:

| Claim | Meaning |
|---|---|
| `sub` | user `pid` (UUID string) |
| `email`, `name` | convenience identity at the edge |
| `iss`, `aud` | issuer / audience (pinned in `Validation`) |
| `exp`, `iat` | expiry / issued-at (unix seconds) |
| `jti` | JWT id — equals `sessions.jid`, the unit of local revocation |

The auth service's
[`tests/sign_verify_contract.rs`](../../authentication/authentication-service-with-loco/spec/index.md)
**pins this contract**: a token signed by `auth::sign_access_token`
verifies through a `Verifier` built from the service's published JWKS,
all eight claims round-trip, `kid = base64url(SHA-256(modulus))` holds,
and a `kid` mismatch fails. A multi-key case asserts a verifier built
from a JWKS carrying more than the primary still verifies a
primary-signed token and rejects an absent-`kid` token. If `Claims` or
the `kid` derivation ever change, the service + verifier change in the
same PR and this test must stay green.

---

## 5. Sessions & revocation

Stateless JWTs cannot be un-issued, so the auth service keeps a
`sessions` table to make them **locally revocable**.

| Column | Role |
|---|---|
| `jid` (unique) | = the token `jti`; one row per issued token. |
| `user_pid` | the subject (UUID). |
| `expires_at` | the token `exp`. |
| `revoked_at` | `NULL` while active; stamped on signout/erasure. |
| `user_agent` | captured at issuance (audit). |

Mechanics (see
[`src/models/sessions.rs`](../../authentication/authentication-service-with-loco/src/models/sessions.rs)):

- **Issue** — `Model::issue` inserts a row at redeem time.
- **Sign out** — `POST /api/auth/signout` looks up the session by `jti`
  and stamps `revoked_at` (`ActiveModel::revoke`).
- **`/me` gate** — rejects a token whose session is revoked, and a
  GDPR-erased subject (`users.deleted_at` set), even though the token is
  still cryptographically valid until `exp`.
- **Erasure** — `revoke_all_for_user` stamps `revoked_at` on every
  active session (leaving already-revoked timestamps intact).

**Locality caveat.** Revocation is enforced where the `sessions` table
lives — the **auth service**. Peers verifying offline do **not** consult
it, so a revoked token is honoured at a peer until `exp`. The short TTL
(§3.2) bounds that staleness. Whether to propagate revocation to peers
(a short-TTL deny-list) is an open question (auth service §16).

---

## 6. Abuse resistance: per-email rate limiter

The two unauthenticated issuance endpoints are throttled per
**normalised email** (trimmed + lowercased) to bound email-bombing and
account-probing — without breaking anti-enumeration (the limiter keys on
request *volume*, never on whether the account exists).

| Parameter | Value |
|---|---|
| `MAX_REQUESTS` | 5 |
| `WINDOW` | 5 minutes (sliding) |
| Over the cap | `429 Too Many Requests` (`{"error":"rate_limited",…}`), no token, no mail. |
| Backing store | Postgres table `auth_rate_limits` (`email_key`, `requested_at TIMESTAMPTZ`). |
| Concurrency | Per-key **advisory lock** `pg_advisory_xact_lock(hashtext(email_key))`, so concurrent same-email checks are exact; different emails never contend. |
| Scale | Shared across horizontally-scaled instances (the previous in-memory map was single-process only). |
| Failure mode | **Fail-open**: a limiter DB error allows the request (logged WARN) — the surrounding handler needs the DB anyway, so failing closed would only lock out legitimate sign-ins on a blip. |

Implementation:
[`src/rate_limit.rs`](../../authentication/authentication-service-with-loco/src/rate_limit.rs)
(`check` / clock-injectable `check_at(db, key, now)` / `reset` test
helper). The advisory-lock pattern is the canonical example documented
in [postgresql §9](../postgresql/index.md) (concurrency: transactions &
advisory locks).

---

## 7. Blanket `/api/*` enforcement (the coordinated contract)

Mandatory auth on every `/api/*` route is a **family-wide contract** —
the guard now requires a valid **PASETO** token (service-to-service) or a
valid **session** (BFF/browser), not a JWT. The full text is
[agents/share/jwt-enforcement.md](../../agents/share/jwt-enforcement.md);
this section summarises it and is subordinate to it.

It applies to the loco services that already embed the verifier via
`src/auth.rs`: **organization**, **care-pathway**, **case**. The older
Axum services (person / worker / place) are a separate follow-up.

### 7.1 Service side — a flag, not a flip

Enforcement is **off by default**, gated by one env var per service.
Turning it on before the front-end attaches a token would `401` every
operator request, so the two sides ship together but *activation* is an
ops decision. Default-off also keeps the existing DB-gated request tests
(which send no token) green until a deployment opts in.

| Service | Env var |
|---|---|
| organization | `ORGANIZATION_REQUIRE_AUTH` |
| care-pathway | `CARE_PATHWAY_REQUIRE_AUTH` |
| case | `CASE_REQUIRE_AUTH` |

Parsed leniently (`1`/`true`/`yes`/`on` ⇒ enabled; anything else,
including unset, ⇒ disabled). `src/auth.rs` exposes
`require_auth() -> bool` behind a `OnceLock<bool>`, mirroring
`verifier()`.

- The decision is a **pure, unit-testable** function
  `enforce(require_auth, path, headers, verifier)`: `Ok(())` lets the
  request through; `Err((401, msg))` rejects. Public paths stay open even
  when enforcement is on: `/_health`, `/_ping`,
  `/api-docs/openapi.json`, `/swagger-ui*`.
- It is wired **unconditionally** as an `axum::middleware::from_fn` layer
  in the app's `after_routes` hook; the layer reads `require_auth()` per
  request and is a near-noop when disabled, so the flag is the only
  switch.

### 7.2 `MaybeAuthUser` vs `AuthUser`

| Extractor | Meaning |
|---|---|
| `AuthUser(Claims)` | A **valid token is required**; the request is rejected (`401`) without one. Use on endpoints that must have a subject. |
| `MaybeAuthUser` | Token optional; populates the audit **actor** when present, `None` otherwise. Handlers keep taking this so behaviour is identical whether or not enforcement is on. When enforcement *is* on, a request that reaches a handler is guaranteed to carry a valid token, so the actor is always populated. |

### 7.3 Cross-origin SSO token handoff (front-end side)

> **Superseded by the BFF pattern.** Under the new model the browser
> holds only the httpOnly `__Host-mxi_session` cookie and never a token;
> each front-end's own SvelteKit server (BFF) holds the session and mints
> the PASETO server-side. There is **no** `localStorage`, no
> `#access_token` fragment, and no `mxi_access_token` key. See
> [authentication-sessions.md §6](../../agents/share/authentication-sessions.md).
> The flow below is the **decommissioned** RS256 handoff, kept for
> reference until the code follow-up lands.

Each operator SPA is its own origin, so `localStorage` is **not** shared
across them — the token is handed across explicitly via an
OAuth-implicit-shaped, first-party federation flow:

```
operator SPA (no token)
   │  user clicks "Sign in"
   ▼
<AUTH_FRONTEND>/signin?return_to=<absolute operator-app URL>
   │  passwordless magic-link → /verify?token=…&return_to=…
   ▼
auth front-end verifies, issues the RS256 token, then:
   • origin(return_to) ∈ allowlist → redirect to return_to#access_token=<jwt>
   • otherwise                     → ignore return_to, go to "/" (NO token appended)
   ▼
operator SPA reads access_token from location.hash, stores it under
"mxi_access_token", then history.replaceState to strip it from the URL.
```

| Rule | Detail |
|---|---|
| **Fragment, not query** | The token rides in the URL `#fragment`, which browsers do not send to servers (no access-log / Referer leak). The SPA strips it immediately with `history.replaceState`. |
| **Allowlist is mandatory** | The auth front-end validates `origin(return_to)` against `VITE_RETURN_TO_ALLOWLIST` (comma-separated `scheme://host[:port]`, exact match). Unset/empty ⇒ same-origin only; a non-matching `return_to` is dropped silently and the token is **never** appended. This is the control that stops token exfiltration via a crafted `return_to`. |
| **Shared federation key** | The token is stored under one shared `localStorage` key, `mxi_access_token`, so any same-origin sibling SPA needs no handoff. The `ApiClient` reads it per request and sets `Authorization: Bearer <token>` when present. |
| **Operator config** | Each SPA knows the auth front-end via `VITE_AUTH_FRONTEND_URL` and builds `${VITE_AUTH_FRONTEND_URL}/signin?return_to=${encodeURIComponent(location.origin + base)}`. |
| **Hardening follow-up** | Implicit-style fragment delivery is acceptable for a first-party MVP with short TTLs + the allowlist; **OAuth auth-code + PKCE** is the documented next step if these apps ever face third-party clients. |

### 7.4 Rollout order

1. Ship both sides (service middleware + front-end attachment), flag off.
2. Stand up the token flow; operators obtain a token into
   `mxi_access_token`.
3. Set `<ENTITY>_REQUIRE_AUTH=true` per service.
4. Wire the DB-gated request suites to run with the flag in CI.

---

## 8. GDPR account endpoints & anti-enumeration

The auth service holds personal data (`email`, `name`) and therefore
ships GDPR subject-rights endpoints, all bearer-gated to the subject.

| Endpoint | Right | Behaviour |
|---|---|---|
| `GET /api/auth/account/export` | Access (Art. 15) | A JSON document of everything held about the caller: their `users` row + `sessions` + `auth_events`. **No** tokens, key material, password hash, or api key. |
| `GET /api/auth/account/audit` | Access (Art. 15) | The subject's **own** audit trail (per-subject counterpart to the open `/audit/recent`). |
| `DELETE /api/auth/account` | Erasure (Art. 17) | Soft-delete + anonymise: stamp `users.deleted_at`, tombstone `email` → `deleted+<pid>@invalid` and `name` → `"deleted user"`, revoke all sessions, write an `account_erased` audit row. Idempotent. After erasure the bearer token still verifies cryptographically until `exp`, but `/me` and export treat the subject as gone (`401`). |

**Anti-enumeration discipline.** `signup` and `magic-link` always return
`200` whether or not the email exists, and the rate limiter keys on
volume rather than account existence (§6), so neither the success shape
nor the throttle leaks account existence. The durable `auth_events`
trail distinguishes `unknown_email` / `rate_limited` internally, but the
HTTP response does not.

**Audit gating decision.** The system-wide `GET /api/auth/audit/recent`
is left **unauthenticated** by family convention (rows carry no tokens
or secrets; mirrors the sibling care-pathway `/audit/recent`). The
right-of-access requirement is met by the bearer-gated per-subject
`/api/auth/account/audit`, so a subject's own trail is reachable only by
that subject while the operator-facing system feed stays open. See
[auditability](../../agents/share/auditability.md) for the family audit
posture.

---

## 9. Compliance & security posture

| Control | Statement |
|---|---|
| **Asymmetric signing (PASETO v4.public / Ed25519)** | Peers hold only the public key and verify offline. There is no shared secret to leak, and a peer compromise cannot mint tokens. Do **not** reintroduce loco's symmetric HS256 helper, and do **not** reintroduce RS256 JWT (decommissioned per [authentication-sessions.md](../../agents/share/authentication-sessions.md)). |
| **No secrets in audit/logs** | `auth_events`, `sessions`, audit rows, and the GDPR export carry **no** tokens, key material, password hash, or api key. Magic-link tokens are never logged (dev links are logged as the full URL only because there is no SMTP in dev). |
| **Short TTL bounds revocation staleness** | The 1 h default access-token lifetime bounds how long a revoked or erased subject's token remains honoured at offline-verifying peers (§5). |
| **Passwordless** | No password is set, stored, or checked; the `users.password` column holds an unusable random hash purely to satisfy `NOT NULL`. |
| **Erasure as anonymisation** | Art. 17 erasure soft-deletes + anonymises rather than hard-deletes, so referential history and the audit trail keep their integrity. |
| **Personal data in claims** | `Claims` carry `email`/`name`; peers must not log them beyond the family's GDPR posture. Verification is local, so no token ever transits to a third party. |

Family compliance scope: UK DPA 2018, UK + EU GDPR, ISO/IEC 27001, and
healthcare regimes (HIPAA / NHS) for the entities that carry clinical
data — see
[compliance-for-technology](../../agents/share/compliance-for-technology.md)
and
[compliance-for-healthcare](../../agents/share/compliance-for-healthcare.md),
and the privacy posture in
[privacy](../../agents/share/privacy.md).

---

## 10. Cross-references

| Topic | Where |
|---|---|
| **Auth & sessions design (SOURCE OF TRUTH — cookie sessions + PASETO v4.public)** | [agents/share/authentication-sessions.md](../../agents/share/authentication-sessions.md) |
| Principle: JWT must not keep users logged in | [agents/share/jwt.md](../../agents/share/jwt.md) |
| Coordinated enforcement + SSO handoff contract | [agents/share/jwt-enforcement.md](../../agents/share/jwt-enforcement.md) |
| Issuing service (behaviour, endpoints, tests) | [authentication-service-with-loco/spec](../../authentication/authentication-service-with-loco/spec/index.md) · [AGENTS](../../authentication/authentication-service-with-loco/AGENTS.md) |
| Peer-side verification library | [authentication-verifier-rust-crate/spec](../../authentication/authentication-verifier-rust-crate/spec/index.md) |
| Advisory locks / transactions (rate limiter) | [spec/postgresql](../postgresql/index.md) §9 |
| Audit + event streaming posture | [agents/share/auditability.md](../../agents/share/auditability.md) |
| RESTful conventions (status codes, OpenAPI/Swagger) | [agents/share/restful.md](../../agents/share/restful.md) |
| Privacy / data masking / GDPR | [agents/share/privacy.md](../../agents/share/privacy.md) |
| Compliance (technology / healthcare) | [agents/share/compliance-for-technology.md](../../agents/share/compliance-for-technology.md) · [agents/share/compliance-for-healthcare.md](../../agents/share/compliance-for-healthcare.md) |

> **Note on sibling topic specs.** The monorepo `spec/` tree currently
> ships `postgresql/`, `data.md`, and `data-modeling.md` as dedicated
> topic docs. The `restful` / `auditability` / `compliance` / `privacy`
> topics are documented under
> [agents/share/](../../agents/share/index.md) (linked above); promoting
> them into sibling `spec/<topic>/index.md` directories is a follow-up.
> Until then, link to the `agents/share/` versions to avoid dead links.
