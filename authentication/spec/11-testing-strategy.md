## 11. Testing Strategy

Per-subproject detail: [`agents/testing.md`](../agents/testing.md).

### 11.1 Service

- **Unit (DB-free):** `src/auth` `#[cfg(test)]` — published key-set
  shape (Ed25519 signing key, `kid` matches the token-footer `kid`),
  PASETO sign → verify
  claim round-trip, tampered-signature rejection, garbage-token
  rejection. Run with `cargo test --lib` against the built-in dev
  seed.
- **Request tests:** loco's `tests/requests/auth.rs` covers the
  magic-link / redeem (single-use, anti-enumeration) / me / signout /
  paseto-keys surface (§13 T-3 done). The PostgreSQL-backed tests are
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

Offline unit tests in `src/lib.rs` using a throwaway Ed25519 keypair
(sign locally, verify against a key-set document built exactly the way
the service derives `kid` and encodes the public key):

| Category | Pins |
|---|---|
| Round-trip | Valid PASETO v4.public returns the full claim set; `key_count` |
| Claim policy | Expired (`exp`) token rejected; not-yet-valid (`nbf`) rejected; wrong audience / issuer rejected |
| Key selection | Missing-`kid` / unknown-`kid` rejected; empty key set builds but rejects everything; non-Ed25519 keys skipped |
| Integrity | Tampered signature rejected; garbage / empty token rejected |
| Document shape | Key set without a key array errors; entries missing `kid` / key material error |

### 11.3 Front-end

Planned, not yet present (front-end spec §13): vitest unit tests for
`ApiClient` + `AuthRepository`; playwright smoke tests for the four
routes. `pnpm run check` (svelte-check, strict) and `pnpm run build`
are the current gates.

### 11.4 Cross-subproject contract tests

The **service-signs / verifier-verifies** contract is pinned by
`authentication-service-with-loco/tests/sign_verify_contract.rs`
(§13 T-4): the service's `auth` module signs a real PASETO; a
`Verifier` built from the service's published key-set document verifies
it through `authentication-verifier` (a dev-dependency of the
service); the claims round-trip byte-for-byte; the
`kid = base64url(SHA-256(public key bytes))` thumbprint is recomputed
independently; and a `kid` mismatch fails with `UnknownKid`. The test
is DB-free and runs un-gated in every `cargo test`.
