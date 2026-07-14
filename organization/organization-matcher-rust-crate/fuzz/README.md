# Fuzzing — `organization-matcher` (SEC-I2)

Coverage-guided [`cargo-fuzz`](https://rust-fuzz.github.io/book/cargo-fuzz.html)
harness for the matcher, adopting the
[`person-matcher` reference scaffolding](../../../person/person-matcher-rust-crate/fuzz/README.md)
(SEC-I2). It complements the crate's `proptest` properties (random-value
generation) with **libFuzzer**'s coverage-guided input search over the same
load-bearing invariants:

- the engine and the pure helpers **never panic** on arbitrary input, and
- every score is **finite and within `[0.0, 1.0]`**.

This crate exposes its pure similarity primitives only through the engine
(the `scoring` module publishes no string-similarity functions), so it
carries **two** targets rather than the reference's three.

## Targets

| Target | What it fuzzes |
|---|---|
| `match_organizations` | Deserialize a JSON `[organization_a, organization_b]` tuple → `MatchingEngine::match_organizations`. The whole deserialize → normalize → score path; asserts finite score in `[0,1]` in both argument orders. |
| `normalize` | The pure `normalize` free functions (fold / legal name / domain / fold-set) over arbitrary UTF-8 — never-panic. |

## Running

Requires a **nightly** toolchain and `cargo-fuzz`
(`cargo install cargo-fuzz`):

```sh
# From the crate root:
cargo +nightly fuzz build                 # compile all targets
cargo +nightly fuzz run match_organizations     # fuzz until a crash / Ctrl-C
cargo +nightly fuzz run normalize -- -max_total_time=60   # time-boxed (CI)
cargo +nightly fuzz list                  # list targets
```

A crash writes a reproducer under `fuzz/artifacts/<target>/`; replay it
with `cargo +nightly fuzz run <target> fuzz/artifacts/<target>/<file>`.

`target/`, `corpus/`, `artifacts/`, and `coverage/` are git-ignored.

## Isolation

The `fuzz/` crate is **not** a member of any parent workspace (the matcher
crates are standalone), so it never affects the crate's normal
`cargo build` / `cargo test` / `cargo clippy` — those stay on stable and
ignore this directory.
