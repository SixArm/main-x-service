#![warn(clippy::pedantic)]

//! Property-based tests (SEC-M6).
//!
//! Each property generates many random inputs via `proptest` and checks
//! an invariant that must hold for **every** input. The goal is to prove
//! the matcher and its pure helpers never panic on arbitrary UTF-8, and
//! that scores stay well-behaved (bounded, non-NaN, symmetric, and
//! reflexive for a well-formed record). These catch the failure modes
//! that example-based tests miss: exotic Unicode, empty / whitespace-only
//! fields, and sparse-vs-dense `Organization` records.
//!
//! Note: the crate has no check-digit id validators (LEI/DUNS/GLN
//! deterministic matching is exact case-folded string comparison, not
//! check-digit validation). The `-> Option` pure helper standing in for
//! an "id validator" is [`phonetic::soundex`], fuzzed here alongside the
//! [`IdentifierScheme::is_deterministic`] scheme classifier.

use organization_matcher::{
    Confidence, IdentifierScheme, MatchingEngine, OrgIdentifier, Organization, PostalAddress,
    normalize, phonetic,
};
use proptest::prelude::*;

// ---------- Strategies ----------

/// Arbitrary UTF-8, including control characters, exotic scripts, and the
/// empty string — the adversarial input class the matcher must survive.
fn any_text() -> impl Strategy<Value = String> {
    any::<String>()
}

/// A name that normalises to a non-empty comparison key, so a well-formed
/// record self-matches on the (always-present) name component.
fn wellformed_name() -> impl Strategy<Value = String> {
    "[\\PC]{1,40}".prop_filter("normalises to an empty name key", |s| {
        !normalize::legal_name(s).is_empty()
    })
}

/// A deterministic-or-classification identifier scheme, plus a free-form
/// `Custom` label, so the deterministic short-circuit paths (R-0/R-1) are
/// exercised.
fn scheme_strategy() -> impl Strategy<Value = IdentifierScheme> {
    prop_oneof![
        Just(IdentifierScheme::Lei),
        Just(IdentifierScheme::Duns),
        Just(IdentifierScheme::Gln),
        Just(IdentifierScheme::Vat),
        Just(IdentifierScheme::TaxId),
        Just(IdentifierScheme::Naics),
        any_text().prop_map(IdentifierScheme::Custom),
    ]
}

/// An identifier with an arbitrary value.
fn identifier_strategy() -> impl Strategy<Value = OrgIdentifier> {
    (scheme_strategy(), any_text()).prop_map(|(scheme, value)| OrgIdentifier { scheme, value })
}

/// A lightweight optional address carrying only locality + postal code —
/// enough to exercise the field-by-field address scorer without a full
/// arbitrary `Strategy` over the nested type.
fn address_strategy() -> impl Strategy<Value = Option<PostalAddress>> {
    prop::option::of(
        (prop::option::of(any_text()), prop::option::of(any_text())).prop_map(|(loc, pc)| {
            PostalAddress {
                locality: loc,
                postal_code: pc,
                ..Default::default()
            }
        }),
    )
}

/// An `Organization` built by varying a handful of string fields (name,
/// url/domain, jurisdiction, keywords) plus identifiers, `same_as`, and a
/// slim address. Lightweight construction only — no arbitrary `Strategy`
/// for the full nested model.
fn org_strategy() -> impl Strategy<Value = Organization> {
    (
        any_text(),                              // name
        prop::option::of(any_text()),            // url
        prop::option::of(any_text()),            // jurisdiction
        prop::collection::vec(any_text(), 0..4), // keywords
        prop::collection::vec(identifier_strategy(), 0..3),
        prop::collection::vec(any_text(), 0..3), // same_as
        address_strategy(),
    )
        .prop_map(
            |(name, url, jurisdiction, keywords, identifiers, same_as, address)| {
                let mut o = Organization::new(name);
                o.url = url;
                o.jurisdiction = jurisdiction;
                o.keywords = keywords;
                o.identifiers = identifiers;
                o.same_as = same_as;
                o.address = address;
                o
            },
        )
}

/// A well-formed `Organization` whose name normalises to a non-empty key,
/// so it is guaranteed to match a clone of itself.
fn wellformed_org() -> impl Strategy<Value = Organization> {
    (
        wellformed_name(),
        prop::option::of("[\\PC]{1,20}"),            // jurisdiction
        prop::collection::vec("[\\PC]{1,15}", 0..3), // keywords
    )
        .prop_map(|(name, jurisdiction, keywords)| {
            let mut o = Organization::new(name);
            o.jurisdiction = jurisdiction;
            o.keywords = keywords;
            o
        })
}

// ---------- Properties ----------

proptest! {
    #![proptest_config(ProptestConfig { cases: 400, .. ProptestConfig::default() })]

    /// The engine and every one-to-many variant MUST NOT panic on
    /// arbitrary records, and the pure helpers MUST NOT panic on
    /// arbitrary UTF-8.
    #[test]
    fn engine_and_helpers_never_panic(a in org_strategy(), b in org_strategy()) {
        let engine = MatchingEngine::default_config();
        let _ = engine.match_organizations(&a, &b);
        let cands = [a.clone(), b.clone()];
        let _ = engine.match_one_to_many(&a, &cands);
        let _ = engine.rank(&a, &cands);
        let _ = engine.find_matches(&a, &cands);
        // Pure helpers on arbitrary UTF-8.
        let _ = normalize::fold(&a.name);
        let _ = normalize::legal_name(&a.name);
        let _ = normalize::domain(&a.name);
        let _ = normalize::fold_set(&a.keywords);
        let _ = phonetic::soundex(&a.name);
        let _ = phonetic::same(&a.name, &b.name);
        for id in &a.identifiers {
            let _ = id.scheme.is_deterministic();
        }
    }

    /// `score` MUST always land in `[0.0, 1.0]` and never be NaN.
    #[test]
    fn score_is_bounded_and_finite(a in org_strategy(), b in org_strategy()) {
        let r = MatchingEngine::default_config().match_organizations(&a, &b);
        prop_assert!(!r.score.is_nan(), "score was NaN");
        prop_assert!(r.score >= 0.0, "score < 0.0: {}", r.score);
        prop_assert!(r.score <= 1.0, "score > 1.0: {}", r.score);
    }

    /// Matching MUST be symmetric: `match(a,b)` and `match(b,a)` agree on
    /// score, `is_match`, `confidence`, and the deterministic flag.
    #[test]
    fn matching_is_symmetric(a in org_strategy(), b in org_strategy()) {
        let engine = MatchingEngine::default_config();
        let fwd = engine.match_organizations(&a, &b);
        let rev = engine.match_organizations(&b, &a);
        prop_assert!(
            (fwd.score - rev.score).abs() < 1e-9,
            "score asymmetric: {} vs {}",
            fwd.score,
            rev.score
        );
        prop_assert_eq!(fwd.is_match, rev.is_match);
        prop_assert_eq!(fwd.confidence, rev.confidence);
        prop_assert_eq!(fwd.breakdown.deterministic_match, rev.breakdown.deterministic_match);
    }

    /// A well-formed record MUST match an identical clone of itself:
    /// `is_match` holds and the score clears the configured threshold.
    #[test]
    fn identical_clone_self_matches(o in wellformed_org()) {
        let engine = MatchingEngine::default_config();
        let r = engine.match_organizations(&o, &o);
        prop_assert!(
            r.score >= engine.config().threshold,
            "self-score {} below threshold {}",
            r.score,
            engine.config().threshold
        );
        prop_assert!(r.is_match, "well-formed clone failed to self-match: {o:?}");
    }

    /// `phonetic::soundex` (the `-> Option` id-style validator) MUST NOT
    /// panic on arbitrary UTF-8, returns `Some` iff the input carries an
    /// ASCII-alphabetic anchor letter, and any `Some` code is exactly
    /// four characters. `phonetic::same` MUST NOT panic either.
    #[test]
    fn soundex_returns_option_and_never_panics(s in any_text(), t in any_text()) {
        let out = phonetic::soundex(&s);
        let has_alpha = s.chars().any(|c| c.is_ascii_alphabetic());
        prop_assert_eq!(out.is_some(), has_alpha);
        if let Some(code) = out {
            prop_assert_eq!(code.chars().count(), 4, "soundex code not 4 chars: {}", code);
        }
        let _ = phonetic::same(&s, &t);
    }

    /// `normalize::fold` MUST NOT panic and its output MUST carry no
    /// ASCII-uppercase characters; the sibling helpers MUST NOT panic.
    #[test]
    fn normalize_helpers_are_well_behaved(s in any_text()) {
        let f = normalize::fold(&s);
        prop_assert!(!f.chars().any(|c| c.is_ascii_uppercase()), "fold left ASCII uppercase: {f:?}");
        let _ = normalize::legal_name(&s);
        let _ = normalize::domain(&s);
        let _ = normalize::fold_set(std::slice::from_ref(&s));
    }

    /// `Confidence::classify` MUST be monotonic non-decreasing in score.
    #[test]
    fn confidence_is_monotonic(a in 0.0f64..=1.0, b in 0.0f64..=1.0) {
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        let rank = |c: Confidence| match c {
            Confidence::Low => 0u8,
            Confidence::Medium => 1,
            Confidence::High => 2,
        };
        prop_assert!(rank(Confidence::classify(hi)) >= rank(Confidence::classify(lo)));
    }
}
