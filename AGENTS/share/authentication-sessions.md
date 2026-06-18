# Authentication & sessions — design

How the Main X Index family authenticates users and services **without
using JWTs for sessions**. This operationalises the principle in
[jwt.md](jwt.md) ("JWTs must not be used to keep users logged in — use
cookie sessions") for a *federated* multi-service system, and **supersedes
the RS256-JWT + JWKS access-token model** described in the
`authentication-service`, `authentication-verifier`, and per-service auth
sections (and the `*_REQUIRE_AUTH` / JWKS notes in their specs §13/§15).
It is a design document: it fixes the session model, the cross-service
token, the cookie/CSRF rules, and the rollout, so each crate adopts it
without re-litigating.

## 1. Why change

The previous design issued **RS256 JWT access tokens** from the
authentication-service, published RSA public keys at
`/.well-known/jwks.json`, and had every peer verify bearer tokens
**offline** via the `authentication-verifier` crate. The browser
front-ends held the token in `localStorage` (`mxi_access_token`).

Per [jwt.md](jwt.md): JWTs are designed for very short-lived tokens, not
sessions; "stateless" auth is not securely feasible (you need state
anyway); a JWT carrying a session is strictly worse than a session cookie;
the JWT spec family is distrusted; and tokens must not live in
`localStorage`. This design moves the **session** to a server-side
**cookie session**, and keeps a **short-lived, offline-verifiable
cross-service token** — but as **PASETO v4** (a spec security experts
trust), used only as the brief signed assertion [jwt.md](jwt.md)
explicitly blesses, never as the session itself.

## 2. Goals & non-goals

**Goals**

- The **human session** is a server-side **Postgres-backed cookie
  session** (opaque id in an httpOnly cookie); no token in browser JS.
- **Cross-service / offline** authentication is preserved (no
  per-request introspection hop) via **short-lived PASETO v4 public
  tokens** verified against a published ed25519 public key.
- Passwordless **magic-link** login is unchanged as the *mechanism*; only
  what it establishes changes (a session, not a JWT).
- CSRF protection for cookie-authenticated mutating requests.
- One uniform model across the family; the `authentication-verifier`
  crate stays the single peer-side verification library (now PASETO).

**Non-goals**

- JWT anywhere in the auth path (removed). Existing RS256/JWKS references
  in specs are superseded by this doc.
- Stateless sessions (rejected per [jwt.md](jwt.md)).
- An external session store / cache — sessions live in Postgres
  (the family avoids external brokers; see [loco.md](loco.md)).
- Long-lived bearer tokens — PASETO tokens are ~5 minutes and derived from
  the session.

## 3. The session (source of truth)

Login (magic-link verified) establishes a **server-side session** and sets
a cookie carrying only an **opaque session id**.

### `sessions` table (authentication-service; Postgres)

Reuses [postgresql-sessions-with-jsonb](postgresql-sessions-with-jsonb/index.md).

```sql
CREATE TABLE sessions (
    sid            TEXT PRIMARY KEY,        -- opaque, high-entropy (NOT a JWT)
    user_pid       UUID NOT NULL,
    data           JSONB NOT NULL DEFAULT '{}',  -- roles, scopes, MFA state, …
    created_at     TIMESTAMPTZ NOT NULL,
    last_seen_at   TIMESTAMPTZ NOT NULL,    -- sliding idle window
    idle_expires_at     TIMESTAMPTZ NOT NULL,    -- now() + idle TTL, bumped on use
    absolute_expires_at TIMESTAMPTZ NOT NULL,    -- hard cap, never extended
    revoked_at     TIMESTAMPTZ              -- explicit logout / admin revoke
);
CREATE INDEX sessions_user ON sessions (user_pid) WHERE revoked_at IS NULL;
```

- **Idle + absolute TTLs** — `idle_expires_at` slides on each use;
  `absolute_expires_at` is a hard ceiling. A session is valid iff
  `revoked_at IS NULL AND now() < idle_expires_at AND now() <
  absolute_expires_at`.
- **Rotation** — the `sid` is rotated on privilege change (login,
  step-up auth) to prevent fixation.
- **Revocation is immediate** — logout / admin action sets `revoked_at`;
  because cross-service tokens are short-lived (§5), revoking the session
  stops new tokens and existing ones expire within the window.

### Cookie attributes

```
Set-Cookie: __Host-mxi_session=<sid>; HttpOnly; Secure; SameSite=Lax; Path=/
```

- `HttpOnly` — browser JS can never read it (kills the `localStorage`
  exfiltration class).
- `Secure` + `__Host-` prefix — HTTPS-only, host-locked.
- `SameSite=Lax` for the SSO and the front-end BFF origins (`Strict`
  where no cross-site top-level navigation is needed).

## 4. CSRF

Cookie auth means CSRF protection is mandatory for **state-changing**
requests (safe methods `GET`/`HEAD` are exempt):

- `SameSite=Lax/Strict` is the first line.
- A **synchroniser / double-submit CSRF token** is required on
  `POST`/`PUT`/`PATCH`/`DELETE`: issued per session, sent in a
  non-httpOnly cookie or page payload, echoed in an `X-CSRF-Token` header,
  compared server-side.
- An `Origin`/`Referer` allow-list check backstops it.

## 5. Cross-service authentication — PASETO v4 (public)

Peers must authenticate requests **offline** (the property the JWKS design
gave us), but JWT is out. The replacement is **PASETO v4 `public`**
(Ed25519-signed, asymmetric):

```
browser ──(__Host-mxi_session cookie)──▶ front-end BFF (SvelteKit server)
front-end BFF ──(session → POST /token)──▶ authentication-service
authentication-service ──(PASETO v4.public, exp ~5 min)──▶ BFF
front-end BFF ──(Authorization: Bearer v4.public.…)──▶ entity service
entity service ──verify OFFLINE via published ed25519 public key──▶ ok
```

- **Issuance** — `POST /token` on the authentication-service exchanges a
  valid **session** for a short-lived PASETO. No long-lived token exists.
- **Format** — PASETO **v4.public** (Ed25519). Payload claims mirror the
  old `Claims` shape: `sub` (user pid), `iss`, `aud`, `iat`, `nbf`, `exp`
  (~5 min), `sid` (originating session, for revocation correlation),
  `scope`/`roles`. **Footer** carries `kid` (key id) for rotation.
- **Offline verification** — the authentication-service publishes its
  **Ed25519 public key(s)** at `/.well-known/paseto-keys` (the JWKS
  analog). Peers fetch once at boot, hold the key, and verify
  `signature`/`iss`/`aud`/`exp`/`kid` locally — **no per-request hop**,
  exactly as before, but with a trusted spec.
- **Key rotation** — multiple published keys keyed by `kid`; the footer's
  `kid` selects the verifier key. Rotating keys never requires a shared
  secret.

### `authentication-verifier` crate → PASETO verifier

The crate keeps its role (peer-side, offline, dependency-light) but its
implementation changes from RS256-JWT to **PASETO v4.public**:

- `Verifier::from_paseto_keys_value` / `from_paseto_keys_url` (behind the
  `fetch` feature) replace `from_jwks_value` / `from_jwks_url`.
- It mirrors the same `Claims` struct and verifies `kid`/`iss`/`aud`/`exp`.
- Published to crates.io; embedded by each service's `src/auth.rs`.

## 6. Front-end — the BFF pattern (no token in the browser)

The SvelteKit front-ends are independent SPAs per entity. To honour
"no token in JS" with httpOnly cookies *and* cross-origin entity APIs,
each front-end uses its **own SvelteKit server as a Backend-For-Frontend
(BFF)**:

- The **browser** talks only to its front-end's **own origin**, carrying
  the `__Host-mxi_session` cookie (httpOnly, same-site). It never holds a
  token and never calls an entity service directly.
- The **SvelteKit server** (`hooks.server.ts` / `+server.ts` /
  `+page.server.ts`) holds the session, exchanges it for a short-lived
  PASETO (§5), and calls the entity service server-side with that bearer.
- This removes `mxi_access_token` / `localStorage` usage entirely and
  removes cross-origin credentialed-cookie complexity.
- CSRF (§4) protects the browser↔BFF mutating calls.

> This reverses the prior SPA-talks-directly-to-the-API assumption in the
> front-end specs (`event.fetch` + `new ApiClient` with a client-held
> bearer). Pages that mutate now go through a server route; read-only SPA
> pages may still render client-side but fetch via the BFF.

## 7. Magic-link login (unchanged mechanism)

The passwordless email magic-link flow is unchanged as the login
*mechanism*. The difference is the **outcome**: verifying the magic link
now **creates a session row + sets the cookie** (§3) instead of returning
a JWT. The front-end BFF receives the `Set-Cookie`; the browser is logged
in via the session.

## 8. Blanket enforcement

[jwt-enforcement.md](jwt-enforcement.md) is updated in lockstep: the
coordinated blanket `/api/*` guard now requires a **valid PASETO token**
(service-to-service) or a **valid session** (BFF/browser), not a JWT. The
`*_REQUIRE_AUTH` env flag and roll-out semantics are unchanged; only the
credential it checks changes.

## 9. Rollout / migration

1. **Sessions.** Add the `sessions` table + cookie issuance to the
   authentication-service; magic-link verify creates a session (§3, §7).
2. **PASETO issuance.** Add `POST /token` (session → PASETO v4.public) and
   publish `/.well-known/paseto-keys`; keep JWKS temporarily for overlap.
3. **Verifier.** Ship `authentication-verifier` PASETO support
   (`from_paseto_keys_*`), same `Claims`; peers add it alongside JWT.
4. **Peers flip.** Each service switches `src/auth.rs` to verify PASETO;
   `*_REQUIRE_AUTH` semantics unchanged. Remove JWT/JWKS once all peers
   verify PASETO.
5. **Front-ends.** Introduce the BFF (§6); remove `mxi_access_token` /
   `localStorage`; add CSRF. Browser holds only the httpOnly cookie.
6. **Decommission JWT.** Drop RS256 signing, JWKS, and all JWT references
   once no peer or front-end depends on them.

## 10. Open questions

- **Token-exchange caching at the BFF** — cache the minted PASETO for its
  ~5-min lifetime per session vs mint per outbound call? (Lean: cache to
  expiry.)
- **PASETO library** — `rusty_paseto` (v4 local/public) is the candidate;
  confirm features + `#![forbid(unsafe_code)]` compatibility.
- **Session store sharing** — authentication-service owns `sessions`;
  peers never read it (they trust the PASETO). Confirm no peer needs
  direct session reads.
- **CSRF token transport** — double-submit cookie vs synchroniser token in
  the BFF page payload. (Lean: synchroniser token via the BFF.)
- **Immediate cross-service revocation** — rely on the ~5-min PASETO
  expiry, or add an optional `sid` deny-list peers poll? (Lean: expiry
  only; add deny-list if a hard-revoke SLA appears.)
