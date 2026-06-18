# AGENTS.md — Authentication Verifier

Entry point for AI coding agents (and humans) working in the
`authentication-verifier` crate — the **peer-side verification
library** for the Main X Index single sign-on.

> If you read one file, read [`spec/index.md`](./spec/index.md): the
> living specification. This guide tells you **how to work**; the spec
> tells you **what to build**.

## What this crate is

A dependency-light Rust library that verifies the
[authentication-service](../authentication-service-with-loco)'s
short-lived **PASETO v4.public** (Ed25519) cross-service tokens
**offline** against the key set it publishes at
`/.well-known/paseto-keys`. Peer services embed it; there is no shared
secret and no per-request introspection call.

> **v0.2.0 pivot — RS256 JWT → PASETO v4.public.** Per
> [authentication-sessions.md](../../agents/share/authentication-sessions.md)
> §5, JWT is removed from the auth path. The crate keeps its role
> (peer-side, offline, dependency-light verification) but changes its
> implementation. The shipped `src/lib.rs` is still the RS256-JWT code;
> the PASETO rewrite is the open spec task (`spec/index.md` §13 T-1).

| Question | Answer |
|---|---|
| Kind | Plain library crate (no framework, no database, no I/O by default). |
| Public API | `Verifier::{from_paseto_keys_value, from_paseto_keys_url, verify, key_count}`, `Claims`, `VerifyError`. |
| Features | `fetch` — HTTPS key-set loading via `reqwest` (rustls). Default: none. |
| Build | `cargo build` |
| Test | `cargo test` (fully offline; throwaway Ed25519 test keypair). |
| Lint | `cargo clippy` |
| Package | `cargo package --list` |

## Golden rules

1. **`Claims` mirrors the service byte-for-byte.** The struct is
   duplicated by convention with the service's `auth::Claims`; the
   service's `tests/sign_verify_contract.rs` pins the round-trip. If
   you change `Claims`, change the service in the same PR and keep the
   contract test green.
2. **The footer `kid` is a contract.** The token footer carries the
   `kid` that selects the verifier key — exactly what the service
   stamps into the PASETO footer and publishes in `/.well-known/paseto-keys`.
3. **Stay dependency-light.** Core deps are a PASETO v4 library (e.g.
   `rusty_paseto`), `serde`, `serde_json`, `thiserror` only. Anything
   heavier (HTTP, async runtimes) goes behind a feature like `fetch`.
4. **PASETO v4.public only; Ed25519 keys only.** Non-Ed25519 key
   entries are skipped, and that's deliberate — the service publishes
   Ed25519 public keys exclusively. No PASETO `local` (symmetric).
5. **No panics in the API.** Every failure mode is a `VerifyError`
   variant. An empty key set is valid input (rejects with `UnknownKid`).
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
