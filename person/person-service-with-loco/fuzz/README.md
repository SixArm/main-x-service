# Fuzzing — person bulk-import parsers (SEC-I2 / SEC-B2)

Coverage-guided [`cargo-fuzz`](https://rust-fuzz.github.io/book/cargo-fuzz.html)
harness for the bulk-import JSONL parsers, adopting the
[matcher-family reference scaffolding](../../../person/person-matcher-rust-crate/fuzz/README.md)
(SEC-I2). `bulk::jsonl` turns **attacker-supplied uploaded file bytes** into
`Person` records before any validation, so its line splitters and per-line
JSON parser are prime never-panic targets (SEC-B2).

## Targets

| Target | What it fuzzes |
|---|---|
| `parse_line` | `bulk::jsonl::LineReader` (the **streaming** framing the import worker actually uses, SEC-B2) and `bulk::jsonl::split_lines` over raw upload bytes, plus `bulk::jsonl::parse_line` over each framed row and the whole blob as one pathological line. Asserts the whole frame-then-deserialize path never panics (no overflow, no unbounded work) on hostile input — it returns a handled `Err` instead. Complements the crate's `parse_line_never_panics` / `line_reader_never_panics_on_random_bytes` proptests with coverage-guided input search. |

## Running

Requires a **nightly** toolchain and `cargo-fuzz`
(`cargo install cargo-fuzz`):

```sh
# From the crate root (person-service-with-loco/):
cargo +nightly fuzz build                 # compile the target
cargo +nightly fuzz run parse_line        # fuzz until a crash / Ctrl-C
cargo +nightly fuzz run parse_line -- -max_total_time=60   # time-boxed (CI)
cargo +nightly fuzz list                  # list targets
```

A crash writes a reproducer under `fuzz/artifacts/parse_line/`; replay it
with `cargo +nightly fuzz run parse_line fuzz/artifacts/parse_line/<file>`.

`target/`, `corpus/`, `artifacts/`, and `coverage/` are git-ignored.

## Isolation

The `fuzz/` crate is **not** a member of any parent workspace (the service
crate declares none), so it never affects the crate's normal
`cargo build` / `cargo test` / `cargo clippy` — those stay on stable and
ignore this directory. It depends on the service's library target only.
