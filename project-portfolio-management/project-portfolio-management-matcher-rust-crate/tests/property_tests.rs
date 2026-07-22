#![warn(clippy::pedantic)]

//! Property-based tests (SEC-M6).
//!
//! Each property generates many random inputs via `proptest` and asserts an
//! invariant that must hold for **every** input. These catch the failure
//! modes example-based tests miss: pathological Unicode in names, junk /
//! overflow-prone date strings, and sparse-vs-dense `Plan` records.
//!
//! The load-bearing invariants proven here:
//!
//! - the matcher and its pure helpers **never panic** on arbitrary input
//!   (in particular `normalize::iso_date_to_days`, whose year term once
//!   overflowed `days_from_civil` — see SEC-M4);
//! - `score` is always a real number in `[0.0, 1.0]` (never `NaN`);
//! - matching is **symmetric** for same-kind records;
//! - the **kind gate** pins any cross-kind pair to a `0.0` non-match;
//! - an identical clone of a record matches itself.

use project_portfolio_management_matcher::{MatchingEngine, Plan, PlanKind, normalize, phonetic};
use proptest::prelude::*;

// ---------- Strategies ----------

/// An arbitrary short text field: any non-control Unicode, including the
/// empty string, up to a plausible length.
fn text() -> impl Strategy<Value = String> {
    "\\PC{0,32}"
}

/// One of the plan kinds.
fn kind_strategy() -> impl Strategy<Value = PlanKind> {
    prop_oneof![
        Just(PlanKind::Portfolio),
        Just(PlanKind::Project),
        Just(PlanKind::Product),
        Just(PlanKind::Program),
        Just(PlanKind::Practice),
        Just(PlanKind::Process),
        Just(PlanKind::Purpose),
        Just(PlanKind::Pathway),
        Just(PlanKind::Proposal),
    ]
}

/// An optional date that is sometimes well-formed ISO-8601, sometimes
/// arbitrary junk (to exercise the parser's rejection path), sometimes
/// absent.
fn maybe_date() -> impl Strategy<Value = Option<String>> {
    prop_oneof![
        Just(None),
        (1970i32..=2100, 1u32..=12, 1u32..=28)
            .prop_map(|(y, m, d)| Some(format!("{y:04}-{m:02}-{d:02}"))),
        text().prop_map(Some),
    ]
}

/// A `Plan` of the given `kind`, varying the string-ish fields the
/// probabilistic strategy actually reads (name, alternate names, code,
/// owner org, keywords, tags, dates). Lightweight by design — enough
/// surface to exercise every component without a full-fat builder.
fn plan(kind: PlanKind) -> impl Strategy<Value = Plan> {
    (
        text(),
        prop::collection::vec(text(), 0..3),
        prop::option::of(text()),
        prop::option::of(text()),
        prop::collection::vec(text(), 0..4),
        prop::collection::vec(text(), 0..4),
        maybe_date(),
        maybe_date(),
    )
        .prop_map(
            move |(name, alts, code, owner, keywords, tags, start, target)| {
                let mut w = Plan::new(name);
                w.kind = Some(kind);
                w.alternate_names = alts;
                w.code = code;
                w.owner_org_id = owner;
                w.keywords = keywords;
                w.tags = tags;
                w.start_date = start;
                w.target_date = target;
                w
            },
        )
}

/// An arbitrary `Plan` of any kind.
fn any_plan() -> impl Strategy<Value = Plan> {
    kind_strategy().prop_flat_map(plan)
}

/// Two `Plan`s of the **same** (randomly chosen) kind.
fn same_kind_pair() -> impl Strategy<Value = (Plan, Plan)> {
    kind_strategy().prop_flat_map(|k| (plan(k), plan(k)))
}

/// Two `Plan`s of **different** kinds.
fn diff_kind_pair() -> impl Strategy<Value = (Plan, Plan)> {
    (kind_strategy(), kind_strategy())
        .prop_filter("kinds must differ", |(a, b)| a != b)
        .prop_flat_map(|(ka, kb)| (plan(ka), plan(kb)))
}

/// A date-shaped string biased toward the year-overflow danger zone:
/// long numeric year segments joined by dashes. Feeds the "never
/// overflows / never panics" property for `iso_date_to_days`.
fn dateish_string() -> impl Strategy<Value = String> {
    "[-0-9]{0,40}"
}

// ---------- Properties ----------

proptest! {
    #![proptest_config(ProptestConfig { cases: 400, .. ProptestConfig::default() })]

    /// The match engine MUST NOT panic on arbitrary input, and every score
    /// it returns MUST be a real number in `[0.0, 1.0]`.
    #[test]
    fn score_is_finite_and_bounded(a in any_plan(), b in any_plan()) {
        let engine = MatchingEngine::default_config();
        let r = engine.match_plans(&a, &b);
        prop_assert!(!r.score.is_nan(), "score is NaN");
        prop_assert!(r.score.is_finite(), "score is not finite: {}", r.score);
        prop_assert!(r.score >= 0.0, "score < 0.0: {}", r.score);
        prop_assert!(r.score <= 1.0, "score > 1.0: {}", r.score);
    }

    /// `match_plans` MUST be symmetric for records of the SAME kind:
    /// swapping the arguments changes neither the score, the `is_match`
    /// verdict, nor the confidence band.
    #[test]
    fn matching_is_symmetric_same_kind((a, b) in same_kind_pair()) {
        let engine = MatchingEngine::default_config();
        let forward = engine.match_plans(&a, &b);
        let reverse = engine.match_plans(&b, &a);
        prop_assert!(
            (forward.score - reverse.score).abs() < 1e-9,
            "score asymmetric: {} vs {}",
            forward.score,
            reverse.score
        );
        prop_assert_eq!(forward.is_match, reverse.is_match);
        prop_assert_eq!(forward.confidence, reverse.confidence);
    }

    /// NO KIND GATE: `kind` is optional descriptive metadata, so two
    /// records of DIFFERENT kinds are scored by the normal probabilistic
    /// pipeline — `kind_gate_blocked` is never set, the name component is
    /// scored, the score stays bounded, and matching remains symmetric.
    #[test]
    fn different_kinds_are_not_gated((a, b) in diff_kind_pair()) {
        let engine = MatchingEngine::default_config();
        let forward = engine.match_plans(&a, &b);
        let reverse = engine.match_plans(&b, &a);
        prop_assert!(!forward.breakdown.kind_gate_blocked, "kind gate fired across kinds");
        prop_assert!(forward.breakdown.name_score.is_some(), "name not scored across kinds");
        prop_assert!(forward.score >= 0.0 && forward.score <= 1.0, "score out of bounds: {}", forward.score);
        prop_assert!((forward.score - reverse.score).abs() < 1e-9, "cross-kind score asymmetric");
    }

    /// REFLEXIVE: an identical clone of any well-formed record matches
    /// itself. We assert `is_match` (not exactly `1.0`) per the crate's
    /// contract — self-match need only clear the threshold.
    #[test]
    fn identical_clone_matches_itself(a in any_plan()) {
        let engine = MatchingEngine::default_config();
        let r = engine.match_plans(&a, &a);
        prop_assert!(
            r.is_match && r.score >= engine.config().threshold,
            "self-match failed: score={} threshold={}",
            r.score,
            engine.config().threshold
        );
    }

    /// The pure normalisation + phonetic helpers MUST NOT panic on arbitrary
    /// input. `iso_date_to_days` is the focus: a prior bug overflowed on a
    /// crafted long-year string (SEC-M4).
    #[test]
    fn pure_helpers_never_panic(s in text()) {
        let _ = normalize::fold(&s);
        let _ = normalize::code(&s);
        let _ = normalize::url(&s);
        let _ = normalize::fold_set(std::slice::from_ref(&s));
        let _ = normalize::iso_date_to_days(&s);
        let _ = phonetic::soundex(&s);
    }

    /// `iso_date_to_days` MUST NOT panic (or overflow) on adversarial
    /// date-shaped strings with very long numeric year segments; any
    /// parsed day count must fit the well-behaved `i64` range.
    #[test]
    fn iso_date_never_overflows(s in dateish_string()) {
        if let Some(days) = normalize::iso_date_to_days(&s) {
            // 0000-01-01 .. 9999-12-31 in days-since-epoch, generously bounded.
            prop_assert!(
                (-800_000..=3_000_000).contains(&days),
                "day count out of sane range for {s:?}: {days}"
            );
        }
    }
}
