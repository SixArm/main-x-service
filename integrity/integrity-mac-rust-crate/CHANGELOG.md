# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [README.md](./README.md), [fuzz/README.md](./fuzz/README.md).

## [Unreleased]

### Fixed — a stored MAC containing a multi-byte character crashed the reader

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
