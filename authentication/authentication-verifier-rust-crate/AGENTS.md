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

> **v0.4.0–v0.8.0 — ABAC grows record/environment awareness + hot-reload
> (all additive).** Record-level `resource.*` attributes (0.4),
> `$sub`/`$email` ownership templates + `env.*` attributes (0.5),
> obligations like `"mask"`/`"audit"` on `Decision` (0.6), and
> hot-reloadable holders for both the policy (`ReloadablePolicy`, 0.7)
> and the verifier (`ReloadableVerifier`, 0.8) — key rotation with no
> restart. See `CHANGELOG.md` for each.
>
> **v0.9.0 (2026-08-05) — security hardening + fuzzing + verifier
> algorithm agility, shipped.** SEC-V1 (`from_paseto_keys_url` now
> requires HTTPS, or HTTP to loopback only, with a
> timeout/no-redirect/64 KiB body cap), SEC-V2 (a negated
> `resource.`/`env.` condition no longer matches vacuously when the
> namespace is absent), a `fuzz/` cargo-fuzz harness (SEC-I2), and
> (2026-07-27) the `Verifier` becoming algorithm-agile all landed
> together in this release — see the golden rule below and
> `spec/index.md` §5/§13/§14. `Cargo.toml` reads `0.9.0`; further
> changes have since accumulated in `CHANGELOG.md`'s `[Unreleased]`
> with no release cut yet (tracked as AV-2).

| Question | Answer |
|---|---|
| Kind | Plain library crate (no framework, no database, no I/O by default). |
| Public API | `Verifier::{from_paseto_keys_value, from_paseto_keys_url, verify, key_count, unsupported_key_count, algorithms}`, `ReloadableVerifier` (0.8 hot-reload holder), `Claims` (incl. the 0.3 `attrs` ABAC claim), `VerifyError` (incl. the 2026-07-27 `UnsupportedAlgorithm` variant), and the `abac` module (`Policy`, `Rule`, `Action`, `ActionPattern`, `Effect`, `Decision`, `ReloadablePolicy` — re-exported at the root). |
| Features | `fetch` — HTTPS key-set loading via `reqwest` (rustls). Default: none. |
| Build | `cargo build` |
| Test | `cargo test` (fully offline; throwaway Ed25519 test keypair). **`cargo test --features fetch`** additionally compiles and runs `from_paseto_keys_url` and its SEC-V1 HTTPS-only / timeout / no-redirect / body-cap tests — the repo's own `scripts/ci-check.sh test` runs this crate with `--features fetch` (AV-1: without it, those items were never compiled, let alone run, by CI). |
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
4. **PASETO v4.public only; verification implements Ed25519 only.**
   The service publishes Ed25519 public keys today, and this build only
   knows how to check that signature — but (2026-07-27, algorithm
   agility) a key set entry naming a *different* algorithm is **kept,
   not silently skipped**: it is diagnosed as `UnsupportedAlgorithm` if
   a token ever selects it, so a partial rollout to a future algorithm
   fails loud rather than as a misleading `UnknownKid`. See
   `spec/index.md` §5 "Algorithm agility". No PASETO `local`
   (symmetric).
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
fuzz/             standalone cargo-fuzz crate (SEC-I2); not a
                  workspace member, never affects the stable build
```

## When you are unsure

The spec wins. If the spec is silent, check the entity-level contract
([../spec/index.md](../spec/index.md)) and the peer-integration guide
([../agents/verification.md](../agents/verification.md)); otherwise
propose a spec update rather than guessing.
