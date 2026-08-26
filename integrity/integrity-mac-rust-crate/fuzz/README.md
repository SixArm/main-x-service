# Fuzzing — `integrity-mac`

Coverage-guided [`cargo-fuzz`](https://rust-fuzz.github.io/book/cargo-fuzz.html)
harness for the keyed-integrity MAC crate, following the SEC-I2
scaffolding the matcher crates use.

This crate is the one place in the family whose correctness rests
entirely on getting cryptography right, and its verification path takes
input from precisely the adversary it exists to defend against: someone
holding the database can rewrite a stored MAC to any string they like,
and `KeySet::verify` then parses it. That is a fuzz target by
construction.

**It found one on the first run.** `decode_hex` sliced the `&str` by
byte index (`&s[i..i + 2]`), so a stored value containing a multi-byte
character — `k1:€a` — panicked instead of returning `Malformed`, giving
a database-write adversary a process abort on every reader. Fixed by
decoding over bytes; pinned by `verify_tag` and by a unit test.

## Targets

| Target | What it fuzzes |
|---|---|
| `verify_tag` | `KeySet::verify` over an arbitrary stored value and pre-image: never-panic; our own tag always verifies; a tag never transfers between HKDF domains; a derived-scheme value that verifies really is the tag we would have written. |
| `assess_key` | `assess_key_hex` over arbitrary UTF-8: never-panic; a `Usable` verdict decodes to at least `MIN_KEY_LEN`; and the checker and the loader never disagree about a key. |
| `load_keys` | `KeySet::load` over an arbitrary inline key and retired-key list: never-panic; an enabled set is self-consistent (tag → verify → `Valid`, tag carries the advertised prefix, the key id round-trips through the lookup). |

## Running

Requires a **nightly** toolchain and `cargo-fuzz`
(`cargo install cargo-fuzz`):

```sh
# From the crate root (integrity-mac-rust-crate/):
cargo +nightly fuzz build                  # compile all targets
cargo +nightly fuzz run verify_tag         # fuzz until a crash / Ctrl-C
cargo +nightly fuzz run verify_tag -- -max_total_time=60   # time-boxed (CI)
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
[`spec/rust-msrv-n-minus-3/index.md`](../../../spec/rust-msrv-n-minus-3/index.md) §2).
