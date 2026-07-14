# Fuzzing — `authentication-verifier` (SEC-I2)

Coverage-guided [`cargo-fuzz`](https://rust-fuzz.github.io/book/cargo-fuzz.html)
harness for the offline PASETO verifier + ABAC engine, adopting the
[matcher-family reference scaffolding](../../../person/person-matcher-rust-crate/fuzz/README.md)
(SEC-I2). Both entry points here process **attacker-controlled input** — a
peer's bearer token and a deployment's policy config — so they directly
exercise the crate's golden rule #5 ("no panics in the API; every failure
mode is a `VerifyError`").

## Targets

| Target | What it fuzzes |
|---|---|
| `verify` | `Verifier::verify` over an arbitrary token string. Exercises the whole `v4.public` structural parse — header check, authenticated footer base64url/JSON decode for the `kid`, key selection, and the Ed25519 signature check (the verifier is built with a real key so the `kid`-found branch is reachable). A random token cannot forge a signature, so a valid claim set is unreachable by luck; the invariant is that the parser never aborts on hostile bytes. |
| `policy` | `Policy::from_json` over arbitrary UTF-8, then — on a parse success — `evaluate_with_context` for every action against a fixed subject / resource / environment. Exercises the policy JSON parser plus the rule evaluator (attribute matching, negation, `$sub`/`$email` templates, the `resource.`/`env.` namespaces); asserts neither panics. |

## Running

Requires a **nightly** toolchain and `cargo-fuzz`
(`cargo install cargo-fuzz`):

```sh
# From the crate root (authentication-verifier-rust-crate/):
cargo +nightly fuzz build                 # compile both targets
cargo +nightly fuzz run verify            # fuzz the token parser
cargo +nightly fuzz run policy -- -max_total_time=60   # time-boxed (CI)
cargo +nightly fuzz list                  # list targets
```

A crash writes a reproducer under `fuzz/artifacts/<target>/`; replay it
with `cargo +nightly fuzz run <target> fuzz/artifacts/<target>/<file>`.

`target/`, `corpus/`, `artifacts/`, and `coverage/` are git-ignored.

## Isolation

The `fuzz/` crate is **not** a member of any parent workspace (this crate is
standalone), so it never affects the crate's normal `cargo build` /
`cargo test` / `cargo clippy` — those stay on stable and ignore this
directory. The targets use only default (offline) features — no `fetch`.
