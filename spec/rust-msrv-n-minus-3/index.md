# Rust MSRV — current N-3

The **Minimum Supported Rust Version** for every Rust crate in this
repository is **the current stable release minus three**: if the newest
stable compiler is `1.N`, the MSRV is `1.(N-3)`.

| | |
|---|---|
| Current stable (`N`) | **1.98.0** (2026-08-18) |
| **Declared MSRV (`N-3`)** | **1.95** |
| Toolchain we *build* with | **1.96.1** (`rust-toolchain.toml`) |
| Single source of truth | [`ci/msrv.txt`](../ci/msrv.txt) |

Those are three different numbers on purpose; §3 explains why.

## 1. The policy

- **MSRV = N-3.** Three releases is about **eighteen weeks** of grace.
  That is long enough for a downstream consumer, a vendored build, or a
  slow-moving distribution image to pick up a compiler at its own pace,
  and short enough that we are never more than a quarter of a year
  behind the language.
- **It is a *moving* floor, not a frozen one.** Every six weeks a new
  stable lands and the computed MSRV moves up by one. The declared value
  is only re-derived when someone bumps it deliberately (§5) — the
  repository always states a concrete version, never a formula that a
  build has to evaluate.
- **The floor is a promise about compilation**, nothing more: every crate
  must `cargo check --all-targets` and `cargo test` on the MSRV
  toolchain. It says nothing about which toolchain a developer or CI
  uses day to day.
- **N-3 is a ceiling on our conservatism, not a target.** Nothing obliges
  a crate to sit exactly at the floor. What is forbidden is *silently*
  drifting above it: a language or `std` feature stabilised after 1.95
  may not be used until the floor moves.

### Why N-3 and not something else

- **N-2 or newer** leaves a consumer roughly three months to upgrade
  before we break them — shorter than many organisations' toolchain
  review cycle, so a routine dependency bump on our side becomes an
  unplanned upgrade on theirs.
- **N-6 or older** is the opposite failure. This family already runs
  **edition 2024**, which requires ≥ 1.85, so an over-long tail buys
  compatibility we cannot actually offer; and the real constraint is not
  us but our dependency graph (§4), which today already refuses anything
  below 1.94.
- **N-3 sits just above that dependency floor** with one release of
  headroom, which is the honest place for it: a floor we cannot hold is
  worse than a higher one we can.

## 2. How it is declared

Every crate's `Cargo.toml` carries the version in its `[package]` table:

```toml
[package]
name = "person-matcher"
edition = "2024"
rust-version = "1.95"
```

- It is declared **per crate** rather than inherited, because this
  repository has **no root `Cargo.toml`** — each crate is its own
  workspace (see [`scripts/ci-crates.sh`](../scripts/ci-crates.sh)), so
  there is nothing to inherit from.
- The value is duplicated across ~50 manifests, and duplication drifts,
  so **[`ci/msrv.txt`](../ci/msrv.txt) is the single source of truth**
  and a CI stage fails when any manifest disagrees with it (§4).
- `rust-version` is not cosmetic. With edition 2024 (resolver `"3"`)
  Cargo is **MSRV-aware**: when resolving a dependency it prefers the
  newest version compatible with the declared floor, and it refuses to
  build when a locked dependency demands a newer compiler than the
  toolchain in use. Declaring the field is what turns "we support 1.95"
  from a README claim into something the build system enforces.

### Exempt: the `fuzz/` sub-crates

The `fuzz/` sub-crates are **excluded** and carry no `rust-version`.
They are `publish = false` scaffolding that only ever builds under
`cargo +nightly fuzz` with sanitizer instrumentation
([`.github/workflows/fuzz.yml`](../.github/workflows/fuzz.yml)); a
stable-toolchain floor would be a claim nothing checks and nobody could
use. Everything else — service crates, matcher crates, library crates,
and the `migration/` sub-crates — declares it.

## 3. MSRV vs. `rust-toolchain.toml` — three numbers, three jobs

They are routinely confused, so, explicitly:

| Number | Where | What it means |
|---|---|---|
| **1.98** (`N`) | upstream | the newest stable compiler that exists |
| **1.96.1** | [`rust-toolchain.toml`](../rust-toolchain.toml) | the compiler this repo *builds and formats with*, pinned so `cargo fmt` is byte-identical on every machine |
| **1.95** (`N-3`) | `ci/msrv.txt` + every `Cargo.toml` | the oldest compiler we *promise still works* |

The pin exists to stop rustfmt drift churning the tree; the MSRV exists
to stop us breaking a consumer. Bumping one does **not** imply bumping
the other, and they may legitimately sit several releases apart. What is
not legitimate is the pin dropping *below* the MSRV — that would mean
nobody, anywhere, is compiling the version we advertise.

## 4. How it is verified

MSRV is the kind of claim that rots the moment it is only written down,
so it is a CI stage like any other:

```sh
scripts/ci-check.sh msrv                       # every crate
scripts/ci-check.sh msrv person/person-matcher-rust-crate
```

The stage does two things:

1. **Consistency** — every non-`fuzz` `Cargo.toml` declares a
   `rust-version` and it equals `ci/msrv.txt`. A crate added without the
   field fails here, which is the only reliable way to keep ~50 manifests
   in step.
2. **Compilation** — `cargo +<msrv> check --all-targets` in each crate,
   which is where a `std` method stabilised in 1.96 is actually caught.
   `--all-targets` matters: benches and tests use APIs the library does
   not, and an MSRV that only holds for `src/` is not one a consumer can
   rely on.

Run locally with `rustup toolchain install 1.95 --profile minimal`
first; CI installs it in the job.

### The dependency floor is the binding constraint

Our own code compiles far below 1.95. What does not is the dependency
graph: as of 2026-08-20 the loco-based services transitively require

```
loco-rs 1.0.1, loco-gen 1.0.0        → rustc 1.94
sea-orm 2.0.0, sqlx 0.9.0            → rustc 1.94
```

so **1.94 is the true floor and 1.95 clears it by one release.** This is
worth watching rather than assuming: a dependency bump can raise the
floor above N-3 without a single line of our code changing, and the
`msrv` stage is what turns that into a red build instead of a surprise
in a consumer's CI. When it happens the choice is to pin the dependency
back or to raise the MSRV early and say so (§5) — never to quietly drop
the claim.

## 5. Bumping it

A bump is a **deliberate, single commit**, not a drift:

1. Recompute `N-3` from the current stable.
2. Update `ci/msrv.txt`, then sweep every `[package]` `rust-version` to
   match.
3. Update the table at the top of this document, including the dated
   `N` it was derived from.
4. `rustup toolchain install <new-msrv>` and run
   `scripts/ci-check.sh msrv` over the whole tree — green before the
   commit lands, not after.
5. Note it in each affected crate's `CHANGELOG.md`.

**An MSRV increase is a semver-visible change** for the crates published
to crates.io (`person-matcher`, `authentication-verifier`, the other
matchers): it can break a downstream build, so it belongs in at least a
minor version bump with a changelog line, never a patch.

Raising the MSRV *early* — ahead of the six-week cadence — is allowed
when a dependency forces it (§4), provided it is stated. Lowering it is
not: a consumer who upgraded on our word cannot un-upgrade.

## 6. What this means when writing code

- Before using a `std` API, check when it stabilised. If it is newer
  than the floor, either avoid it or bump the floor deliberately — do
  not discover it from a downstream bug report.
- `cargo add` may resolve a dependency version that raises the floor;
  the `msrv` stage catches it, but noticing at the time is cheaper.
- A crate that genuinely cannot hold the floor states the higher number
  in its own manifest **and** says why in its `spec/`. One crate above
  the floor with a reason is fine; a floor nobody actually checks is
  not.

## See also

- [`../rust-toolchain.toml`](../rust-toolchain.toml) — the build pin (§3)
- [`../ci/msrv.txt`](../ci/msrv.txt) — the declared floor
- [`../scripts/ci-check.sh`](../scripts/ci-check.sh) — the `msrv` stage
- [`tech-stack/index.md`](tech-stack/index.md) — the wider stack and its
  hard constraints
- [`testing/index.md`](testing/index.md) — the surrounding test strategy
