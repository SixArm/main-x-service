# Authentication Verifier — Specification

> **Single source of truth.** Code conforms to this spec, not the other
> way around. A behavioural change is a three-part PR: spec edit + code
> edit + test edit. Live work queue is §13; open questions are §16.
>
> Issuing service:
> [authentication-service-with-loco](../../authentication-service-with-loco/spec/index.md).
> Entity-level contract:
> [../../spec/index.md](../../spec/index.md).
> Canonical design (single source of truth for the auth model):
> [authentication-sessions.md](../../../agents/share/authentication-sessions.md)
> §5 (PASETO v4 public + offline verification + this crate's shape).

## 1. Purpose and vision

A reusable, dependency-light Rust library that lets any Main X Index
peer service verify the authentication-service's short-lived
cross-service tokens **offline**. The token format is **PASETO
v4.public** (Ed25519-signed): fetch the published Ed25519 public
key(s) once at boot, then verify every bearer token locally. No shared
secret, no per-request introspection hop.

> **Pivot (v0.2.0).** This crate previously verified **RS256 JWTs**
> against a JWKS. Per
> [authentication-sessions.md](../../../agents/share/authentication-sessions.md),
> JWT is removed from the auth path: the human session is a Postgres-backed
> cookie session, and the only cross-service token is a short-lived PASETO
> v4.public. The crate keeps its **role** (peer-side, offline,
> dependency-light verification) but changes its **implementation** from
> RS256-JWT/JWKS to PASETO v4.public.

## 2. Scope

In scope: PASETO-keys parsing (Ed25519 public keys), footer-`kid`-based
key selection, PASETO v4.public signature verification, `iss` / `aud` /
`exp` / `nbf` enforcement, optional HTTPS key-set fetching (`fetch`
feature).

Out of scope: token issuance, sessions/revocation (auth-service only),
PASETO `local` (symmetric) tokens, non-Ed25519 algorithms, key-set
refresh scheduling (callers refetch on `UnknownKid`), framework-specific
extractors.

## 3. Stakeholders and users

Peer service crates (person / worker / place / thing / event / course /
organization / care-pathway) that accept the federation's bearer
tokens; the loco conversion uses this crate instead of re-implementing
verification per service.

## 4. Glossary

See the entity glossary
([../../spec/04-glossary.md](../../spec/04-glossary.md)): PASETO, `kid`,
`sid`, `pid`. **PASETO v4.public** — a versioned, Ed25519-signed,
asymmetric token format (a trusted alternative to JWT). **Footer** — the
PASETO trailer (here carrying `kid`), authenticated but not encrypted.

## 5. Domain model — the public API contract

- **`Verifier`** — Ed25519 public keys indexed by `kid` + pinned issuer
  and audience. Constructed once, shared behind an `Arc`; `verify` is
  read-only.
  - `from_paseto_keys_value(&serde_json::Value, issuer, audience)` —
    loads every published Ed25519 public-key entry (requires `kid` and
    the public-key material); skips non-Ed25519 entries; **permits an
    empty key set** (boots before the key source is reachable; rejects
    everything with `UnknownKid`).
  - `from_paseto_keys_url(url, issuer, audience)` *(feature `fetch`)* —
    GET the `/.well-known/paseto-keys` document, then delegate to
    `from_paseto_keys_value`.
  - `verify(token) -> Result<Claims, VerifyError>` — parse the PASETO
    footer → require `kid` → look up key → verify the v4.public
    signature + `iss`/`aud`/`exp`/`nbf`.
  - `key_count() -> usize`.
- **`Claims`** — `sub` (user pid, UUID string), `iss`, `aud`, `iat`
  (unix s), `nbf` (unix s), `exp` (unix s), `sid` (originating
  auth-service session, for revocation correlation), `scope`/`roles`.
  **Byte-identical** to the service's `auth::Claims`; pinned by the
  service's cross-crate contract test.
- **`VerifyError`** — `Keys(String)`, `MissingKid`,
  `UnknownKid(String)`, `Paseto(String)` (signature / claim / parse
  failure), and `Fetch(String)` (feature `fetch`).

### The PASETO-keys / `kid` contract

- The service publishes its Ed25519 public key(s) at
  `/.well-known/paseto-keys` (the JWKS analog), each entry carrying a
  `kid` and the base64url (no padding) Ed25519 public-key bytes.
- Tokens are **PASETO v4.public**; the **footer** carries the `kid` that
  selects the verifier key, so rotation never needs a shared secret.
- Defaults at the service: issuer `authentication-service`, audience
  `main-x-service`, token TTL ~300 s (5 min; derived from the session).

## 6. Functional requirements

1. A v4.public token signed by the auth-service verifies and
   round-trips all claims (`sub`, `iss`, `aud`, `iat`, `nbf`, `exp`,
   `sid`, `scope`/`roles`).
2. Expired (`exp`) tokens, not-yet-valid (`nbf`) tokens, wrong-audience
   tokens, wrong-issuer tokens, tampered tokens, and garbage strings are
   rejected via `VerifyError::Paseto`.
3. A missing footer `kid` yields `MissingKid`; an unmatched `kid`
   yields `UnknownKid(kid)`.
4. A key-set document without a key array, or an Ed25519 entry missing
   `kid` / public-key material, yields `Keys(...)` at construction.
5. Non-Ed25519 key entries are skipped silently; an empty key set
   constructs successfully.

## 7. Non-functional requirements

- `#![forbid(unsafe_code)]`.
- Dependency-light core: a PASETO v4 library (e.g. `rusty_paseto`),
  `serde`, `serde_json`, `thiserror`; `reqwest` only behind `fetch`.
- No async in the core path; `from_paseto_keys_url` is the only async fn.

## 8. Architecture

Single-module library (`src/lib.rs`). No I/O in the default feature
set. Callers cache the `Verifier` for the process lifetime and refetch
on `UnknownKid` to pick up key rotation (entity spec §13 T-5).

## 9. API surface

See §5. Crate name: `authentication-verifier` (lib
`authentication_verifier`).

## 10. Persistence

None. The crate is stateless; the published key-set document
(`/.well-known/paseto-keys`) is the caller's input.

## 11. Testing strategy

Offline unit tests in `src/lib.rs` using a committed throwaway Ed25519
keypair (never used in production): round-trip, expiry (`exp`),
not-yet-valid (`nbf`), audience, issuer, unknown-`kid`, missing-`kid`,
tampered token, garbage tokens, empty/malformed key set, non-Ed25519
skipping, and the FR4 malformed paths (entry missing `kid` / public-key
material, and unparsable public-key bytes).

The `fetch` feature is tested **offline by design**: a test exercises
the `from_paseto_keys_url` transport-error mapping (an unsupported URL
scheme must surface as `VerifyError::Fetch`, never a panic or `Keys`)
without opening a socket. A real server round-trip is deliberately
**not** exercised here — there is no mock-HTTP/wiremock dependency, to
keep the suite network-free and the crate dependency-light. The full
sign-then-fetch-then-verify round-trip is covered by the service
crate's **cross-crate contract test** (`tests/sign_verify_contract.rs`),
which also pins the `Claims` shape and footer-`kid` selection against the
service's signer.

## 12. Compliance

Claims may carry identity data (`sub`, `scope`/`roles`): peers must not
log them beyond the family's GDPR posture. Verification is local, so no
token ever transits to a third party.

## 13. Tasks (live work queue)

- [x] **PASETO v4.public pivot (code follow-up).** *(2026-06-17 —
      shipped as v0.2.0)* Replaced the
      RS256-JWT/JWKS implementation in `src/lib.rs` with PASETO
      v4.public per §5/§6 and
      [authentication-sessions.md](../../../agents/share/authentication-sessions.md)
      §5: swap the `jsonwebtoken`/RSA stack for a PASETO v4 library
      (e.g. `rusty_paseto`); rename `from_jwks_value`/`from_jwks_url`
      to `from_paseto_keys_value`/`from_paseto_keys_url`; parse the
      footer `kid`; verify the Ed25519 signature + `iss`/`aud`/`exp`/
      `nbf`; rename `VerifyError::Jwks`→`Keys` and
      `Jwt`→`Paseto`; keep the same `Claims` shape (now `sid` +
      `scope`/`roles`, no `jti`/`email`/`name`). Updated the throwaway
      keypair and all unit tests to Ed25519. Published to crates.io as
      `authentication-verifier` 0.2.
- [ ] Refetch-on-`UnknownKid` helper (or document the pattern per
      entity spec §13 T-5 key rotation).
- [ ] Property-test the PASETO-keys parser against fuzzed documents.

### Done (RS256-JWT era, superseded by the PASETO pivot)

- [x] Pin every validated claim rule with an offline unit test.
      *(2026-06-13)*
- [x] Crate-level lints: `#![forbid(unsafe_code)]`,
      `#![warn(clippy::pedantic)]`, `#![deny(missing_docs)]` all land
      green. *(2026-06-13)*
- [x] Pin the FR4 malformed-JWKS paths with targeted unit tests.
      *(2026-06-15)*
- [x] Pin the `fetch`-feature transport-error mapping offline.
      *(2026-06-15)*

## 14. Implementation status

**PASETO v4.public shipped (v0.2.0, 2026-06-17).** The shipped
`src/lib.rs` implements the PASETO v4.public surface of §5/§6
(`from_paseto_keys_*`, footer-`kid` selection, Ed25519 verification via
`rusty_paseto`); the RS256-JWT/JWKS implementation (v0.1.x) is removed.
The doc set, `fetch` feature, offline-test discipline, and
packageability (`cargo package --list`) carried over unchanged in shape.

## 15. Roadmap

v0.1 (RS256-JWT, superseded): core JWKS verification + `fetch`. **v0.2.0
(here): PASETO v4.public pivot** — `from_paseto_keys_*`, footer-`kid`
selection, Ed25519 verification; same `Claims` role. A **BREAKING**
change (see [CHANGELOG.md](../CHANGELOG.md)). Later: rotation ergonomics
(refetch-on-`UnknownKid`), adoption by peer services as each flips
`src/auth.rs` to PASETO (shared-doc §9 step 4).

## 16. Open questions

- Should the crate offer an Axum extractor, or stay framework-free and
  let each service wrap it? (Currently framework-free.)
- Multiple audiences per verifier, if peers ever get distinct `aud`s.
- ~~PASETO library choice~~ — resolved: `rusty_paseto` (v4 public,
  `default-features = false`) ships in v0.2.0 and builds under
  `#![forbid(unsafe_code)]` (shared-doc §10).

## 17. References

- [authentication-sessions.md](../../../agents/share/authentication-sessions.md)
  — canonical auth/session design; §5 is this crate's contract.
- [src/lib.rs](../src/lib.rs) — implementation + rustdoc.
- [../../spec/index.md](../../spec/index.md) — entity-level contract.
- [../../AGENTS/verification.md](../../AGENTS/verification.md) — peer
  integration guide.
- [PASETO](https://paseto.io/) — Platform-Agnostic Security Tokens;
  v4.public = Ed25519 (RFC 8032). `rusty_paseto` crate.

## 18. Change control

Update this spec in the same PR as any behavioural change. Bump
[CHANGELOG.md](../CHANGELOG.md) under `[Unreleased]`.
