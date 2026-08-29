# Fuzzing — `organization-service`

Coverage-guided [`cargo-fuzz`](https://rust-fuzz.github.io/book/cargo-fuzz.html)
harness for this service's **pure request-path logic** (repo `tasks.md`
FUZZ-2, extending SEC-I2 past the library crates).

Until this landed, the fuzz harnesses covered the matcher crates, the
verifier, `entity-ref`, `integrity-mac`, and person's bulk line parser —
all dependency-light libraries. The services had none, despite carrying
the surface that actually faces the network: `validation::problems` runs
on an attacker-supplied request body, and `merge::merge_orgs` folds two
stored payloads, where "stored" is not the same as "trusted" (rows
predate today's validators, and bulk import writes them too). Both are
pure and total, so they fuzz with no database.

## Targets

| Target | What it fuzzes |
|---|---|
| `validate_json` | The real request path: arbitrary bytes → `serde_json` → `Organization` → `validation::problems`. Never-panic, deterministic, and a **bounded problem report**. |
| `validate_built` | The validator driven directly, building the `Organization` from raw bytes so the fuzzer controls **array cardinality and entry contents** without first learning JSON. A run of NUL bytes becomes a run of blank entries — the exact SEC-M8 shape — for the cost of the bytes. |
| `merge_orgs` | The merge fold over two arbitrary payloads: never-panic; the survivor keeps its `name`; deterministic; and **absorbing** — re-merging the same duplicate adds nothing, which is what stops a retried merge inflating the record. |

The bounded-report invariant is the one worth understanding. SEC-M8 was a
defect where an over-long array was reported once for cardinality and
then still enumerated, so ten thousand blank entries produced ten
thousand problem strings in a single `422` body — a small request buying
a large response. The unit tests pin that for one hand-written payload;
these targets pin it for any payload the fuzzer can reach.

## Running

Requires a **nightly** toolchain and `cargo-fuzz`
(`cargo install cargo-fuzz`):

```sh
# From the crate root:
cargo +nightly fuzz build                    # compile all targets
cargo +nightly fuzz run validate_built       # fuzz until a crash / Ctrl-C
cargo +nightly fuzz run validate_built -- -max_total_time=60   # time-boxed (CI)
cargo +nightly fuzz list                     # list targets
```

A crash writes a reproducer under `fuzz/artifacts/<target>/`; replay it
with `cargo +nightly fuzz run <target> fuzz/artifacts/<target>/<file>`.

`target/`, `corpus/`, `artifacts/`, and `coverage/` are git-ignored, so
nothing carries between runs — CI's
[`fuzz` stage](../../../scripts/ci-check.sh) is a short smoke run
(`FUZZ_SECONDS`, default 30, per target), not exhaustive fuzzing.

## Why this sub-crate declares `[workspace]`

The parent service crate declares `[workspace]`, which would otherwise
pull `fuzz/` in as a member; a cargo-fuzz build needs its own sanitizer
flags and lockfile, not the service's. The empty `[workspace]` table
makes this its own root. (The matcher crates' fuzz sub-crates need no
such table — their parents are not workspace roots.)

It declares no `rust-version`: cargo-fuzz is nightly-only, so the repo
MSRV does not apply (see
[`spec/rust-msrv-n-minus-2/index.md`](../../../spec/rust-msrv-n-minus-2/index.md) §2).
