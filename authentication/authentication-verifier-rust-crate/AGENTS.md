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
> implementation. The shipped `src/lib.rs` **is** the PASETO code —
> v0.2.0 is published to crates.io; the RS256-JWT implementation is gone.

> **v0.3.0 — ABAC (additive).** Per
> [authorization-attributes.md](../../agents/share/authorization-attributes.md),
> the crate is also the family's shared **authorization** foundation:
> `Claims` gains the `attrs` subject-attribute map (`#[serde(default)]`
> — pre-0.3 tokens verify to an empty map), and `src/abac.rs` ships the
> pure policy engine (first-match-wins allow/deny rules over attrs +
> derived action + entity; default allow-read / deny-mutation) that the
> nine entity services call from their blanket `/api/*` guards.
> `scope` / `roles` are deprecated for authorization.

| Question | Answer |
|---|---|
| Kind | Plain library crate (no framework, no database, no I/O by default). |
| Public API | `Verifier::{from_paseto_keys_value, from_paseto_keys_url, verify, key_count}`, `Claims` (incl. the 0.3 `attrs` ABAC claim), `VerifyError`, and the `abac` module (`Policy`, `Rule`, `Action`, `ActionPattern`, `Effect`, `Decision` — re-exported at the root). |
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
src/lib.rs        verification: Verifier, Claims, VerifyError,
                  fetch-feature impl, offline unit tests
src/abac.rs       authorization: the shared ABAC policy engine
                  (Policy, Rule, Action, Decision) + engine unit tests
spec/index.md     living spec (§1–§18)
```

## When you are unsure

The spec wins. If the spec is silent, check the entity-level contract
([../spec/index.md](../spec/index.md)) and the peer-integration guide
([../AGENTS/verification.md](../AGENTS/verification.md)); otherwise
propose a spec update rather than guessing.
