# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md) — single source of truth;
> [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

### Fixed

- Formatting drift in `src/lib.rs` (six spots not rustfmt-formatted);
  `cargo fmt --check` is clean again. No behaviour change.

## [0.4.0] - 2026-07-05

> **Record-level resource attributes (additive).** Per
> [authorization-attributes.md](../../agents/share/authorization-attributes.md)
> §9, the engine can now feed **attributes of the specific target
> record** into a decision, so a deployment can express e.g. "deny write
> on a high-sensitivity record unless `access=admin`". Additive — the
> existing `Policy::evaluate` is unchanged in behaviour and signature.

### Added

- **`Policy::evaluate_with_resource(claims, action, entity, resource)`**
  — like `evaluate`, plus a `&BTreeMap<String, Vec<String>>` of
  **resource attributes** describing the loaded record. `Policy::evaluate`
  now delegates to it with an empty map.
- **`resource.<name>` `when` namespace** — a rule condition keyed
  `resource.sensitivity` matches the resource attribute `sensitivity`
  (prefix stripped). The namespace is disjoint from subject attributes,
  so a caller cannot spoof a resource attribute through its token; under
  the plain `evaluate` (no record loaded) every `resource.*` key resolves
  empty, keeping the coarse blanket-guard path sound.
- Engine tests: record-sensitivity deny gated below an admin allow;
  `resource.*` resolves empty without resource attrs (delegation
  identity); negated resource values; namespace disjointness.

## [0.3.0] - 2026-07-05

> **ABAC — the crate becomes the family's shared authorization
> foundation.** Per the canonical design
> [authorization-attributes.md](../../agents/share/authorization-attributes.md),
> the nine entity services move from planned per-crate roles/RBAC to
> **attribute-based access control**: verified claims carry subject
> attributes, and this crate ships the one shared, pure policy engine
> their blanket `/api/*` guards call. Additive — no breaking changes;
> pre-0.3 tokens verify unchanged.

### Added

- **`Claims.attrs`** — subject attributes for ABAC, a
  `BTreeMap<String, Vec<String>>` minted by the auth-service (e.g.
  `access: ["write"]`, `svc: ["true"]`). `#[serde(default)]`: absent on
  old tokens ⇒ empty map, no re-issue needed; omitted from the wire
  when empty.
- **`abac` module** — the shared policy engine (re-exported at the
  crate root): `Policy` / `Rule` / `Action` / `ActionPattern` /
  `Effect` / `Decision`, with `Policy::from_json` (config loading),
  `Policy::default_policy` (the built-in §5 default: `svc=true` ⇒
  everything, `access=admin` ⇒ destructive+write, `access=write` ⇒
  write), and `Policy::evaluate(claims, action, entity)` —
  first-match-wins over ordered allow/deny rules, defaulting to
  allow-read / deny-mutation. Pure data + pure evaluation: no I/O, no
  clock, no panics on any input. Supports `!`-negated values, the `*`
  action wildcard, delete-implies-destructive matching, and the
  reserved pseudo-attributes `sub` / `email` / `entity` (not
  shadowable by `attrs`).
- Tests: engine unit suite (matching, negation, first-match, defaults,
  unknown-field tolerance, malformed-policy errors) plus wire pins —
  `attrs` round-trips mint→verify; a pre-0.3 token without `attrs`
  verifies to an empty map.

### Deprecated

- **`Claims.scope` and `Claims.roles` for authorization.** Kept on the
  wire for compatibility (removal is a future major); the ABAC guard
  ignores both and decides from `attrs` — a role, where one is wanted,
  is just another attribute (`role=editor`).

## [0.2.0] - 2026-06-17

> **BREAKING — the PASETO v4.public pivot (implemented).** Per the
> canonical design
> [authentication-sessions.md](../../agents/share/authentication-sessions.md)
> §5, JWT is removed from the federation's auth path: the human session
> becomes a Postgres-backed cookie session, and the only cross-service
> token is a short-lived **PASETO v4.public** (Ed25519). This crate keeps
> its **role** (peer-side, offline, dependency-light verification) but
> changes its **implementation** from RS256-JWT/JWKS to PASETO v4.public.
> `src/lib.rs` is rewritten (`rusty_paseto` v4.public + `ed25519-dalek`);
> the suite (14 unit + 3 doc tests) is green and clippy-clean under both
> the default and `fetch` feature sets.

### Changed

- **Verification target: RS256 JWT → PASETO v4.public (Ed25519).** The
  crate now verifies PASETO `v4.public` tokens against the
  authentication-service's published Ed25519 public key(s) at
  `/.well-known/paseto-keys` (replacing JWKS at `/.well-known/jwks.json`),
  selecting the key by the token **footer `kid`** and enforcing `iss` /
  `aud` / `exp` / `nbf` offline.
- **API rename:** `Verifier::from_paseto_keys_value` /
  `from_paseto_keys_url` (behind the `fetch` feature) replace
  `from_jwks_value` / `from_jwks_url`. The `Verifier::verify` /
  `key_count` signatures are unchanged.
- **`Claims` shape:** keeps `sub` / `email` / `name` / `iss` / `aud` /
  `iat` / `exp`; renames the JWT-era `jti` → `sid` (originating session,
  for revocation correlation); adds `nbf` and `scope` / `roles` (both
  defaulting to empty/absent). Must stay byte-identical to the service's
  `auth::Claims`, pinned by the service's cross-crate contract test.
- **`VerifyError` variants:** `Jwks` → `Keys`, `Jwt(..)` → `Paseto(..)`;
  `MissingKid` / `UnknownKid` / `Fetch` retained.
- **Dependencies:** a PASETO v4 library (e.g. `rusty_paseto`) replaces
  the `jsonwebtoken` / RSA stack. `#![forbid(unsafe_code)]`,
  dependency-light, crates.io-published all unchanged.

### Migration (0.1.x → 0.2.0)

- Point boot-time loading at `/.well-known/paseto-keys` (Ed25519) instead
  of `/.well-known/jwks.json` (RSA), and call `from_paseto_keys_value` /
  `from_paseto_keys_url`.
- Tokens must now be PASETO `v4.public`; bearer JWTs no longer verify.
- Update claim reads: use `sid` (not `jti`); `email` / `name` are gone.
- Match on `VerifyError::Keys` / `Paseto` instead of `Jwks` / `Jwt`.
- During the family rollover, peers may run JWT and PASETO side by side
  (shared-doc §9 step 3) before JWT is decommissioned.

### Superseded (RS256-JWT era, before the pivot)

- Targeted unit tests for the FR4 malformed-JWKS paths and the `fetch`
  feature's transport-error mapping, and the documentation-harmonization
  pass, all from the RS256-JWT implementation. Retained as history; the
  PASETO rewrite re-pins equivalent paths against Ed25519 (`spec/index.md`
  §11).

## [0.1.0] - 2026-06-13

### Added

- **Inaugural release.** Offline RS256 JWT verification for Main X
  Index peer services, mirroring the auth-service's token contract.
  - `Verifier::from_jwks_value(&jwks, issuer, audience)` — build a
    verifier from an in-memory JWKS document. RSA keys only; non-RSA
    entries are skipped; an empty key set is permitted (rejects every
    token with `UnknownKid`).
  - `Verifier::from_jwks_url(url, issuer, audience)` — fetch the JWKS
    over HTTPS at boot, behind the optional `fetch` feature
    (`reqwest` + rustls).
  - `Verifier::verify(token)` — `kid`-selected RS256 signature check
    plus `iss` / `aud` / `exp` enforcement, returning the verified
    `Claims`.
  - `Verifier::key_count()` — number of loaded RSA keys.
  - `Claims` — byte-identical mirror of the auth-service claims:
    `sub` (user pid), `email`, `name`, `iss`, `aud`, `exp`, `iat`,
    `jti` (= `sessions.jid`).
  - `VerifyError` — `Jwks`, `MissingKid`, `UnknownKid`, `Jwt`, and
    (with `fetch`) `Fetch`.
  - Offline unit tests with a throwaway RSA keypair: claim round-trip,
    expiry / audience / unknown-`kid` / tampered-signature / garbage
    rejection, empty-JWKS and malformed-JWKS handling, non-RSA-key
    skipping.
  - `#![forbid(unsafe_code)]`; dependency-light by design
    (`jsonwebtoken`, `serde`, `serde_json`, `thiserror`).
