# AGENTS.md — Authentication Verifier

Entry point for AI coding agents (and humans) working in the
`authentication-verifier` crate — the **peer-side verification
library** for the Main X Index single sign-on.

> If you read one file, read [`spec/index.md`](./spec/index.md): the
> living specification. This guide tells you **how to work**; the spec
> tells you **what to build**.

## What this crate is

A dependency-light Rust library that verifies the
[authentication-service](../authentication-service-rust-crate)'s RS256
access tokens **offline** against the JWKS it publishes at
`/.well-known/jwks.json`. Peer services embed it; there is no shared
secret and no per-request introspection call.

| Question | Answer |
|---|---|
| Kind | Plain library crate (no framework, no database, no I/O by default). |
| Public API | `Verifier::{from_jwks_value, from_jwks_url, verify, key_count}`, `Claims`, `VerifyError`. |
| Features | `fetch` — HTTPS JWKS loading via `reqwest` (rustls). Default: none. |
| Build | `cargo build` |
| Test | `cargo test` (fully offline; throwaway test keypair). |
| Lint | `cargo clippy` |
| Package | `cargo package --list` |

## Golden rules

1. **`Claims` mirrors the service byte-for-byte.** The struct is
   duplicated by convention with the service's `auth::Claims`; the
   service's `tests/sign_verify_contract.rs` pins the round-trip. If
   you change `Claims`, change the service in the same PR and keep the
   contract test green.
2. **The `kid` derivation is a contract.** `kid` = base64url (no
   padding) of `SHA-256(big-endian RSA modulus bytes)` — exactly what
   the service's `auth::load_keys` publishes and stamps into headers.
3. **Stay dependency-light.** Core deps are `jsonwebtoken`, `serde`,
   `serde_json`, `thiserror` only. Anything heavier (HTTP, async
   runtimes) goes behind a feature like `fetch`.
4. **RS256 only; RSA keys only.** Non-RSA JWKS entries are skipped, and
   that's deliberate — the service publishes RS256 keys exclusively.
5. **No panics in the API.** Every failure mode is a `VerifyError`
   variant. An empty JWKS is valid input (rejects with `UnknownKid`).
6. **`#![forbid(unsafe_code)]`** stays.

## Layout

```
src/lib.rs        the whole crate: Verifier, Claims, VerifyError,
                  fetch-feature impl, offline unit tests
spec/index.md     living spec (§1–§18)
```

## When you are unsure

The spec wins. If the spec is silent, check the entity-level contract
([../spec/index.md](../spec/index.md)) and the peer-integration guide
([../AGENTS/verification.md](../AGENTS/verification.md)); otherwise
propose a spec update rather than guessing.
