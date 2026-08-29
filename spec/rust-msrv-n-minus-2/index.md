# Rust MSRV — current N-2

The **Minimum Supported Rust Version** for every Rust crate in this
repository is **the current stable release minus two**: if the newest
stable compiler is `1.N`, the MSRV is `1.(N-2)`.

| | |
|---|---|
| Current stable (`N`) | **1.98.0** (2026-08-18) |
| **Declared MSRV (`N-2`)** | **1.96** |
| Toolchain we *build* with | **1.96.1** (`rust-toolchain.toml`) |
| Single source of truth | [`ci/msrv.txt`](../ci/msrv.txt) |

At N-2 the MSRV's minor version happens to coincide with the build
pin's minor version (both `1.96`) — the pin still carries a patch
number the MSRV does not, so they remain two different numbers with
two different jobs (§3). This is closer together than the family's
former N-3 policy ever put them, and is a property of today's specific
numbers, not something the policy guarantees going forward — the next
stable release moves the MSRV but not the pin, and they will drift
apart again until the pin is next bumped.

## 1. The policy

- **MSRV = N-2.** Two releases is about **twelve weeks** of grace —
  shorter than the family's former N-3 (eighteen weeks), and a
  deliberate tightening: less time downstream consumers have to sit on
  an old compiler, and less time our own code can silently drift above
  a floor nobody is checking day to day.
- **It is a *moving* floor, not a frozen one.** Every six weeks a new
  stable lands and the computed MSRV moves up by one. The declared value
  is only re-derived when someone bumps it deliberately (§5) — the
  repository always states a concrete version, never a formula that a
  build has to evaluate.
- **The floor is a promise about compilation**, nothing more: every crate
  must `cargo check --all-targets` and `cargo test` on the MSRV
  toolchain. It says nothing about which toolchain a developer or CI
  uses day to day.
- **N-2 is a ceiling on our conservatism, not a target.** Nothing obliges
  a crate to sit exactly at the floor. What is forbidden is *silently*
  drifting above it: a language or `std` feature stabilised after 1.96
  may not be used until the floor moves.

### Why N-2 and not N-3

The family ran N-3 from 2026-08-20 (see `tasks.md` `MSRV-1`) until this
change. The reasoning for tightening to N-2:

- **N-3's eighteen weeks was more slack than this family's actual
  consumers need.** Nothing published to crates.io from this repo
  (`person-matcher`, `authentication-verifier`, the other matchers) has
  reported needing that much runway, and a tighter floor pushes
  everyone — including us — to notice a newly-stabilised `std` API
  sooner rather than banking three releases of not looking.
- **N-1 or newer** leaves under six weeks between a stable release and
  a build that might start requiring it — shorter than most
  organisations' toolchain review cycle, so a routine dependency bump
  on our side becomes an unplanned upgrade on theirs. That is the
  failure N-3 was originally chosen to avoid, and N-2 still avoids it.
- **N-2 still sits comfortably above the real dependency floor** (§4):
  as of this change the loco-based services transitively require
  1.94, so 1.96 clears it by two releases — one more release of
  headroom than N-3 had, not less, because stable itself moved on in
  the interim.

## 2. How it is declared

Every crate's `Cargo.toml` carries the version in its `[package]` table:

```toml
[package]
name = "person-matcher"
edition = "2024"
rust-version = "1.96"
```

- It is declared **per crate** rather than inherited, because this
  repository has **no root `Cargo.toml`** — each crate is its own
  workspace (see [`scripts/ci-crates.sh`](../scripts/ci-crates.sh)), so
  there is nothing to inherit from.
- The value is duplicated across 46 manifests, and duplication drifts,
  so **[`ci/msrv.txt`](../ci/msrv.txt) is the single source of truth**
  and a CI stage fails when any manifest disagrees with it (§4).
- `rust-version` is not cosmetic. With edition 2024 (resolver `"3"`)
  Cargo is **MSRV-aware**: when resolving a dependency it prefers the
  newest version compatible with the declared floor, and it refuses to
  build when a locked dependency demands a newer compiler than the
  toolchain in use. Declaring the field is what turns "we support 1.96"
  from a README claim into something the build system enforces.

### Exempt: the `fuzz/` sub-crates

The `fuzz/` sub-crates (18 of them) are **excluded** and carry no
`rust-version`. They are `publish = false` scaffolding that only ever
builds under `cargo +nightly fuzz` with sanitizer instrumentation
([`.github/workflows/fuzz.yml`](../.github/workflows/fuzz.yml)); a
stable-toolchain floor would be a claim nothing checks and nobody could
use. Everything else — service crates, matcher crates, library crates,
and the `migration/` sub-crates — declares it: 46 crates in total.

## 3. MSRV vs. `rust-toolchain.toml` — three numbers, two jobs

They are routinely confused, so, explicitly:

| Number | Where | What it means |
|---|---|---|
| **1.98** (`N`) | upstream | the newest stable compiler that exists |
| **1.96.1** | [`rust-toolchain.toml`](../rust-toolchain.toml) | the compiler this repo *builds and formats with*, pinned so `cargo fmt` is byte-identical on every machine |
| **1.96** (`N-2`) | `ci/msrv.txt` + every `Cargo.toml` | the oldest compiler we *promise still works* |

The pin exists to stop rustfmt drift churning the tree; the MSRV exists
to stop us breaking a consumer. Bumping one does **not** imply bumping
the other, and they may legitimately sit several releases apart — at
N-2 today they merely happen to share a minor version, a coincidence of
timing, not a rule (see the note under the table at the top). What is
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
   field fails here, which is the only reliable way to keep 46 manifests
   in step.
2. **Compilation** — `cargo +<msrv> check --all-targets` in each crate,
   which is where a `std` method stabilised in 1.97 is actually caught.
   `--all-targets` matters: benches and tests use APIs the library does
   not, and an MSRV that only holds for `src/` is not one a consumer can
   rely on.

Run locally with `rustup toolchain install 1.96 --profile minimal`
first; CI installs it in the job.

### The dependency floor is the binding constraint

Our own code compiles far below 1.96. What does not is the dependency
graph: as of this change the loco-based services transitively require

```
loco-rs 1.0.1, loco-gen 1.0.0        → rustc 1.94
sea-orm 2.0.0, sqlx 0.9.0            → rustc 1.94
```

so **1.94 is the true floor and 1.96 clears it by two releases.** This is
worth watching rather than assuming: a dependency bump can raise the
floor above N-2 without a single line of our code changing, and the
`msrv` stage is what turns that into a red build instead of a surprise
in a consumer's CI. When it happens the choice is to pin the dependency
back or to raise the MSRV early and say so (§5) — never to quietly drop
the claim.

## 5. Bumping it

A bump is a **deliberate, single commit**, not a drift:

1. Recompute `N-2` from the current stable.
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
not: a consumer who upgraded on our word cannot un-upgrade. Widening
the gap back out to N-3 or further is a deliberate policy reversal, not
a routine bump: it belongs in `plan.md`, argued the same way this
change argued for tightening (§1), not decided inside a version-number
update.

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
- A tighter N-2 window means less runway between "a `std` feature
  stabilises" and "the floor might need to move" — treat a
  stable-release announcement as a prompt to re-check §4's dependency
  floor, not just a calendar event.

## See also

- [`../rust-toolchain.toml`](../rust-toolchain.toml) — the build pin (§3)
- [`../ci/msrv.txt`](../ci/msrv.txt) — the declared floor
- [`../scripts/ci-check.sh`](../scripts/ci-check.sh) — the `msrv` stage
- [`tech-stack/index.md`](../tech-stack/index.md) — the wider stack and its
  hard constraints
- [`testing/index.md`](../testing/index.md) — the surrounding test strategy
