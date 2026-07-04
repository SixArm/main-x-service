# Testing Strategy & Guide — Authentication Entity

Entity-level summary. Normative strategy: entity spec
[§11 Testing Strategy](../spec/11-testing-strategy.md).

> **Auth model source of truth:**
> [`../../agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md).
> Sessions are server-side httpOnly **cookie sessions**; cross-service
> auth is **PASETO v4.public** verified offline via the published Ed25519
> key at `/.well-known/paseto-keys`. RS256 JWT + JWKS are
> **decommissioned** and removed from the code; the crypto tests below
> exercise the PASETO/Ed25519 surface.

## Service (`authentication-service-with-loco`)

```bash
cargo test --lib        # DB-free unit tests (the src/auth module)
cargo clippy --bins
cargo test              # full loco request tests — needs PostgreSQL
```

| Layer | Where | Covers |
|---|---|---|
| Unit (DB-free) | `src/auth/mod.rs` `#[cfg(test)]` | Published key-set shape (Ed25519 entries; published `kid` = token-footer `kid`), PASETO sign → verify claim round-trip, tampered-signature rejection, garbage-token rejection. Runs against the built-in dev seed. |
| Request | `tests/requests/auth.rs` | Magic-link surface: signup / magic-link / redeem (single-use, anti-enumeration) / me / signout / paseto-keys. Postgres-backed tests are `#[ignore]`d (run: `cargo test -- --ignored`); DB-free route-table + params-contract tests always run. |

## Verifier (`authentication-verifier-rust-crate`)

The verifier crate is **already harmonized** to PASETO v4.public; see its
own docs for the authoritative test list.

```bash
cargo test                      # offline unit tests
cargo test --features fetch     # compile the HTTP published-key path too
```

All tests are offline: a throwaway Ed25519 keypair signs locally; the
published-key document is rebuilt exactly the way the service derives
`kid`.

| Category | Tests |
|---|---|
| Round-trip | valid token returns full claims; `key_count` |
| Claim policy | expired rejected; wrong audience rejected |
| Key selection | unknown `kid` rejected; empty key set builds but rejects all |
| Integrity | tampered signature rejected; garbage/empty token rejected |
| Document shape | malformed published-key document errors |

## Front-end (`authentication-front-end-with-svelte`)

```bash
pnpm run check      # svelte-check, strict — current gate
pnpm run build
pnpm run test       # vitest — planned, not yet written
pnpm run test:e2e   # playwright — planned, not yet written
```

Unit tests for `ApiClient` / `AuthRepository` and a four-route
playwright smoke are queued (entity spec §13 T-11; front-end spec §13).

## Cross-subproject contract test

The **service-signs / verifier-verifies** contract is pinned by the
service crate's `tests/sign_verify_contract.rs` (entity spec §13 T-4):
the two crates duplicate the `Claims` struct and `kid` derivation by
convention, and the test fails if they drift. It is DB-free and runs
in every `cargo test`. Any change to claims or `kid` derivation MUST
still be made in both crates in the same PR (see
[`spec-driven-development.md`](spec-driven-development.md)) — the
test is the tripwire, not the fix.

## Writing new tests here

- Keep crypto tests **offline and deterministic**: committed dev
  keypair (service) or in-test throwaway keypair (verifier). Never
  reach for the network in unit tests.
- Negative paths matter most at a trust boundary: expired, tampered,
  wrong-`aud`, wrong-`iss`, unknown-`kid`, revoked-session — every new
  feature should add its rejection case.
- Anti-enumeration is behaviour: tests for signup / magic-link MUST
  assert `200` for unknown emails too.
