# Fuzzing — `entity-ref`

Coverage-guided [`cargo-fuzz`](https://rust-fuzz.github.io/book/cargo-fuzz.html)
harness for the cross-service-linking contract
([`agents/share/cross-service-linking.md`](../../../agents/share/cross-service-linking.md)),
following the SEC-I2 scaffolding the matcher crates use.

This crate is small and pure, which is exactly why it is worth fuzzing:
it is a **parser that eight other crates depend on**. An `EntityRef` is
one opaque `TEXT` column and one request-body field naming a record in
another service, so `FromStr` and the serde bridge run on strings this
family does not control — and a `Display`/parse asymmetry here would
repoint an edge at a different record rather than fail loudly.

## Targets

| Target | What it fuzzes |
|---|---|
| `parse_ref` | `EntityRef::from_str` over arbitrary UTF-8: never-panic, plus the **round-trip** invariant (anything that parses must `Display` back to something that re-parses to the identical value) and a non-empty owning service. |
| `deserialize_ref` | The `#[serde(try_from = "String")]` path via `serde_json` — the entry point real callers use — plus serialize→deserialize equality. |
| `registry_tokens` | The closed `EntityType` / `EdgeKind` token registries: `from_token` accepts only registry members and round-trips; `is_symmetric` agrees with `inverse`; `permits` is order-symmetric for a symmetric kind. |

## Running

Requires a **nightly** toolchain and `cargo-fuzz`
(`cargo install cargo-fuzz`):

```sh
# From the crate root (entity-ref-rust-crate/):
cargo +nightly fuzz build                  # compile all targets
cargo +nightly fuzz run parse_ref          # fuzz until a crash / Ctrl-C
cargo +nightly fuzz run parse_ref -- -max_total_time=60   # time-boxed (CI)
cargo +nightly fuzz list                   # list targets
```

A crash writes a reproducer under `fuzz/artifacts/<target>/`; replay it
with `cargo +nightly fuzz run <target> fuzz/artifacts/<target>/<file>`.

`target/`, `corpus/`, `artifacts/`, and `coverage/` are git-ignored, so
nothing is carried between runs — CI's
[`fuzz` stage](../../../scripts/ci-check.sh) is a short smoke run
(`FUZZ_SECONDS`, default 30, per target), not exhaustive fuzzing.

This sub-crate is **not** a member of any workspace and declares no
`rust-version`: cargo-fuzz is nightly-only, so the repo MSRV does not
apply to it (see
[`spec/rust-msrv-n-minus-2/index.md`](../../../spec/rust-msrv-n-minus-2/index.md) §2).
