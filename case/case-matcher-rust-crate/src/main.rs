//! Demo binary — a runnable walkthrough of the `case-matcher` public
//! API. Not part of the public API's `SemVer` surface.

#![forbid(unsafe_code)] // demo binary needs no `unsafe` either.
#![deny(missing_docs)]
#![warn(clippy::pedantic)]

/// Use mimalloc as the global allocator on MUSL static builds, where it
/// noticeably outperforms the default allocator. Gated to `musl` so glibc
/// builds keep the system allocator.
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use case_matcher::{
    Case, CaseIdentifier, CaseStatus, CaseType, IdentifierScheme, MatchConfig, MatchingEngine,
};

/// Demo entry point: prints four representative scenarios (fuzzy title +
/// subject, deterministic docket short-circuit, same-agency case number,
/// and an unrelated pair) so the public API can be exercised by running
/// `cargo run`. Side effects: writes to stdout only.
fn main() {
    let engine = MatchingEngine::new(MatchConfig::default());

    println!("== case-matcher demo ==\n");

    // 1. Fuzzy title + shared subject + corroborating type/status.
    let mut a = Case::new("Housing benefit appeal — J. Smith");
    a.subjects = vec!["person:pid-42".into()];
    a.case_type = Some(CaseType::Housing);
    a.status = Some(CaseStatus::Open);
    let mut b = Case::new("Housing benefit appeal — John Smith");
    b.subjects = vec!["person:pid-42".into()];
    b.case_type = Some(CaseType::Housing);
    b.status = Some(CaseStatus::Open);
    let r = engine.match_cases(&a, &b);
    println!(
        "title+subject  : score={:.3}  {:?}  is_match={}",
        r.score, r.confidence, r.is_match
    );

    // 2. Deterministic docket short-circuit despite different titles.
    let mut a = Case::new("Smith v. Housing Authority");
    let mut b = Case::new("Appeal of benefit denial");
    a.identifiers.push(CaseIdentifier {
        scheme: IdentifierScheme::Docket,
        value: "CV-2024-001234".into(),
    });
    b.identifiers.push(CaseIdentifier {
        scheme: IdentifierScheme::Docket,
        value: "cv-2024-001234".into(),
    });
    let r = engine.match_cases(&a, &b);
    println!(
        "docket id      : score={:.3}  deterministic={}",
        r.score, r.breakdown.deterministic_match
    );

    // 3. Same-agency case number.
    let mut a = Case::new("Benefit claim v1");
    let mut b = Case::new("Benefit claim v2");
    a.agency_id = Some("agency-1".into());
    b.agency_id = Some("agency-1".into());
    a.case_number = Some("CV-2024-001234".into());
    b.case_number = Some("cv 2024 001234".into());
    let r = engine.match_cases(&a, &b);
    println!(
        "case number    : score={:.3}  deterministic={}",
        r.score, r.breakdown.deterministic_match
    );

    // 4. Unrelated.
    let a = Case::new("Housing benefit appeal — J. Smith");
    let b = Case::new("Commercial driving licence renewal");
    let r = engine.match_cases(&a, &b);
    println!(
        "unrelated      : score={:.3}  is_match={}",
        r.score, r.is_match
    );
}
