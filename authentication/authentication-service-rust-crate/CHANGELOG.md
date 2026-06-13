# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md) — single source of truth;
> [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

### Added

- **Cross-crate sign→verify contract test**
  (`tests/sign_verify_contract.rs`): signs with this crate's `auth`
  module and verifies through the sibling
  [`authentication-verifier`](../authentication-verifier-rust-crate)
  crate (new dev-dependency), pinning the duplicated-by-convention
  `Claims` shape and the `kid = base64url(SHA-256(modulus))`
  derivation. DB-free; runs in every `cargo test`.
- **Magic-link request tests**: `tests/requests/auth.rs` now covers
  signup / magic-link / redeem (single-use, anti-enumeration) / me /
  signout / JWKS with direct assertions. Postgres-backed tests are
  `#[ignore]`d (run with `cargo test -- --ignored`) so plain
  `cargo test` stays green; DB-free route-table and params-contract
  tests always run.

### Removed

- The starter's password-flow request tests and their insta snapshots
  (`register`/`login`/`forgot`/`reset`/`verify` endpoints no longer
  exist).

### Added (inaugural)

- **Inaugural scaffold (v0.1.0).** The Main X Index family's central
  single sign-on provider and reference loco.rs application.
  - Real loco.rs 0.16 app generated via `loco new` (Postgres,
    Postgres-backed queue, no asset tier).
  - **Passwordless magic-link** flow: `POST /api/auth/signup`,
    `POST /api/auth/magic-link`, `GET /api/auth/magic-link/{token}`,
    `GET /api/auth/me`, `POST /api/auth/signout`.
  - **RS256 JWT** issuance with a self-contained `src/auth` module
    (`jsonwebtoken` + `rsa`), and a **JWKS** endpoint at
    `/.well-known/jwks.json` so peer services verify tokens offline —
    no shared secret, no introspection hop.
  - **sessions** table (`jid` = JWT `jti`) for real signout/revocation,
    honoured locally by `/me` and `/signout`.
  - Console magic-link delivery in development (SMTP disabled); env-based
    RSA key configuration with a committed dev keypair under
    `config/keys/`.
  - DB-free unit tests covering the sign/verify roundtrip, JWKS shape,
    and rejection of tampered/garbage tokens. Green `cargo build`,
    clippy clean.

### Notes

- The Postgres-backed model tests (`tests/models/users.rs`) are
  `#[ignore]`d so `cargo test` stays green without a database; several
  still exercise password-era model helpers that survive only to
  satisfy the schema.
