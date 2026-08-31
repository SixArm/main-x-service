#![warn(clippy::pedantic)]

//! Criterion benchmarks for `authentication-verifier`.
//!
//! Run with `cargo bench`. This crate sits on the **per-request** path of
//! every service in the family: each incoming call verifies a PASETO
//! v4.public token offline and then evaluates an ABAC policy. Both are
//! pure CPU with no I/O, so their cost is a fixed tax on every request —
//! which is exactly the kind of cost that goes unnoticed until it is
//! measured.
//!
//! Two things worth reading off the results:
//!
//! - **Reject paths must not be cheap by accident.** A bad signature
//!   costs roughly what a good one does, because the Ed25519 verify runs
//!   either way. A *much* cheaper reject would mean something is
//!   short-circuiting before the cryptography.
//! - **Policy evaluation is O(rules), first-match-wins.** The
//!   `policy_rule_count` group makes that linearity visible, so a
//!   deployment can see what a hundred-rule policy costs before writing
//!   one.

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use authentication_verifier::abac::{Action, Policy};
use authentication_verifier::{Claims, Verifier};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use ed25519_dalek::SigningKey;
use rusty_paseto::core::{Footer, Key, Paseto, PasetoAsymmetricPrivateKey, Payload, Public, V4};
use std::hint::black_box;

/// A fixed seed → a deterministic keypair. Benchmark-only; the real
/// service reads its seed from the environment.
const SEED: [u8; 32] = [7u8; 32];
const ISSUER: &str = "authentication-service";
const AUDIENCE: &str = "main-x-service";
const KID: &str = "bench-key-1";
const ENTITY: &str = "person";

fn signing_key() -> SigningKey {
    SigningKey::from_bytes(&SEED)
}

/// The published key-set document, exactly as the auth-service serves it
/// at `/.well-known/paseto-keys`.
fn keys_document(seed: &[u8; 32]) -> serde_json::Value {
    let public = SigningKey::from_bytes(seed).verifying_key().to_bytes();
    serde_json::json!({
        "keys": [{
            "kty": "OKP", "crv": "Ed25519", "use": "sig",
            "kid": KID, "x": URL_SAFE_NO_PAD.encode(public),
        }]
    })
}

/// Realistic claims: an unexpired token carrying the coarse `access`
/// attribute plus a couple of deployment-specific ones, so policy
/// evaluation has real attributes to match rather than an empty map.
fn claims() -> Claims {
    let now = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock is after the epoch")
            .as_secs(),
    )
    .expect("seconds since the epoch fit in i64");
    let mut attrs = BTreeMap::new();
    attrs.insert("access".to_string(), vec!["write".to_string()]);
    attrs.insert("dept".to_string(), vec!["cardiology".to_string()]);
    attrs.insert("purpose".to_string(), vec!["care".to_string()]);
    Claims {
        sub: "11111111-1111-1111-1111-111111111111".to_string(),
        email: "alice@example.com".to_string(),
        name: "Alice".to_string(),
        iss: ISSUER.to_string(),
        aud: AUDIENCE.to_string(),
        exp: now + 300,
        iat: now,
        nbf: None,
        sid: "22222222-2222-2222-2222-222222222222".to_string(),
        scope: vec![],
        roles: vec![],
        attrs,
    }
}

/// Mint a v4.public token with `seed` — the inverse of what the verifier
/// does, so a benchmark needs no running auth-service.
fn mint(seed: &[u8; 32], claims: &Claims) -> String {
    let payload = serde_json::to_string(claims).expect("claims serialize");
    let keypair = SigningKey::from_bytes(seed).to_keypair_bytes();
    let key = Key::<64>::from(keypair);
    let private = PasetoAsymmetricPrivateKey::<V4, Public>::from(&key);
    let footer = format!(r#"{{"kid":"{KID}"}}"#);
    let mut builder = Paseto::<V4, Public>::builder();
    builder.set_payload(Payload::from(payload.as_str()));
    builder.set_footer(Footer::from(footer.as_str()));
    builder.try_sign(&private).expect("sign")
}

/// Offline token verification — the per-request cost every guarded
/// endpoint pays before it does any work of its own.
fn bench_verify(c: &mut Criterion) {
    let mut group = c.benchmark_group("verify");
    let verifier = Verifier::from_paseto_keys_value(&keys_document(&SEED), ISSUER, AUDIENCE)
        .expect("key set loads");
    assert_eq!(verifier.key_count(), 1);

    let claims = claims();
    let good = mint(&SEED, &claims);
    // Signed by a different key, so the signature check must fail while
    // everything up to it succeeds.
    let forged = mint(&[9u8; 32], &claims);
    let garbage = "v4.public.not-a-real-token";

    group.bench_function("valid_token", |b| {
        b.iter(|| verifier.verify(black_box(&good)).expect("valid"));
    });
    group.bench_function("bad_signature", |b| {
        b.iter(|| verifier.verify(black_box(&forged)).unwrap_err());
    });
    group.bench_function("malformed_token", |b| {
        b.iter(|| verifier.verify(black_box(garbage)).unwrap_err());
    });
    group.finish();

    // Keep the signing key alive in the benchmark's mind: dropping it
    // would be fine, but referencing it documents that verification uses
    // only the public half.
    let _ = signing_key();
}

/// ABAC evaluation against the built-in default policy, across the four
/// derived actions. `read` is the default-allow path and `destructive`
/// the one that has to walk furthest before deciding.
fn bench_default_policy(c: &mut Criterion) {
    let mut group = c.benchmark_group("default_policy");
    let policy = Policy::default_policy();
    let claims = claims();

    for (label, action) in [
        ("read", Action::Read),
        ("write", Action::Write),
        ("delete", Action::Delete),
        ("destructive", Action::Destructive),
    ] {
        group.bench_function(label, |b| {
            b.iter(|| policy.evaluate(black_box(&claims), action, black_box(ENTITY)));
        });
    }
    group.finish();
}

/// Record-level and environment-aware evaluation — the finer decision a
/// handler runs *after* loading the target record.
fn bench_contextual_policy(c: &mut Criterion) {
    let mut group = c.benchmark_group("contextual_policy");
    let policy = Policy::default_policy();
    let claims = claims();

    let mut resource = BTreeMap::new();
    resource.insert("case_type".to_string(), vec!["housing".to_string()]);
    resource.insert("status".to_string(), vec!["open".to_string()]);
    resource.insert("owner".to_string(), vec![claims.sub.clone()]);
    let mut env = BTreeMap::new();
    env.insert("hour".to_string(), vec!["22".to_string()]);
    env.insert("after_hours".to_string(), vec!["true".to_string()]);

    group.bench_function("with_resource", |b| {
        b.iter(|| {
            policy.evaluate_with_resource(black_box(&claims), Action::Write, ENTITY, &resource)
        });
    });
    group.bench_function("with_context", |b| {
        b.iter(|| {
            policy.evaluate_with_context(black_box(&claims), Action::Write, ENTITY, &resource, &env)
        });
    });
    group.finish();
}

/// Evaluation cost against policies of 1 / 10 / 100 rules, none of which
/// match until the last — the worst case for first-match-wins, and the
/// shape that shows the O(rules) linearity directly.
fn bench_policy_rule_count(c: &mut Criterion) {
    let mut group = c.benchmark_group("policy_rule_count");
    let claims = claims();

    for &n in &[1usize, 10, 100] {
        // n-1 rules that cannot match (an attribute nobody carries),
        // then one that does — so every rule is examined.
        let mut rules: Vec<String> = (0..n.saturating_sub(1))
            .map(|i| {
                format!(
                    r#"{{"effect":"allow","actions":["write"],"when":{{"unit":["ward-{i}"]}}}}"#
                )
            })
            .collect();
        rules.push(
            r#"{"effect":"allow","actions":["write"],"when":{"access":["write","admin"]}}"#
                .to_string(),
        );
        let json = format!(r#"{{"rules":[{}]}}"#, rules.join(","));
        let policy = Policy::from_json(&json).expect("policy parses");

        group.bench_with_input(BenchmarkId::from_parameter(n), &policy, |b, policy| {
            b.iter(|| policy.evaluate(black_box(&claims), Action::Write, ENTITY));
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_verify,
    bench_default_policy,
    bench_contextual_policy,
    bench_policy_rule_count
);
criterion_main!(benches);
