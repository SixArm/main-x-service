# Authentication & Authorization — monorepo-wide spec

> **Model — implemented, RS256/JWKS decommissioned.** The session/
> credential model below moved off RS256 JWT + JWKS to cookie sessions +
> PASETO v4.public. The migration **landed 2026-07-04** and RS256/JWKS
> was **fully removed repo-wide 2026-08-21** (see
> [agents/share/security.md](../../agents/share/security.md) §7 —
> `cargo deny check` is clean, and the `rsa`/`jsonwebtoken` dependency
> chain loco's JWT feature pulled in is gone). The **source of truth**
> for the design is
> [agents/share/authentication-sessions.md](../../agents/share/authentication-sessions.md)
> (with the principle in [agents/share/jwt.md](../../agents/share/jwt.md));
> the model, as shipped:
> - The **human session** is a server-side **Postgres-backed cookie
>   session** (opaque `sid` in an httpOnly cookie) — *not* a token. The
>   browser holds **no token** and uses **no `localStorage`**.
> - **Cross-service** auth is a short-lived (~5 min) **PASETO v4.public**
>   token (Ed25519), verified **offline** against the issuer's published
>   Ed25519 key at **`/.well-known/paseto-keys`**. This **replaced the
>   RS256 JWT + JWKS access-token model 1:1** (`/.well-known/jwks.json`,
>   `Authorization: Bearer <jwt>` → PASETO bearer) — there is no RS256
>   code path left to fall back to.
> - Front-ends use a **BFF** (their own SvelteKit server holds the
>   session and mints/attaches the PASETO server-side); confirmed in
>   place (e.g. `person-front-end-with-svelte/src/lib/server/{auth,session}.ts`
>   + `hooks.server.ts`) — no front-end holds `mxi_access_token` in
>   `localStorage` any more.
> - `authentication-verifier` is a **PASETO verifier**
>   (`from_paseto_keys_value` / `_url`), same `Claims` shape, now also
>   carrying the shared **ABAC** policy engine (§7a below).
>
> The text below has been rewritten to describe the **PASETO model as
> shipped**. Historical RS256/JWKS mechanics are kept only where useful
> as a record of the superseded design, and are clearly marked
> **(historical, removed)**.

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
| Cookie sessions + PASETO v4.public issuance + `/.well-known/paseto-keys` publication (the model) | **Implemented** — rolled out 2026-07-04 per [authentication-sessions.md](../../agents/share/authentication-sessions.md). `POST /api/auth/token` mints from a valid session; supersedes RS256/JWKS entirely. |
| RS256 JWT issuance + `/.well-known/jwks.json` publication (prior model) | **Removed** (historical) — decommissioned 2026-08-21 alongside the `rsa`/`jsonwebtoken` dependency chain ([security.md](../../agents/share/security.md) §7). No code path remains. |
| `authentication-verifier` library — PASETO verifier (`from_paseto_keys_value` / `_url`) + shared ABAC policy engine | **Implemented**, `authentication-verifier` 0.3+ (the `attrs` claim + `abac` module landed 2026-07-05; see [authorization-attributes.md](../../agents/share/authorization-attributes.md)). The RS256/`from_jwks_value` API is gone. |
| Sessions + local revocation; GDPR account export/audit/erasure | **Implemented** (auth service). |
| Per-email Postgres-backed rate limiter | **Implemented** (auth service). |
| Multi-key set + zero-downtime key rotation | **Implemented** (operator-driven; no auto-rotation scheduler). |
| Blanket `/api/*` enforcement middleware, `<ENTITY>_REQUIRE_AUTH` | **Implemented, default-off, on all ten entity registries** (person, worker, place, thing, event, course, organization, care-pathway, case, portfolio) — not limited to the three loco-native crates; activation is an ops decision. See [jwt-enforcement.md](../../agents/share/jwt-enforcement.md) and [security.md](../../agents/share/security.md) §4. |
| ABAC authorization (subject/action/resource attributes, default read-allow/mutation-deny policy) | **Implemented**, family-wide since 2026-07-05 — see §7a below and [authorization-attributes.md](../../agents/share/authorization-attributes.md). |
| Front-end **BFF** (browser holds only the httpOnly cookie; BFF mints/attaches PASETO server-side) | **Implemented** — replaced the prior SPA token-attachment + `localStorage` handoff (per [authentication-sessions.md §6](../../agents/share/authentication-sessions.md)); confirmed in the operator SPAs' `hooks.server.ts` / `src/lib/server/`. |
| Prior front-end token attachment + cross-origin SSO handoff (RS256, `localStorage`) | **Removed** (historical) — no `mxi_access_token` / `localStorage` remains in the operator SPAs. |
| Published-key-over-HTTP fetch (`from_paseto_keys_url`, `fetch` feature) wired into peer boot | **Implemented** — peers fetch `/.well-known/paseto-keys` at boot with an env-injected fallback so a service always boots (per [authentication-sessions.md §5](../../agents/share/authentication-sessions.md)). |
| OAuth auth-code + PKCE hardening of the handoff | Moot — the fragment-delivery SPA handoff this applied to was removed with the BFF migration. |
| Auto-rotation scheduler; refresh tokens; revocation propagation to peers | **Open** (auth service §16). |

---

## 2. Token issuance flow

All issuance lives in the auth service; peers never participate. The
unauthenticated endpoints (`signup`, `magic-link`) **always return
`200`** regardless of whether the email exists — the anti-enumeration
discipline (§8). Verifying a magic link establishes a **session**, not a
token; a token is minted **separately**, from that session, on demand.

```
POST /api/auth/signup    {email, name?}   ─┐  always 200 (no enumeration)
POST /api/auth/magic-link {email}         ─┘  rate-limited (§6) → 429 over cap
        │  a magic-link token is generated, stored on the user row,
        │  and emailed (dev: logged to the tracing console — no SMTP)
        ▼
GET  /api/auth/magic-link/{token}              redeem: validate (unexpired,
        │                                       single-use), mark email verified,
        │                                       INSERT a `sessions` row +
        ▼                                       Set-Cookie __Host-mxi_session
   session established (httpOnly cookie); no token yet
        │
        ├── POST /api/auth/token     (session + CSRF)  mint a short-lived
        │                                               PASETO v4.public bearer
        │                                               (~5 min), carrying the
        │                                               session's ABAC `attrs`
        ├── GET  /api/auth/me        (session)  current user; 401 if revoked/erased
        └── POST /api/auth/signout   (session)  stamp sessions.revoked_at
```

| Step | Endpoint | Auth | Notes |
|---|---|---|---|
| Create account | `POST /api/auth/signup` | — | Passwordless. `users.password` holds an unusable random hash to satisfy `NOT NULL`. Always `200`. |
| Request link | `POST /api/auth/magic-link` | — | For an existing account. Always `200`. |
| Redeem link | `GET /api/auth/magic-link/{token}` | — | Single-use (token cleared on consume), expires after `MAGIC_LINK_EXPIRATION_MIN` (5 min). Establishes the server-side session + `__Host-mxi_session` cookie; mints no bearer token. |
| Mint bearer | `POST /api/auth/token` | Session + CSRF | Exchanges the session for a short-lived PASETO v4.public bearer (~5 min), carrying the session's ABAC `attrs` claim (§7a). Requires `X-CSRF-Token` to match the session's synchroniser token (`403` on mismatch). |
| Current user | `GET /api/auth/me` | Session | Rejects locally-revoked sessions and GDPR-erased subjects (`401`). |
| Sign out | `POST /api/auth/signout` | Session | Revokes the current session. |
| Published keys | `GET /.well-known/paseto-keys` | — | Ed25519 public key(s) for offline PASETO verification. |

**Token shape.** The access token is short-lived — default **5 minutes**
(`TOKEN_EXPIRATION=300`, `auth::DEFAULT_EXPIRATION_SECS`). The lifetime
is deliberately short because offline verification means a revoked
token stays cryptographically valid at *peers* until it expires (§5,
§9). Claims are pinned by the cross-crate contract test (§4).

---

## 3. PASETO key set + key rotation

The signing/verification material is a **key set**, modelled by
`auth::AuthKeys` in
[`src/auth/mod.rs`](../../authentication/authentication-service-with-loco/src/auth/mod.rs):
one **primary** Ed25519 signing key plus zero or more **additional**
verify-only Ed25519 public keys. (Historical note: this section
formerly described an RSA/RS256 + JWKS key set; that model is removed
— see the top-of-file note.)

| Concept | Detail |
|---|---|
| `kid` derivation | `base64url(SHA-256(Ed25519 public key bytes))`, no padding. Stable across restarts; identical in the token footer and the published key set. |
| Signing | `sign_access_token` always signs with the **primary** (PASETO v4.public) and stamps the primary's `kid` into the token footer. |
| Verifying | `verify_token` selects the verifying key by the token footer `kid` from `{primary} ∪ {additional}`. Unknown/absent `kid` ⇒ rejected. |
| Published key set | Publishes **all** keys (primary first) at `GET /.well-known/paseto-keys`, in the OKP/Ed25519 JWK form `authentication-verifier` parses. |
| De-duplication | An additional key whose `kid` duplicates the primary (or a prior key) is skipped, so the published set has no duplicate entries. |
| `key_count()` | Number of usable verification keys in the set (1 primary + additional) — exposed for health checks. |

### 3.1 Zero-downtime rotation runbook (summary)

Because a token signed by a key that has been *demoted* to verify-only
still verifies until it expires, rotation is downtime-free:

1. Generate a new Ed25519 keypair. Add the **new public** key to the
   additional set (`TOKEN_ADDITIONAL_PUBLIC_KEYS`) on every instance so
   peers (via the published key set) begin trusting it.
2. Promote the new keypair to **primary** (`TOKEN_PRIVATE_KEY_SEED` /
   `TOKEN_PRIVATE_KEY_FILE`), moving the **old public** key into the
   additional set.
3. After the old token TTL has elapsed (≥ 5 min, the default
   `TOKEN_EXPIRATION`), drop the old public key from the additional set.

Full operator procedure:
[`config/keys/README.md`](../../authentication/authentication-service-with-loco/config/keys/README.md)
(auth service spec §8.4) and the family runbook
[`agents/share/runbooks/paseto-key-rotation.md`](../../agents/share/runbooks/paseto-key-rotation.md).
There is no auto-rotation scheduler yet (follow-up, §16).

### 3.2 Environment variables

| Var | Default | Purpose |
|---|---|---|
| `TOKEN_PRIVATE_KEY_SEED` | — | Primary Ed25519 signing seed, 32 bytes base64url (no padding). Takes precedence over the file var. **Required in production** — a fail-closed guard refuses the dev fallback outside an explicitly non-production environment (SEC-A1, [security.md](../../agents/share/security.md) §5). |
| `TOKEN_PRIVATE_KEY_FILE` | — | Path to a file holding the same base64url seed. |
| *(neither set)* | built-in `DEV_SEED` | Dev-only stable keypair (`auth::DEV_SEED`) so `cargo test` and local dev run offline and deterministically; refused in production. |
| `TOKEN_ADDITIONAL_PUBLIC_KEYS` | — | Comma-separated base64url 32-byte Ed25519 **verify-only** public keys (rotated-out keys whose tokens are still live). |
| `TOKEN_ISSUER` | `authentication-service` | `iss` claim + published key-set issuer. |
| `TOKEN_AUDIENCE` | `main-x-service` | `aud` claim — the federation audience. |
| `TOKEN_EXPIRATION` | `300` | Access-token lifetime (seconds) — deliberately short (~5 min); the cookie session is the durable thing. |
| `FRONTEND_URL` | `http://localhost:5173` | Base for the magic link in emails/logs. |

No key files are committed to the repo (unlike the retired RS256 dev
PEMs); the dev fallback is a built-in constant, not a checked-in
keypair. Production keys come from the env edges, and
`TOKEN_PRIVATE_KEY_SEED` is mandatory there (§9).

---

## 4. Offline verification (the verifier library)

Peers do **not** re-implement token verification. They embed the
published
[`authentication-verifier`](../../authentication/authentication-verifier-rust-crate/spec/index.md)
crate in their `src/auth.rs`, verifying **PASETO v4.public**
(`from_paseto_keys_value` / `from_paseto_keys_url`) against
`/.well-known/paseto-keys`. The prior RS256/JWKS API
(`from_jwks_value` / `from_jwks_url`) has been **removed** — see
[authentication-sessions.md §5](../../agents/share/authentication-sessions.md).

| API | Behaviour |
|---|---|
| `Verifier::from_paseto_keys_value(&json, issuer, audience)` | Load every Ed25519 key entry (needs `kid` + public-key bytes); an **empty** key set is permitted (boots before the key source is reachable; rejects with `UnknownKid`). A key entry naming an algorithm this build doesn't implement is kept, not dropped, and a token naming it is refused with `UnsupportedAlgorithm`. |
| `Verifier::from_paseto_keys_url(url, issuer, audience)` *(feature `fetch`)* | GET the published key set over **HTTPS only** (loopback excepted), time-bounded and size-capped (SEC-V1), then delegate to `from_paseto_keys_value`. Wired into peer boot with an env-injected fallback (`<ENTITY>_PASETO_KEYS`) so a peer always boots even if the fetch fails. |
| `verify(token) -> Result<Claims, VerifyError>` | Decode the PASETO → require footer `kid` → look up key → check signature + `iss`/`aud`/`exp`. |
| `Policy::evaluate[_with_resource/_context](claims, action, entity, …)` | The shared ABAC engine (verifier 0.3+) — see §7a. |

**The shared `Claims` shape** is byte-identical between the service's
`auth::Claims` and the verifier's `Claims`:

| Claim | Meaning |
|---|---|
| `sub` | user `pid` (UUID string) |
| `email`, `name` | convenience identity at the edge |
| `iss`, `aud` | issuer / audience (pinned in verification) |
| `exp`, `iat`, `nbf` | expiry / issued-at / not-before (unix seconds) |
| `sid` | originating server-side session id (`sessions.jid`) — enables correlation and revocation |
| `scope`, `roles` | deprecated for authorization (kept on the wire); ignored by the ABAC guard |
| `attrs` | ABAC subject attributes, `BTreeMap<String, Vec<String>>` — absent/empty on old tokens (§7a) |

The auth service's
[`tests/sign_verify_contract.rs`](../../authentication/authentication-service-with-loco/tests/sign_verify_contract.rs)
**pins this contract**: a token signed by `auth::sign_access_token`
verifies through a `Verifier` built from the service's published key set
at `/.well-known/paseto-keys`, every claim round-trips, `kid =
base64url(SHA-256(Ed25519 public key bytes))` holds, and the `kid`
contract holds (a mismatch fails). A multi-key case
asserts a verifier built from a key set carrying more than the primary
still verifies a primary-signed token and rejects an absent-`kid`
token; the ABAC `attrs` claim round-trips (a non-empty map round-trips,
an empty map is omitted on the wire yet still verifies to `{}`). If
`Claims` or the `kid` derivation ever change, the service + verifier
change in the same PR and this test must stay green.

---

## 5. Sessions & revocation

The **session**, not the bearer token, is the durable, revocable thing
(the model [jwt.md](../../agents/share/jwt.md) argues for): the auth
service keeps a `sessions` table, and a short-lived PASETO minted from a
session cannot itself be un-issued, so revoking the session is what
stops new tokens (existing ones still expire within their own ~5-minute
window — see the locality caveat below).

| Column | Role |
|---|---|
| `jid` (unique) | the session id — stamped into every token minted from this session as the `sid` claim; one row per session. |
| `user_pid` | the subject (UUID). |
| `data` | `JSONB` — the session's copy of the user's ABAC attributes (`attrs`, §7a) plus other session-scoped state. |
| `idle_expires_at` | sliding idle-window deadline, bumped on each `/me`; the session expires once idle this long. |
| `absolute_expires_at` | hard ceiling set at issuance, never extended. |
| `last_seen_at` | last activity timestamp (bumps `idle_expires_at`). |
| `revoked_at` | `NULL` while active; stamped on signout/erasure/attribute change. |
| `user_agent` | captured at issuance (audit). |

Mechanics (see
[`src/models/sessions.rs`](../../authentication/authentication-service-with-loco/src/models/sessions.rs)):

- **Issue** — `Model::issue` inserts a row at magic-link redemption,
  setting the idle/absolute TTLs and copying the user's ABAC attributes
  into `data`.
- **Sign out** — `POST /api/auth/signout` looks up the session and
  stamps `revoked_at` (`ActiveModel::revoke`).
- **`/me` gate** — rejects a revoked, idle-expired, or absolute-expired
  session (`is_active`), and a GDPR-erased subject (`users.deleted_at`
  set); slides `idle_expires_at` forward on success.
- **Erasure** — `revoke_all_for_user` stamps `revoked_at` on every
  active session (leaving already-revoked timestamps intact).
- **Attribute change** — an operator changing a user's ABAC attributes
  revokes that user's sessions, so a newly-minted token reflects the
  new attributes rather than stale ones (SEC-A8).

**Locality caveat.** Revocation is enforced where the `sessions` table
lives — the **auth service**. Peers verifying a PASETO offline do
**not** consult it, so a token minted before revocation is honoured at
a peer until its own `exp` (~5 minutes, §3.2) — the session itself stops
issuing new tokens immediately. Whether to propagate revocation to
peers (a short-TTL deny-list) is an open question (auth service §16).

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
the guard requires a valid **PASETO** token (service-to-service) or a
valid **session** (BFF/browser), never a JWT. The full text is
[agents/share/jwt-enforcement.md](../../agents/share/jwt-enforcement.md);
this section summarises it and is subordinate to it.

**It is shipped on all ten entity registries** — person, worker, place,
thing, event, course, organization, care-pathway, case, and portfolio —
not only the three loco-native crates that embed the verifier first.
Every registry's `src/app.rs` layers the guard (via `src/auth.rs`) and
carries an `<ENTITY>_REQUIRE_AUTH` flag and a `tests/enforcement.rs`
suite; see [security.md](../../agents/share/security.md) §4. The
authentication-service itself is the one exception: it has **no**
blanket `/api/*` guard of its own (there is nothing else to gate — it
*is* the issuer), so each of its handlers authorises itself directly
(session or `AuthUser`/`access=admin`, per the API table in its own
`AGENTS.md`).

### 7.1 Service side — a flag, not a flip

Enforcement is **off by default**, gated by one env var per service.
Turning it on before the front-end attaches a token would `401` every
operator request, so the two sides ship together but *activation* is an
ops decision — the **default-off exposure pin** (SEC-G8) makes turning
it on for an exposed deployment a tracked release gate, not an
afterthought. Default-off also keeps the existing DB-gated request tests
(which send no token) green until a deployment opts in.

| Service | Env var |
|---|---|
| person | `PERSON_REQUIRE_AUTH` |
| worker | `WORKER_REQUIRE_AUTH` |
| place | `PLACE_REQUIRE_AUTH` |
| thing | `THING_REQUIRE_AUTH` |
| event | `EVENT_REQUIRE_AUTH` |
| course | `COURSE_REQUIRE_AUTH` |
| organization | `ORGANIZATION_REQUIRE_AUTH` |
| care-pathway | `CARE_PATHWAY_REQUIRE_AUTH` |
| case | `CASE_REQUIRE_AUTH` |
| portfolio | `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH` |

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

### 7.3 Cross-origin SSO token handoff (front-end side) — historical, removed

> **Superseded by the BFF pattern — this section is history, not the
> live design.** Under the shipped model the browser holds only the
> httpOnly `__Host-mxi_session` cookie and never a token; each
> front-end's own SvelteKit server (BFF) holds the session and mints the
> PASETO server-side (confirmed live, e.g.
> `person-front-end-with-svelte/src/lib/server/{auth,session}.ts` +
> `hooks.server.ts`). There is **no** `localStorage`, no
> `#access_token` fragment, and no `mxi_access_token` key anywhere in the
> operator SPAs. See
> [authentication-sessions.md §6](../../agents/share/authentication-sessions.md).
> The flow below is the **removed** RS256 handoff, kept only as a record
> of the superseded design.

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

### 7.4 Rollout order (historical — completed)

1. Ship both sides (service middleware + front-end attachment), flag off.
2. ~~Stand up the token flow; operators obtain a token into
   `mxi_access_token`.~~ Superseded — the front-end now runs the BFF
   pattern (§7.3): the browser never holds a token, the SvelteKit server
   mints it server-side per request from the session.
3. Set `<ENTITY>_REQUIRE_AUTH=true` per service — remains a **per-deployment
   activation decision** (default-off, §7.1); the flag and the DB-gated
   test wiring below are shipped and ready for any service that opts in.
4. DB-gated request suites are wired to run **with** the flag exercised
   in each crate's `tests/enforcement.rs`; whether a live deployment
   itself runs with the flag on is still an ops decision.

---

## 7a. Authorization — ABAC (attribute-based access control)

Authentication (§1–§7) answers "who is this caller"; **authorization**
answers "what may this caller do". The family's live authorization model
is **ABAC**, not a fixed role list — decisions are policy evaluations
over **attributes** of the subject, the action, and the resource. It
shipped family-wide **2026-07-05** via `authentication-verifier` 0.3's
`attrs` claim + a shared policy engine. Full design:
[agents/share/authorization-attributes.md](../../agents/share/authorization-attributes.md);
summary here.

**Attribute model.**

- **Subject attributes** ride the PASETO `attrs` claim (§4) — a
  string→strings map, e.g. `{"access": ["write"], "dept":
  ["cardiology"], "svc": ["true"]}` — sourced from `users.attributes`
  (this service) and copied into the session at establishment.
- **Action** is derived per request from the HTTP method: `read` (GET/
  HEAD/OPTIONS), `write` (POST/PUT/PATCH), `delete` (DELETE), or
  `destructive` (DELETE plus each crate's destructive named POSTs —
  merge, batch deduplicate, bulk import).
- **Resource attributes** (record-level, e.g. a case's classification)
  and **environment attributes** (e.g. after-hours) are opt-in,
  handler-level extensions (`evaluate_with_resource` /
  `evaluate_with_context`) layered on top of the coarse guard (§7).

**Default policy** (used when no policy is configured, and the starting
point for every deployment): **read-allow, mutation-deny** — any
authenticated subject may read; every non-read action needs an explicit
`allow` rule (`svc=true` ⇒ everything; `access=admin` ⇒
destructive+write; `access=write` ⇒ write). `401` = missing/invalid
credential; `403` = valid credential, policy denied.

**Sourcing (this service).** `users.attributes` (Postgres `JSONB`,
default `{}`) holds the assignable map. Two operator surfaces assign it:
the `user_attributes` CLI task (`src/tasks/attributes.rs`:
`op:show|set|unset|clear`) and the admin HTTP API (`GET`/`PUT
/api/auth/admin/users/{pid}/attributes`, `src/controllers/admin.rs`,
gated by an `access=admin` caller). Both write an `attributes_assigned`
`auth_events` audit row and validate against the optional
`AUTH_ATTRIBUTE_VOCABULARY[_FILE]` allow-set. Until assigned, a user has
`{}` (read-only under the default policy). ABAC checks apply **only**
when the consuming service's `<ENTITY>_REQUIRE_AUTH` flag (§7) is on —
same activation gate, same ops decision.

Each downstream service loads its own policy
(`<ENTITY>_ABAC_POLICY[_FILE]`, hot-reloadable) and evaluates it with the
shared engine; this service's role is limited to minting the `attrs`
claim and hosting attribute assignment — it does not itself gate other
services' authorization decisions.

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
is **admin-gated** (`401` no/invalid token, `403` unless `access=admin`)
— **not** left open. This reverses the crate's earlier "left
unauthenticated by family convention" design: an audit-recent feed on
*this* service carries **emails**, so leaving it open was itself an
enumeration oracle (SEC-A2, fixed; see
[security.md](../../agents/share/security.md) §2). The sibling
care-pathway `/audit/recent` stays open because its rows carry no PII —
the two services differ precisely because their audit rows carry
different sensitivity, not because of an inconsistency to close. The
right-of-access requirement is separately met by the bearer-gated
per-subject `/api/auth/account/audit`, so a subject's own trail is
reachable by that subject regardless of admin status. See
[auditability](../../agents/share/auditability.md) for the family audit
posture.

---

## 9. Compliance & security posture

| Control | Statement |
|---|---|
| **Asymmetric signing (PASETO v4.public / Ed25519)** | Peers hold only the public key and verify offline. There is no shared secret to leak, and a peer compromise cannot mint tokens. Do **not** reintroduce loco's symmetric HS256 helper, and do **not** reintroduce RS256 JWT (decommissioned per [authentication-sessions.md](../../agents/share/authentication-sessions.md)). |
| **No secrets in audit/logs** | `auth_events`, `sessions`, audit rows, and the GDPR export carry **no** tokens, key material, password hash, or api key. Magic-link tokens are never logged (dev links are logged as the full URL only because there is no SMTP in dev). |
| **Short TTL bounds revocation staleness** | The 5-minute default access-token lifetime (`TOKEN_EXPIRATION`) bounds how long a revoked or erased subject's token remains honoured at offline-verifying peers (§5). |
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
| **Authorization (SOURCE OF TRUTH — ABAC: attrs claim + policy language)** | [agents/share/authorization-attributes.md](../../agents/share/authorization-attributes.md) |
| Cross-cutting security invariants + the `REQUIRE_AUTH` activation gate | [agents/share/security.md](../../agents/share/security.md) |
| Issuing service (behaviour, endpoints, tests) | [authentication-service-with-loco/spec](../../authentication/authentication-service-with-loco/spec/index.md) · [AGENTS](../../authentication/authentication-service-with-loco/AGENTS.md) |
| Peer-side verification library | [authentication-verifier-rust-crate/spec](../../authentication/authentication-verifier-rust-crate/spec/index.md) |
| Advisory locks / transactions (rate limiter) | [spec/postgresql](../postgresql/index.md) §9 |
| Audit + event streaming posture | [agents/share/auditability.md](../../agents/share/auditability.md) |
| RESTful conventions (status codes, OpenAPI/Swagger) | [agents/share/restful.md](../../agents/share/restful.md) |
| Privacy / data masking / GDPR | [agents/share/privacy.md](../../agents/share/privacy.md) |
| Compliance (technology / healthcare) | [agents/share/compliance-for-technology.md](../../agents/share/compliance-for-technology.md) · [agents/share/compliance-for-healthcare.md](../../agents/share/compliance-for-healthcare.md) |

> **Note on sibling topic specs.** `restful`, `auditability`,
> `compliance`, and `privacy` each now also have their own
> `spec/<topic>/index.md` (alongside `postgresql/`, `data.md`, and
> `data-modeling.md`) — the promotion this note used to describe as a
> follow-up has happened. The `agents/share/` versions linked above
> remain the shorter briefs / underlying design docs; the `spec/<topic>/`
> versions are the monorepo-wide references in the same style as this
> file.
