## 17. References

### Subprojects

- Service: [README](../authentication-service-rust-crate/README.md) ·
  [spec](../authentication-service-rust-crate/spec/index.md) ·
  [AGENTS.md](../authentication-service-rust-crate/AGENTS.md) ·
  [CHANGELOG](../authentication-service-rust-crate/CHANGELOG.md)
- Verifier: [README](../authentication-verifier-rust-crate/README.md) ·
  [spec](../authentication-verifier-rust-crate/spec/index.md) ·
  [AGENTS.md](../authentication-verifier-rust-crate/AGENTS.md) ·
  [CHANGELOG](../authentication-verifier-rust-crate/CHANGELOG.md) ·
  [Cargo.toml](../authentication-verifier-rust-crate/Cargo.toml) ·
  [src/lib.rs](../authentication-verifier-rust-crate/src/lib.rs)
- Front-end: [README](../authentication-front-end-with-svelte/README.md) ·
  [spec](../authentication-front-end-with-svelte/spec/index.md) ·
  [AGENTS.md](../authentication-front-end-with-svelte/AGENTS.md) ·
  [CHANGELOG](../authentication-front-end-with-svelte/CHANGELOG.md)

### Entity-level reference set

- [`AGENTS/index.md`](../AGENTS/index.md) — directory of the
  entity-level reference docs.
- [`AGENTS/verification.md`](../AGENTS/verification.md) — how peers
  verify tokens (this entity's counterpart to siblings'
  `matching.md`).

### Family

- Project root: [`AGENTS.md`](../../AGENTS.md) ·
  [`agents/share/index.md`](../../agents/share/index.md).
- Sibling entity specs (same §1–§18 shape):
  [person-service](../../person/person-service-rust-crate/spec/index.md),
  the identity registry whose records this entity deliberately does
  **not** duplicate.
- Loco conventions: [`agents/share/loco.md`](../../agents/share/loco.md) ·
  [`agents/share/rust-loco-stack.md`](../../agents/share/rust-loco-stack.md).

### Standards

- RFC 7519 (JWT), RFC 7517 (JWK / JWKS), RFC 7518 (JWA — RS256).
- OpenID Connect Core / Discovery (roadmap — §15).
- W3C WebAuthn (roadmap — §15).
- loco.rs — https://loco.rs/
