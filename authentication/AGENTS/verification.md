# Verification Reference — Authentication Entity

How a peer service verifies this entity's **PASETO v4.public** bearer
tokens **offline**. This is the authentication entity's counterpart to
the sibling entities' `AGENTS/matching.md`. Source of truth for the
contract:
[`../../agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
+ entity spec [§5–§6](../spec/05-domain-model.md); source code: verifier
[`src/lib.rs`](../authentication-verifier-rust-crate/src/lib.rs) and
service
[`src/auth/mod.rs`](../authentication-service-with-loco/src/auth/mod.rs).

> The verifier crate is **already harmonized** to PASETO. The auth
> service's own issuance is **pivot in progress** — RS256 JWT + JWKS are
> decommissioned but the running binary may still emit JWTs until the
> code follow-up in the service spec §13 lands.

## The model

- A short-lived **PASETO v4.public** token is minted from a valid cookie
  session and signed with an **Ed25519 private** key; the service
  publishes the **public** key(s) at `/.well-known/paseto-keys`.
- A peer fetches the published key(s) **once at boot**, builds a
  `Verifier`, and calls `verify()` per request — pure local CPU, no
  network, no shared secret, no introspection endpoint.
- Revocation (signout / session revoke) is enforced **at the auth
  service**; peers honour a PASETO until `exp` — the documented tradeoff,
  bounded by the very short token TTL (~5 min).

## Verifier crate API

```rust
use authentication_verifier::{Verifier, Claims, VerifyError};

// Boot: from a published-key document you already have…
let verifier = Verifier::from_paseto_keys_value(&keys_json, "authentication-service", "main-x-service")?;

// …or fetch it over HTTPS (feature = "fetch"):
let verifier = Verifier::from_paseto_keys_url(
    "https://auth.example.gov/.well-known/paseto-keys",
    "authentication-service",
    "main-x-service",
).await?;

// Per request:
let claims: Claims = verifier.verify(bearer_token)?;
// claims.sub  — user pid (UUID string)
// claims.email, claims.name — convenience identity at the edge
// claims.jti  — token id (correlates the originating session)
```

Construct once, share behind an `Arc`; `verify` is read-only and
allocation-light.

### What `verify()` enforces, in order

1. **Footer `kid` present** — else `VerifyError::MissingKid`.
2. **`kid` known** — key looked up in the published-key map; else
   `VerifyError::UnknownKid(kid)` (stale cache, wrong issuer, or
   forgery).
3. **PASETO v4.public (Ed25519) signature** valid for that key.
4. **`iss`** equals the configured issuer (default
   `authentication-service`, env `JWT_ISSUER` at the service).
5. **`aud`** equals the configured audience (default
   `main-x-service`, env `JWT_AUDIENCE`).
6. **`exp`** in the future (with the default leeway).

### Published-key handling rules

- Only Ed25519 public-key entries are loaded; others are skipped.
- A key entry missing its `kid` / key material, or a malformed
  document → a key-document error.
- An **empty key set is permitted**: the verifier builds and rejects
  every token with `UnknownKid` — so a service can boot before the
  published-key source is reachable, without panicking.

### Caching and rotation

Fetch `/.well-known/paseto-keys` once at boot and cache for the process
lifetime — the auth service rotates keys rarely. Treat `UnknownKid` as
the refetch trigger: on rotation, the new key's `kid` won't be in your
cache; refetch, rebuild the `Verifier`, retry once. (Multi-key
grace-window rotation at the service is entity spec §13 T-5.)

## kid derivation (both sides)

The `kid` (carried in the PASETO footer) is derived from the public key
the same way on both sides. The service stamps it into every token
footer and the published-key document; the verifier's tests rebuild it
the same way. If you change one side, change both in the same PR — the
service crate's `tests/sign_verify_contract.rs` pins the contract and
fails on drift (entity spec §13 T-4).

## What the service itself does differently

For browser/BFF traffic the service authenticates the **cookie session**
(`__Host-mxi_session`) directly, not a PASETO. Handlers that need
revocation (`/me`) check the session is still active. Peers cannot do
that check — they trust the short-lived PASETO until `exp`, by design.

## Checklist for adding auth enforcement to a peer service

1. Add the verifier crate (path dependency; `fetch` feature if you
   want boot-time HTTP loading).
2. Build the `Verifier` at boot from the published-key URL
   (`/.well-known/paseto-keys`, via config); share via app state.
3. Write a `FromRequestParts` extractor that pulls
   `Authorization: Bearer v4.public.…`, calls `verify()`, and yields
   `Claims` — mirror the service's extractor shape.
4. Reject failures with `401`. Do not log the token.
5. Authorize locally from claims (`sub` = user pid); there are no
   role claims today.
