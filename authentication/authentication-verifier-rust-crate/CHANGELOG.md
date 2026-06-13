# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md) — single source of truth;
> [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

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
