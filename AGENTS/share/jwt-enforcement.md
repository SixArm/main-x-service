# Blanket auth enforcement (coordinated)

> **Superseded credential — read [authentication-sessions.md](authentication-sessions.md) first.**
> The family has moved **off JWT** (per [jwt.md](jwt.md)). The blanket
> `/api/*` guard, the `*_REQUIRE_AUTH` flag, and the rollout semantics
> below are **unchanged**, but the credential the guard checks is now a
> short-lived **PASETO v4 public token** (service-to-service) or a valid
> **cookie session** (browser via the BFF), **not a JWT**. Concretely:
> the service-side `enforce(...)` / middleware shape below is reused
> verbatim except `bearer_claims` verifies a **PASETO** token via the
> PASETO `Verifier`; and the entire **"Front-end side"** section below
> (the `mxi_access_token` / `localStorage` bearer + cross-origin fragment
> handoff) is **superseded** by the **BFF + httpOnly-cookie** model in
> [authentication-sessions.md §6](authentication-sessions.md) — browsers
> hold no token, so there is nothing to attach in JS. Treat the front-end
> mechanics here as historical.

How the Main X Index family turns on **mandatory** auth for every
`/api/*` route. This is the family-wide contract; each loco service
implements the service side identically. It supersedes the per-crate
"follow-up: blanket enforcement" notes in the service specs §13.

Applies to every entity service. The loco-idiomatic services embed
[`authentication-verifier`](../../authentication/authentication-verifier-rust-crate)
via `src/auth.rs`: **organization**, **care-pathway**, **case**,
**portfolio**. The five older api/rest-architecture services
(**person / worker / place / thing / event**) embed the same verifier in
`src/api/rest/auth.rs` (opt-in `AuthUser` extractor + `whoami`,
env-driven `<ENTITY>_PASETO_KEYS` / `_TOKEN_ISSUER` / `_TOKEN_AUDIENCE`)
and, as of 2026-07-04, carry the same default-off blanket middleware
(`<ENTITY>_REQUIRE_AUTH`, flag read at router construction — restart to
change), layered on both their Axum and loco router surfaces. Remaining
per-crate §13 item: attribute-based access control (ABAC) per
[authorization-attributes.md](authorization-attributes.md). Boot-time
HTTP key fetch landed
2026-07-04 in all nine services: set `<ENTITY>_PASETO_KEYS_URL` to fetch
the published key set once at boot (fetched set wins; fetch failure
warn-logs and falls back to the `<ENTITY>_PASETO_KEYS` env key set, so
the service always boots; no refresh loop — rotation re-fetch is
roadmap).

## Why a flag, not a flip

Enforcement is **off by default** and gated by one env var per service.
Turning it on without the front-end attaching a token would 401 every
operator request, so the two sides ship together but the *activation* is
an operations decision (set the env var once the SSO token flow is live).
Default-off also keeps the existing DB-gated request tests green: they
don't send tokens, so they must keep working until a deployment opts in.

## Service side (loco)

### Config

One boolean env var per service, read once:

| Service | Env var |
|---|---|
| organization | `ORGANIZATION_REQUIRE_AUTH` |
| care-pathway | `CARE_PATHWAY_REQUIRE_AUTH` |
| case | `CASE_REQUIRE_AUTH` |

Parsed leniently: `1`/`true`/`yes`/`on` (case-insensitive) ⇒ enabled;
anything else (incl. unset/blank) ⇒ disabled. Expose it from `src/auth.rs`
as `pub fn require_auth() -> bool` behind a `OnceLock<bool>`, mirroring
`verifier()`.

### The middleware

Add a pure decision function to `src/auth.rs` so it is unit-testable
without booting the app or a database:

```rust
/// Paths that stay public even when enforcement is on: health/ping and
/// the OpenAPI doc + Swagger UI. Everything else under `/api` requires a
/// valid bearer token.
fn is_public_path(path: &str) -> bool {
    path == "/_health"
        || path == "/_ping"
        || path == "/api-docs/openapi.json"
        || path.starts_with("/swagger-ui")
}

/// The blanket-enforcement decision. `Ok(())` ⇒ let the request through;
/// `Err((401, msg))` ⇒ reject. Pure: caller passes the flag, path, headers
/// and verifier, so it is fully unit-testable.
pub fn enforce(
    require_auth: bool,
    path: &str,
    headers: &HeaderMap,
    verifier: &Verifier,
) -> Result<(), (StatusCode, String)> {
    if !require_auth || is_public_path(path) {
        return Ok(());
    }
    bearer_claims(headers, verifier).map(|_| ())
}
```

Wire it as an `axum::middleware::from_fn` layer in the app's
`after_routes` hook (loco gives you the Axum `Router` there). Add the
layer **unconditionally** — the middleware reads `require_auth()` per
request and is a near-noop when disabled, so wiring stays static and the
flag is the only switch:

```rust
async fn require_auth_mw(req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    match auth::enforce(auth::require_auth(), &path, req.headers(), auth::verifier()) {
        Ok(()) => next.run(req).await,
        Err((status, msg)) => (status, msg).into_response(),
    }
}

// in Hooks::after_routes(router, _ctx):
let router = router.layer(axum::middleware::from_fn(require_auth_mw));
Ok(router)
```

Handlers keep taking `MaybeAuthUser` for the audit `actor`. When
enforcement is on, a request that reaches a handler is guaranteed to
carry a valid token, so `actor` is always populated; when off, behaviour
is exactly as today.

### Tests

- **Un-gated unit tests** of `enforce(...)` in `auth::tests` (reuse the
  in-module test PASETO key pair + `sign`): off + no token ⇒ `Ok`; on + public path
  ⇒ `Ok`; on + protected + no token ⇒ `401`; on + protected + valid token
  ⇒ `Ok`; on + protected + expired/tampered ⇒ `401`. Also a
  `require_auth` parser test (`"1"`/`"true"`/`"on"` ⇒ true; ``/`"0"`/junk
  ⇒ false) via a small pure `parse_bool(&str) -> bool` helper.
- **DB-gated request test** (`#[ignore]`, runs with Postgres): with the
  flag set, an un-authenticated `GET /api/<plural>` returns `401`, and the
  public `GET /api-docs/openapi.json` still returns `200`. These set the
  env var inside the test; keep them `#[serial]`.

## Front-end side (SvelteKit SPA) — SUPERSEDED

> **This whole section is superseded by [authentication-sessions.md §6](authentication-sessions.md)
> (the BFF + httpOnly-cookie model).** Under the new model the browser
> holds **no token** (no `mxi_access_token`, no `localStorage`), so there
> is no `Authorization: Bearer` to attach in JS and no cross-origin
> fragment handoff: the SvelteKit **server** holds the session cookie and
> attaches a short-lived PASETO server-side when calling an entity
> service. The text below is retained only as a record of the prior
> JWT-bearer design and MUST NOT be implemented.

Each operator front-end attaches `Authorization: Bearer <token>` to every
API request when a token is present.

### Shared token convention

The token is obtained out-of-band from the **authentication-service**
(passwordless magic-link → access token) and stored under a single shared
`localStorage` key so any operator SPA on the same origin family can read
it:

```
localStorage key: "mxi_access_token"
```

The token is obtained from the authentication front-end via the handoff
protocol below; the store reads/writes this key, and a minimal session
affordance lets an operator sign in / sign out (or paste a token).

### Token acquisition handoff (cross-origin SSO)

Each operator SPA is its own deployment, so `localStorage` is **not**
shared across origins — the token must be handed across explicitly. The
flow is OAuth-implicit-shaped (first-party federation), with a strict
allowlist so the bearer credential can never be redirected to an
untrusted site:

```
operator SPA (no token)
   │  user clicks "Sign in"
   ▼
<AUTH_FRONTEND>/signin?return_to=<absolute operator-app URL>
   │  passwordless magic-link → /verify?token=…&return_to=…
   ▼
authentication front-end verifies, issues the RS256 access token, then:
   • origin(return_to) ∈ allowlist  → redirect to  return_to#access_token=<jwt>
   • otherwise                      → ignore return_to, go to "/" (NO token appended)
   ▼
operator SPA loads, reads access_token from location.hash,
stores it under "mxi_access_token", then history.replaceState to strip it.
```

Rules:

- **Fragment, not query.** The token rides in the URL `#fragment`, which
  browsers do not send to servers (no access-log / Referer leak). The
  receiving SPA strips it with `history.replaceState` immediately.
- **Allowlist is mandatory.** The authentication front-end validates
  `origin(return_to)` against `VITE_RETURN_TO_ALLOWLIST` (comma-separated
  origins, exact `scheme://host[:port]` match). Unset/empty ⇒ same-origin
  only; a non-matching `return_to` is dropped silently and the token is
  **never** appended. This is the control that stops token exfiltration
  via a crafted `return_to`.
- **Federation key.** The authentication front-end writes the issued token
  to the shared `mxi_access_token` key (in addition to any of its own
  session bookkeeping), so a same-origin sibling needs no handoff.
- **Operator config.** Each operator SPA knows the auth front-end URL via
  `VITE_AUTH_FRONTEND_URL`; the "Sign in" affordance builds
  `${VITE_AUTH_FRONTEND_URL}/signin?return_to=${encodeURIComponent(location.origin + base)}`.
- **Hardening follow-up.** Implicit-style fragment delivery is acceptable
  for a first-party MVP with short token TTLs + the allowlist; OAuth
  auth-code + PKCE is the documented next step if these apps ever face
  third-party clients.

### Token store + client

- Add `src/lib/auth.svelte.ts` (or `session.ts`): a small reactive store
  holding the token, hydrated from `localStorage["mxi_access_token"]`,
  with `setToken(t)`, `clearToken()`, and a `token` getter. Guard
  `localStorage` for SSR/`vite preview` (`typeof localStorage !==
  "undefined"`).
- The `ApiClient` reads the current token on each request and sets the
  `Authorization` header when present, omits it when absent. (Most clients
  already accept a per-call bearer — switch them to pull from the store by
  default.)
- A minimal session UI (a field in the layout to paste/clear the token, or
  a `/session` route) so the operator can authenticate the SPA. Keep it
  dependency-light and consistent with the existing layout.

### Tests

- **vitest:** the client attaches `Authorization: Bearer <token>` when the
  store holds a token, and omits the header when it does not; `setToken` /
  `clearToken` round-trip through the store.
- **Playwright (against `vite preview`):** existing smoke tests keep
  passing (they stub the API and don't assert on auth headers). Optionally
  add a test that sets a token and asserts a request carries the header.

## Rollout order (operations)

1. Ship both sides (service middleware + front-end attachment), flag off.
2. Stand up the authentication-service session + PASETO token flow; each
   front-end BFF exchanges its session for short-lived PASETO tokens
   ([authentication-sessions.md §5–§6](authentication-sessions.md)).
3. Set `<ENTITY>_REQUIRE_AUTH=true` per service. Un-authenticated calls
   now 401; the front-end's attached token lets operator traffic through.
4. Wire the DB-gated request suites to run with the flag in CI.

## Status

Implemented (default-off) in all nine entity services as of 2026-07-04,
including the paseto-keys-over-HTTP boot fetch (`<ENTITY>_PASETO_KEYS_URL`).
Remaining operational follow-ups: activation (step 3), the DB-gated
request suites in CI (step 4), and per-crate ABAC policy authorization
(spec §13; [authorization-attributes.md](authorization-attributes.md)).
