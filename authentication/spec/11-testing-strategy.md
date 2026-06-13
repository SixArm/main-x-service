## 11. Testing Strategy

Per-subproject detail: [`AGENTS/testing.md`](../AGENTS/testing.md).

### 11.1 Service

- **Unit (DB-free):** `src/auth` `#[cfg(test)]` — JWKS shape (one RSA
  signing key, `kid` matches the token-header `kid`), sign → verify
  claim round-trip, tampered-signature rejection, garbage-token
  rejection. Run with `cargo test --lib` against the committed dev
  keypair.
- **Request tests:** loco's `tests/requests/auth.rs` covers the
  magic-link / redeem (single-use, anti-enumeration) / me / signout /
  JWKS surface (§13 T-3 done). The PostgreSQL-backed tests are
  `#[ignore]`d so plain `cargo test` stays green without a database;
  run them with `cargo test -- --ignored`. DB-free route-table and
  params-contract assertions always run (including the optional
  `locale` field on `SignupParams` / `MagicLinkParams`).
- **i18n (DB-free):** `src/i18n.rs` `#[cfg(test)]` (8 tests) pins the
  magic-link email catalog — `en` / `cy` copy, `{link}` substitution,
  `en` fallback for unknown locales, region-subtag reduction, and the
  `select_locale` input→locale mapping. The DB-gated
  `signup_locale_does_not_change_the_response_shape` request test
  asserts the always-`200` shape across `en` / `cy` / unknown / absent
  locales.

### 11.2 Verifier

Nine offline unit tests in `src/lib.rs` using a throwaway RSA keypair
(sign locally, verify against a JWKS built exactly the way the service
derives `kid` / `n` / `e`):

| Category | Pins |
|---|---|
| Round-trip | Valid token returns the full claim set; `key_count` |
| Claim policy | Expired token rejected; wrong audience rejected |
| Key selection | Missing-`kid` / unknown-`kid` rejected; empty JWKS builds but rejects everything; non-RSA keys skipped |
| Integrity | Tampered signature rejected; garbage / empty token rejected |
| Document shape | JWKS without a `keys` array errors |

### 11.3 Front-end

Planned, not yet present (front-end spec §13): vitest unit tests for
`ApiClient` + `AuthRepository`; playwright smoke tests for the four
routes. `pnpm run check` (svelte-check, strict) and `pnpm run build`
are the current gates.

### 11.4 Cross-subproject contract tests

The **service-signs / verifier-verifies** contract is pinned by
`authentication-service-rust-crate/tests/sign_verify_contract.rs`
(§13 T-4): the service's `auth` module signs a real token; a
`Verifier` built from the service's published JWKS document verifies
it through `authentication-verifier` (a dev-dependency of the
service); the claims round-trip byte-for-byte; the
`kid = base64url(SHA-256(modulus))` thumbprint is recomputed
independently; and a `kid` mismatch fails with `UnknownKid`. The test
is DB-free and runs un-gated in every `cargo test`.
