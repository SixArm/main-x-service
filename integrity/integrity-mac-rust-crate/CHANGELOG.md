# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [README.md](./README.md), [fuzz/README.md](./fuzz/README.md).
>
> **Documentation-tier waiver (PRO-P26, 2026-08-29).** This crate has no
> `spec/`, `AGENTS.md`, `CLAUDE.md`, or `index.md` — the same posture as
> [`link/entity-ref-rust-crate`](../../link/entity-ref-rust-crate). The
> reason is not the same reason, though: entity-ref is waived because it
> is trivially small; this crate is waived because its design rationale
> is *already* written down in full prose, not scattered code comments —
> and a `spec/` would duplicate that prose rather than add to it. The
> `src/lib.rs` module doc (`//! Why this is a crate rather than a copy`,
> `//! What the MAC defends against, stated plainly`, `//! Domain
> separation`) covers the threat model, the HKDF-SHA256 per-(service,
> domain) derivation design and the two failure modes it closes, and the
> extraction rationale; the doc comments on every public item cover the
> fail-closed posture item by item (key-file-over-inline sourcing with no
> silent fallback, root-key zeroization, placeholder refusal, the
> scheme-tag migration story). The README repeats the same design as a
> non-rustdoc entry point plus an explicit "what it does not defend
> against" section. The family-wide integration story — which services
> embed this crate, in what order, and how to activate the MAC in
> production — lives in
> [`agents/share/security.md`](../../agents/share/security.md) invariant 1
> and [`agents/share/runbooks/integrity-activation.md`](../../agents/share/runbooks/integrity-activation.md),
> which is where that cross-crate narrative belongs rather than in a
> single library's own spec. If this crate's design ever grows a facet
> that isn't yet prose anywhere — a new key scheme, a new sourcing
> precedence rule — the obligation is to extend that prose (rustdoc,
> README, or this file), not to open a `spec/` that would need to be kept
> in lock-step with it.

## [Unreleased]

### Fixed — doc drift: stale MSRV 1.95/N-3 reference (2026-09-06)

`Cargo.toml` already declares `rust-version = "1.96"` (matching
`ci/msrv.txt` and the repository's current **N-2** policy), but the
`[0.2.0] - 2026-08-21` entry below still says `"1.95"` and "current
stable minus three" — the policy in effect when that entry was
written, since tightened. That entry is left as-written (a dated
record of what was true then); this entry is the correction, matching
the pattern already used for course-service (T-29) and
authentication-verifier (AV-3). No behaviour change — `Cargo.toml`,
`ci/msrv.txt`, and `scripts/ci-check.sh msrv` already agreed on 1.96
before this change.

### Fixed — missing manifest `readme` field (IM-3)

`Cargo.toml` had no `readme` field, so a future published crates.io
page would show only the one-line `description`, not this file's
"Why it exists" / "What it does not defend against" sections. Added
`readme = "README.md"`, ahead of the IM-1 publish. `cargo package
--list` now includes `README.md`.

## [0.2.0] - 2026-08-21

### Security — a stored MAC containing a multi-byte character crashed the reader (SEC-M7)

`decode_hex` indexed the input as a `&str` (`&s[i..i + 2]`), which
panics when the boundary lands inside a multi-byte character. It runs on
two inputs this crate does not control:

- the **stored MAC string**, which an adversary holding only the
  database — the exact adversary this crate exists to defend against —
  can rewrite to anything, and
- an **operator-supplied key**, via `assess_key_hex`.

So `KeySet::verify(domain, Some("k1:€a"), …)` aborted the process
instead of returning `MacVerdict::Malformed`. That violates the
family's never-panic-on-untrusted-input invariant
(`agents/share/security.md` §3, invariant 2) and turns a database-write
foothold into a crash on every reader of the affected row.

Decoding now works over **bytes**, so no slice can split a character.
Pinned by a unit test and by the new `verify_tag` fuzz target, which is
what found it.

**Behaviour change, deliberate:** the replaced `u8::from_str_radix`
accepted a leading sign, so `"+f"` decoded as `0x0f`. Nothing writes
such a value, and a sign in the middle of a hex digest is a corrupted
row rather than a number; it is now rejected.

### Added — cargo-fuzz harness (SEC-I2)

A `fuzz/` [`cargo-fuzz`](https://rust-fuzz.github.io/book/) crate with
three coverage-guided libFuzzer targets, following the matcher family's
scaffolding:

- **`verify_tag`** — `KeySet::verify` over an arbitrary stored value and
  pre-image. Never-panic; our own tag always verifies; a tag never
  transfers between HKDF domains; a derived-scheme value that verifies
  really is the tag we would have written.
- **`assess_key`** — `assess_key_hex` over arbitrary UTF-8. Never-panic;
  a `Usable` verdict decodes to at least `MIN_KEY_LEN`; and the
  pre-flight checker and the loader never disagree about a key.
- **`load_keys`** — `KeySet::load` over an arbitrary inline key and
  retired-key list. Never-panic; an enabled key set is self-consistent
  (tag → verify → `Valid`, tag carries the advertised prefix, the key id
  round-trips through the lookup).

Nightly only, so the sub-crate is exempt from the repository MSRV. See
[`fuzz/README.md`](./fuzz/README.md).

### Added — Criterion benchmarks

`benches/mac.rs` prices the per-row cost this crate adds to every
audited write: key-set load (the HKDF derivation, paid once at boot),
tagging against pre-image size (`Throughput::Bytes`, so the fixed
overhead and the per-byte slope separate), a pre-declared domain versus
one derived on demand, and each verification verdict — including the
parse-reject paths, which should cost almost nothing because they never
reach the HMAC.

### Changed — hex encoding no longer formats byte by byte

`to_hex` built its output by `write!`ing each byte into a growing
`String`. Formatting machinery per byte cost most of a microsecond on
every 32-byte digest, on a path every audited write takes. It is now a
nibble lookup into a pre-allocated `String` — measured ~25% faster for a
tag over a small pre-image (`benches/mac.rs`, which is how it surfaced).
Output is byte-identical.

### Added — declared MSRV (Rust 1.95)

`Cargo.toml` declares `rust-version = "1.95"`, the repository's current
stable minus three floor (`spec/rust-msrv-n-minus-3/index.md`), sourced from
`ci/msrv.txt` and enforced by `scripts/ci-check.sh msrv`.

## [0.1.0] - 2026-07-27

### Added

Initial extraction. Keyed integrity MACs (HMAC-SHA256, FIPS 198-1) with
HKDF-SHA256 per-(service, domain) subkey derivation, key-file sourcing
that takes precedence over the environment and never falls back, root-key
zeroization, placeholder refusal, and key generation from the OS CSPRNG —
one audited implementation instead of a copy per service, because a
key-handling defect in a copy would make MACs forgeable while every test
stayed green.
