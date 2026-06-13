# Verification Reference — Authentication Entity

How a peer service verifies this entity's RS256 bearer tokens
**offline**. This is the authentication entity's counterpart to the
sibling entities' `AGENTS/matching.md`. Source of truth for the
contract: entity spec [§5–§6](../spec/05-domain-model.md); source
code: verifier
[`src/lib.rs`](../authentication-verifier-rust-crate/src/lib.rs) and
service
[`src/auth/mod.rs`](../authentication-service-rust-crate/src/auth/mod.rs).

## The model

- The auth service signs tokens with an RSA **private** key and
  publishes the **public** key(s) at `/.well-known/jwks.json`.
- A peer fetches the JWKS **once at boot**, builds a `Verifier`, and
  calls `verify()` per request — pure local CPU, no network, no shared
  secret, no introspection endpoint.
- Revocation (signout) is enforced **only at the auth service**
  (`/me` consults the `sessions` table). Peers honour a token until
  `exp` — that is the documented tradeoff, bounded by the short
  default TTL (3600 s).

## Verifier crate API

```rust
use authentication_verifier::{Verifier, Claims, VerifyError};

// Boot: from a JWKS document you already have…
let verifier = Verifier::from_jwks_value(&jwks_json, "authentication-service", "main-x-service")?;

// …or fetch it over HTTPS (feature = "fetch"):
let verifier = Verifier::from_jwks_url(
    "https://auth.example.gov/.well-known/jwks.json",
    "authentication-service",
    "main-x-service",
).await?;

// Per request:
let claims: Claims = verifier.verify(bearer_token)?;
// claims.sub  — user pid (UUID string)
// claims.email, claims.name — convenience identity at the edge
// claims.jti  — token id (the auth service's sessions.jid)
```

Construct once, share behind an `Arc`; `verify` is read-only and
allocation-light.

### What `verify()` enforces, in order

1. **Header `kid` present** — else `VerifyError::MissingKid`.
2. **`kid` known** — key looked up in the JWKS-derived map; else
   `VerifyError::UnknownKid(kid)` (stale cache, wrong issuer, or
   forgery).
3. **RS256 signature** valid for that key.
4. **`iss`** equals the configured issuer (default
   `authentication-service`, env `JWT_ISSUER` at the service).
5. **`aud`** equals the configured audience (default
   `main-x-service`, env `JWT_AUDIENCE`).
6. **`exp`** in the future (with `jsonwebtoken`'s default leeway).

Signature/claim failures surface as `VerifyError::Jwt(_)`.

### JWKS handling rules

- Only `kty: "RSA"` entries are loaded; others are skipped.
- A JWK missing `kid` / `n` / `e`, or a document without a `keys`
  array → `VerifyError::Jwks`.
- An **empty key set is permitted**: the verifier builds and rejects
  every token with `UnknownKid` — so a service can boot before the
  JWKS source is reachable, without panicking.

### Caching and rotation

Fetch the JWKS once at boot and cache for the process lifetime — the
auth service rotates keys rarely. Treat `UnknownKid` as the refetch
trigger: on rotation, the new key's `kid` won't be in your cache;
refetch, rebuild the `Verifier`, retry once. (Multi-key grace-window
rotation at the service is entity spec §13 T-5.)

## kid derivation (both sides)

`kid = base64url_no_pad( SHA-256( RSA public modulus big-endian bytes ) )`

The service stamps it into every token header and the JWKS; the
verifier's tests rebuild it the same way. If you change one side,
change both in the same PR — the service crate's
`tests/sign_verify_contract.rs` pins the contract and fails on drift
(entity spec §13 T-4).

## What the service itself does differently

The service's own bearer extractor (`auth::AuthUser`, a plain Axum
`FromRequestParts`) verifies against the **locally held public key**
rather than the published JWKS — same algorithm, same claims, same
policy. Handlers that need revocation (`/me`) additionally check
`sessions.is_active()`. Peers cannot do that check — by design.

## Checklist for adding JWT enforcement to a peer service

1. Add the verifier crate (path dependency; `fetch` feature if you
   want boot-time HTTP loading).
2. Build the `Verifier` at boot from `AUTH_JWKS_URL` (or config);
   share via app state.
3. Write a `FromRequestParts` extractor that pulls
   `Authorization: Bearer <jwt>`, calls `verify()`, and yields
   `Claims` — mirror the service's `AuthUser` shape.
4. Reject failures with `401`. Do not log the token.
5. Authorize locally from claims (`sub` = user pid); there are no
   role claims today.
