# Fuzzing — `place-matcher` (SEC-I2)

Coverage-guided [`cargo-fuzz`](https://rust-fuzz.github.io/book/cargo-fuzz.html)
harness for the matcher, adopting the
[`person-matcher` reference scaffolding](../../../person/person-matcher-rust-crate/fuzz/README.md)
(SEC-I2). It complements the crate's `proptest` properties (random-value
generation) with **libFuzzer**'s coverage-guided input search over the same
load-bearing invariants:

- the engine and the pure helpers **never panic** on arbitrary input, and
- every score / similarity is **finite and within `[0.0, 1.0]`**.

## Targets

| Target | What it fuzzes |
|---|---|
| `match_places` | Deserialize a JSON `[place_a, place_b]` tuple → `MatchingEngine::match_places`. The whole deserialize → normalize → score path; asserts finite score in `[0,1]` in both argument orders. |
| `normalizer` | The pure `Normalizer` string helpers (name / postcode / phone / E.164 / address / phonetic / email) over arbitrary UTF-8 — never-panic. |
| `scorer` | The pure `Scorer` similarities (Jaro-Winkler / Levenshtein / exact / combined) over two arbitrary UTF-8 strings; asserts finite similarity in `[0,1]`. |

## Running

Requires a **nightly** toolchain and `cargo-fuzz`
(`cargo install cargo-fuzz`):

```sh
# From the crate root:
cargo +nightly fuzz build                 # compile all targets
cargo +nightly fuzz run match_places     # fuzz until a crash / Ctrl-C
cargo +nightly fuzz run scorer -- -max_total_time=60   # time-boxed (CI)
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
