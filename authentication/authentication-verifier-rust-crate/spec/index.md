# Authentication Verifier — Specification

> **Single source of truth.** Code conforms to this spec, not the other
> way around. A behavioural change is a three-part PR: spec edit + code
> edit + test edit. Live work queue is §13; open questions are §16.
>
> Issuing service:
> [authentication-service-rust-crate](../../authentication-service-rust-crate/spec/index.md).
> Entity-level contract:
> [../../spec/index.md](../../spec/index.md).

## 1. Purpose and vision

A reusable, dependency-light Rust library that lets any Main X Index
peer service verify the authentication-service's **RS256** access
tokens **offline**: fetch the published JWKS once at boot, then verify
every bearer token locally. No shared secret, no per-request
introspection hop.

## 2. Scope

In scope: JWKS parsing (RSA signing keys), `kid`-based key selection,
RS256 signature verification, `iss` / `aud` / `exp` enforcement,
optional HTTPS JWKS fetching (`fetch` feature).

Out of scope: token issuance, sessions/revocation (auth-service only),
non-RSA algorithms, JWKS refresh scheduling (callers refetch on
`UnknownKid`), framework-specific extractors.

## 3. Stakeholders and users

Peer service crates (person / worker / place / thing / event / course /
organization / care-pathway) that accept the federation's bearer
tokens; the loco conversion uses this crate instead of re-implementing
verification per service.

## 4. Glossary

See the entity glossary
([../../spec/04-glossary.md](../../spec/04-glossary.md)): JWKS, `kid`,
`jti`/`jid`, `pid`.

## 5. Domain model — the public API contract

- **`Verifier`** — RSA keys indexed by `kid` + a `jsonwebtoken`
  `Validation` (RS256, pinned issuer and audience). Constructed once,
  shared behind an `Arc`; `verify` is read-only.
  - `from_jwks_value(&serde_json::Value, issuer, audience)` — loads
    every `kty == "RSA"` entry (requires `kid`, `n`, `e`); skips
    non-RSA entries; **permits an empty key set** (boots before the
    JWKS source is reachable; rejects everything with `UnknownKid`).
  - `from_jwks_url(url, issuer, audience)` *(feature `fetch`)* — GET
    the JWKS, then delegate to `from_jwks_value`.
  - `verify(token) -> Result<Claims, VerifyError>` — decode header →
    require `kid` → look up key → verify signature + `iss`/`aud`/`exp`.
  - `key_count() -> usize`.
- **`Claims`** — `sub` (user pid, UUID string), `email`, `name`,
  `iss`, `aud`, `exp` (unix s), `iat` (unix s), `jti` (= auth-service
  `sessions.jid`). **Byte-identical** to the service's
  `auth::Claims`; pinned by the service's cross-crate contract test.
- **`VerifyError`** — `Jwks(String)`, `MissingKid`,
  `UnknownKid(String)`, `Jwt(jsonwebtoken::errors::Error)`, and
  `Fetch(String)` (feature `fetch`).

### The JWKS / `kid` contract

- JWKS entries: `{kty: "RSA", use: "sig", alg: "RS256", kid, n, e}`
  with `n`/`e` base64url, no padding.
- `kid` = base64url (no padding) of `SHA-256(big-endian RSA modulus
  bytes)`; the service stamps the same `kid` into every token header.
- Defaults at the service: issuer `authentication-service`, audience
  `main-x-service`, token TTL 3600 s.

## 6. Functional requirements

1. A token signed by the auth-service verifies and round-trips all
   eight claims.
2. Expired tokens, wrong-audience tokens, wrong-issuer tokens,
   tampered signatures, and garbage strings are rejected via
   `VerifyError::Jwt` (or `Jwt`-wrapped decode errors).
3. A missing header `kid` yields `MissingKid`; an unmatched `kid`
   yields `UnknownKid(kid)`.
4. A JWKS without a `keys` array, or an RSA entry missing
   `kid`/`n`/`e`, yields `Jwks(...)` at construction.
5. Non-RSA JWKS entries are skipped silently; an empty key set
   constructs successfully.

## 7. Non-functional requirements

- `#![forbid(unsafe_code)]`.
- Dependency-light core: `jsonwebtoken`, `serde`, `serde_json`,
  `thiserror`; `reqwest` only behind `fetch`.
- No async in the core path; `from_jwks_url` is the only async fn.

## 8. Architecture

Single-module library (`src/lib.rs`). No I/O in the default feature
set. Callers cache the `Verifier` for the process lifetime and refetch
on `UnknownKid` to pick up key rotation (entity spec §13 T-5).

## 9. API surface

See §5. Crate name: `authentication-verifier` (lib
`authentication_verifier`).

## 10. Persistence

None. The crate is stateless; the JWKS document is the caller's input.

## 11. Testing strategy

Offline unit tests in `src/lib.rs` using a committed throwaway RSA
keypair (never used in production): round-trip, expiry, audience,
unknown-`kid`, tampered signature, garbage tokens, empty/malformed
JWKS, non-RSA skipping. The **cross-crate contract test** lives in the
service crate (`tests/sign_verify_contract.rs`) and pins the
`Claims` shape and `kid` derivation against the service's signer.

## 12. Compliance

Claims carry personal data (`email`, `name`): peers must not log them
beyond the family's GDPR posture. Verification is local, so no token
ever transits to a third party.

## 13. Tasks (live work queue)

- [x] Pin every validated claim rule with an offline unit test.
      *(2026-06-13)* `exp`, `aud`, `iss`, `kid`, signature, and
      structural failures each have a dedicated test; `iss` mismatch
      added (`wrong_issuer_is_rejected`). `nbf` is intentionally not in
      `Claims`, so it is not validated and nothing to pin.
- [x] Crate-level lints: `#![forbid(unsafe_code)]`,
      `#![warn(clippy::pedantic)]`, `#![deny(missing_docs)]` all land
      green. *(2026-06-13)*
- [ ] Refetch-on-`UnknownKid` helper (or document the pattern per
      entity spec §13 T-5 key rotation).
- [ ] Property-test the JWKS parser against fuzzed documents.

## 14. Implementation status

Done: full §5/§6 surface, `fetch` feature, offline unit tests, doc
set, packageable (`cargo package --list`). Consumed by the service's
cross-crate contract test.

## 15. Roadmap

v0.1 (here): core verification + `fetch`. v0.2: rotation ergonomics
(refetch-on-`UnknownKid`), adoption by peer services during the loco
conversion.

## 16. Open questions

- Should the crate offer an Axum extractor, or stay framework-free and
  let each service wrap it? (Currently framework-free.)
- Multiple audiences per verifier, if peers ever get distinct `aud`s.

## 17. References

- [src/lib.rs](../src/lib.rs) — implementation + rustdoc.
- [../../spec/index.md](../../spec/index.md) — entity-level contract.
- [../../AGENTS/verification.md](../../AGENTS/verification.md) — peer
  integration guide.
- RFC 7519 (JWT), RFC 7517 (JWK), RFC 7518 (JWA / RS256).

## 18. Change control

Update this spec in the same PR as any behavioural change. Bump
[CHANGELOG.md](../CHANGELOG.md) under `[Unreleased]`.
