#![warn(clippy::pedantic)]

//! Criterion benchmarks for `integrity-mac`.
//!
//! Run with `cargo bench`. Every audited write in the family computes a
//! MAC, and every integrity check verifies one, so this is per-row cost
//! on paths that already hold a database transaction open. Three things
//! the numbers are meant to answer:
//!
//! - **What does one tag cost?** The `tag_by_preimage_size` group sweeps
//!   the pre-image size, so the fixed HMAC overhead and the per-byte
//!   slope are separable rather than conflated in a single figure.
//! - **Is derivation paid once or per call?** `KeySet::load` runs HKDF;
//!   `tag`/`verify` should not. The `load` group prices the boot-time
//!   cost, and `pre_declared_domain` versus `on_demand_domain` shows
//!   what a domain that was *not* pre-declared costs on every call —
//!   which is the difference between deriving at boot and deriving in
//!   the hot loop.
//! - **Are the reject paths cheap for the right reason?** A malformed
//!   stored value should be rejected by parsing, long before any HMAC
//!   work.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use integrity_mac::{KeyConfig, KeySet, MacVerdict};
use std::hint::black_box;

/// A realistic 32-byte root key — varied bytes, so it passes the
/// placeholder rule and the key set is actually enabled.
const ROOT_KEY: &str = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";

const CONFIG: KeyConfig = KeyConfig::new("bench-service", "BENCH");
const AUDIT: &str = "audit-chain";
const RECORD: &str = "record";

/// A pre-image of `n` bytes, shaped like the row digests these MACs
/// actually cover (a delimited concatenation of column values).
fn preimage(n: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(n);
    while v.len() < n {
        v.extend_from_slice(b"person|0c4f1e2a-0000-4000-8000-000000000000|updated|");
    }
    v.truncate(n);
    v
}

/// Key-set construction: HKDF derivation, paid once at boot. Pre-declaring
/// the domains moves their derivation here, out of the per-call path.
fn bench_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("load");
    group.bench_function("no_domains", |b| {
        b.iter(|| KeySet::load(black_box(&CONFIG), black_box(Some(ROOT_KEY)), None));
    });
    group.bench_function("two_domains_pre_declared", |b| {
        b.iter(|| {
            KeySet::load_with_domains(
                black_box(&CONFIG),
                black_box(Some(ROOT_KEY)),
                None,
                None,
                &[AUDIT, RECORD],
            )
        });
    });
    group.bench_function("with_two_retired_keys", |b| {
        let retired = format!("k0:{ROOT_KEY},k00:{ROOT_KEY}");
        b.iter(|| {
            KeySet::load(
                black_box(&CONFIG),
                black_box(Some(ROOT_KEY)),
                Some(&retired),
            )
        });
    });
    group.finish();
}

/// Tagging cost against pre-image size, with `Throughput::Bytes` so
/// Criterion reports throughput and the per-byte slope is visible.
fn bench_tag_by_preimage_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("tag_by_preimage_size");
    let keys = KeySet::load_with_domains(&CONFIG, Some(ROOT_KEY), None, None, &[AUDIT]);

    for &n in &[64usize, 1024, 65536] {
        let bytes = preimage(n);
        group.throughput(Throughput::Bytes(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &bytes, |b, bytes| {
            b.iter(|| keys.tag(black_box(AUDIT), black_box(bytes)));
        });
    }
    group.finish();
}

/// A pre-declared domain versus one derived on demand. The gap is the
/// per-call HKDF a service pays for forgetting to declare its domains.
fn bench_domain_derivation(c: &mut Criterion) {
    let mut group = c.benchmark_group("domain_derivation");
    let keys = KeySet::load_with_domains(&CONFIG, Some(ROOT_KEY), None, None, &[AUDIT]);
    let bytes = preimage(256);

    group.bench_function("pre_declared_domain", |b| {
        b.iter(|| keys.tag(black_box(AUDIT), black_box(&bytes)));
    });
    group.bench_function("on_demand_domain", |b| {
        b.iter(|| keys.tag(black_box("not-pre-declared"), black_box(&bytes)));
    });
    group.finish();
}

/// Verification: the match path, the mismatch path (the one that fires
/// on a tampered row), and the parse-reject paths, which should cost
/// almost nothing because they never reach the HMAC.
fn bench_verify(c: &mut Criterion) {
    let mut group = c.benchmark_group("verify");
    let keys = KeySet::load_with_domains(&CONFIG, Some(ROOT_KEY), None, None, &[AUDIT]);
    let bytes = preimage(256);
    let tag = keys.tag(AUDIT, &bytes).expect("enabled");
    let tampered = preimage(257);

    group.bench_function("valid", |b| {
        b.iter(|| {
            assert_eq!(
                keys.verify(black_box(AUDIT), black_box(Some(&tag)), black_box(&bytes)),
                MacVerdict::Valid
            );
        });
    });
    group.bench_function("content_changed", |b| {
        b.iter(|| {
            keys.verify(
                black_box(AUDIT),
                black_box(Some(&tag)),
                black_box(&tampered),
            )
        });
    });
    group.bench_function("unknown_key_id", |b| {
        let other = tag.replace("k1:", "k9:");
        b.iter(|| keys.verify(black_box(AUDIT), black_box(Some(&other)), black_box(&bytes)));
    });
    group.bench_function("malformed", |b| {
        b.iter(|| {
            keys.verify(
                black_box(AUDIT),
                black_box(Some("not-a-mac")),
                black_box(&bytes),
            )
        });
    });
    group.bench_function("absent", |b| {
        b.iter(|| keys.verify(black_box(AUDIT), black_box(None), black_box(&bytes)));
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_load,
    bench_tag_by_preimage_size,
    bench_domain_derivation,
    bench_verify
);
criterion_main!(benches);
