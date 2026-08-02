# Fuzzing — `person-matcher` (SEC-I2 reference)

Coverage-guided [`cargo-fuzz`](https://rust-fuzz.github.io/book/cargo-fuzz.html)
harness for the matcher. It complements the crate's `proptest` properties
(random-value generation) with **libFuzzer**'s coverage-guided input
search over the same load-bearing invariants:

- the engine and the pure helpers **never panic** on arbitrary input, and
- every score / similarity is **finite and within `[0.0, 1.0]`**.

This is the **reference scaffolding** for the matcher family
(`agents/share/security.md`, repo `tasks.md` SEC-I2); the other matcher
crates adopt the same shape (see *Rolling out* below).

## Targets

| Target | What it fuzzes |
|---|---|
| `match_persons` | Deserialize a JSON `[person_a, person_b]` tuple → `MatchingEngine::match_persons`. The whole deserialize → normalize → score path; asserts finite score in `[0,1]` in both argument orders. |
| `normalizer` | The pure `Normalizer` string helpers (name / postcode / phone / E.164 / address / phonetic / email) over arbitrary UTF-8 — never-panic. |
| `scorer` | The pure `Scorer` similarities (Jaro-Winkler / Levenshtein / exact / combined) over two arbitrary UTF-8 strings; asserts finite similarity in `[0,1]`. |

## Running

Requires a **nightly** toolchain and `cargo-fuzz`
(`cargo install cargo-fuzz`):

```sh
# From the crate root (person-matcher-rust-crate/):
cargo +nightly fuzz build                 # compile all targets
cargo +nightly fuzz run match_persons     # fuzz until a crash / Ctrl-C
cargo +nightly fuzz run scorer -- -max_total_time=60   # time-boxed (CI)
cargo +nightly fuzz list                  # list targets
```

A crash writes a reproducer under `fuzz/artifacts/<target>/`; replay it
with `cargo +nightly fuzz run <target> fuzz/artifacts/<target>/<file>`.

`target/`, `corpus/`, `artifacts/`, and `coverage/` are git-ignored.

## CI

`.github/workflows/fuzz.yml` (GitHub) and the `fuzz-smoke` step in
`.woodpecker.yml` (Codeberg) run every target in every matcher's `fuzz/`
crate — plus authentication-verifier's and person's bulk-import parser —
for a short, fixed time bound (`FUZZ_SECONDS`, default 30s per target)
via `scripts/ci-check.sh fuzz [crate-path]`. This is a **smoke run**, not
exhaustive fuzzing: no corpus persists between runs, so it re-discovers
the same easy coverage every time rather than accumulating a deeper one.
It exists to catch an immediate regression (a panic a plain unit test
would not think to construct) before it reaches `main`, not to replace a
real, long-running fuzzing campaign — run one of those locally
(`cargo +nightly fuzz run <target>`, hours, a kept `corpus/`) if you are
specifically hardening a parser. GitHub runs the job on push/PR/weekly;
Woodpecker has no inline cron equivalent, so it runs on every triggering
event there.

## Isolation

The `fuzz/` crate is **not** a member of any parent workspace (the matcher
crates are standalone), so it never affects the crate's normal
`cargo build` / `cargo test` / `cargo clippy` — those stay on stable and
ignore this directory.

## Rolling out to the other matchers

Each sibling matcher (`worker`, `place`, `thing`, `event`, `course`,
`organization`, `care-pathway`, `case`, `portfolio`) adopts the same
scaffolding:

1. `cargo fuzz init` in the matcher crate root.
2. Add a `match_<entity>` target (deserialize the two records via
   `serde_json`, run `MatchingEngine::match_<entity>`, assert finite score
   in `[0,1]`), plus targets for that crate's pure normalizer / scorer
   helpers.
3. `cargo +nightly fuzz build` to verify, and a short `-max_total_time`
   run in CI.
