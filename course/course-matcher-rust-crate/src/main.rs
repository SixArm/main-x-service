//! # Course matcher demo binary
//!
//! A runnable demonstration of the `course-matcher` library. It builds a
//! handful of [`Course`] records and prints the deterministic and
//! probabilistic match results for each pair, illustrating perfect
//! matches, near matches, phonetic bonuses, and non-matches.
//!
//! Run it with `cargo run`. This binary is not part of the library's
//! `SemVer` surface; see the [`course_matcher`] crate documentation for
//! the supported public types.

// Always start with high quality coding conventions.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]

// When we build for MUSL static, use faster memory allocator.
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use course_matcher::{
    Course, CourseIdentifier, EducationalLevel, IdentifierScheme, MatchConfig, MatchingEngine,
};

fn main() {
    println!("Course matcher");
    println!("================\n");

    let engine = MatchingEngine::default_config();

    demo_deterministic_provider_code(&engine);
    demo_deterministic_doi(&engine);
    demo_fuzzy_name_keywords(&engine);
    demo_phonetic_bonus(&engine);
    demo_educational_level_adjacency(&engine);
    demo_no_match(&engine);
    demo_strict_vs_lenient();
    demo_rank_candidates(&engine);
}

/// Example 1: Deterministic match — same provider + same course code.
fn demo_deterministic_provider_code(engine: &MatchingEngine) {
    println!("Example 1: Same provider + course code (deterministic)");
    let mut intro_cs = Course::new("Introduction to Computer Science");
    intro_cs.provider_id = Some("ror-021nxhr62".into());
    intro_cs.course_code = Some("CS101".into());
    let mut intro_cs_variant = Course::new("Intro to Computer Science");
    intro_cs_variant.provider_id = Some("ror-021nxhr62".into());
    intro_cs_variant.course_code = Some("CS 101".into());
    let result = engine.match_courses(&intro_cs, &intro_cs_variant);
    println!("Match Score: {:.2}", result.score);
    println!("Is Match: {}", result.is_match);
    println!("Deterministic: {}\n", result.breakdown.deterministic_match);
}

/// Example 2: Deterministic match — shared DOI short-circuits.
fn demo_deterministic_doi(engine: &MatchingEngine) {
    println!("Example 2: Shared DOI (deterministic short-circuit)");
    let mut qm = Course::new("Quantum Mechanics I");
    let mut qm_renamed = Course::new("Completely different title");
    qm.identifiers.push(CourseIdentifier {
        scheme: IdentifierScheme::Doi,
        value: "10.1234/intro-qm".into(),
    });
    qm_renamed.identifiers.push(CourseIdentifier {
        scheme: IdentifierScheme::Doi,
        value: "10.1234/intro-qm".into(),
    });
    let result = engine.match_courses(&qm, &qm_renamed);
    println!("Match Score: {:.2}", result.score);
    println!("Deterministic: {}\n", result.breakdown.deterministic_match);
}

/// Example 3: Probabilistic name match with keyword overlap.
fn demo_fuzzy_name_keywords(engine: &MatchingEngine) {
    println!("Example 3: Fuzzy name + keyword overlap (probabilistic)");
    let mut ml_long = Course::new("Introduction to Machine Learning");
    ml_long.keywords = vec!["ml".into(), "statistics".into(), "python".into()];
    let mut ml_short = Course::new("Intro to Machine Learning");
    ml_short.keywords = vec!["ml".into(), "python".into()];
    let result = engine.match_courses(&ml_long, &ml_short);
    println!("Match Score: {:.2}", result.score);
    println!("Is Match: {}", result.is_match);
    println!("Confidence: {:?}", result.confidence);
    println!(
        "Name Score: {:.2}",
        result.breakdown.name_score.unwrap_or(0.0)
    );
    println!(
        "Keywords Score: {:.2}\n",
        result.breakdown.keywords_score.unwrap_or(0.0)
    );
}

/// Example 4: Phonetic (Soundex) bonus on near-homophone titles.
fn demo_phonetic_bonus(engine: &MatchingEngine) {
    println!("Example 4: Phonetic bonus (Smyth vs Smith)");
    let first_spelling = Course::new("Smyth");
    let second_spelling = Course::new("Smith");
    let result = engine.match_courses(&first_spelling, &second_spelling);
    println!("Name Score: {:.2} (Soundex bonus applied)\n", result.score);
}

/// Example 5: Educational-level adjacency partial credit.
fn demo_educational_level_adjacency(engine: &MatchingEngine) {
    println!("Example 5: Adjacent educational levels (partial credit)");
    let mut stats_beginner = Course::new("Statistics");
    stats_beginner.educational_level = Some(EducationalLevel::Beginner);
    let mut stats_intermediate = Course::new("Statistics");
    stats_intermediate.educational_level = Some(EducationalLevel::Intermediate);
    let result = engine.match_courses(&stats_beginner, &stats_intermediate);
    println!("Match Score: {:.2}", result.score);
    println!(
        "Educational Level Score: {:.2}\n",
        result.breakdown.educational_level_score.unwrap_or(0.0)
    );
}

/// Example 6: No match — unrelated courses.
fn demo_no_match(engine: &MatchingEngine) {
    println!("Example 6: Unrelated courses (no match)");
    let algebra = Course::new("Linear Algebra");
    let history = Course::new("Tudor History 1485-1603");
    let result = engine.match_courses(&algebra, &history);
    println!("Match Score: {:.2}", result.score);
    println!("Is Match: {}\n", result.is_match);
}

/// Example 7: Strict vs lenient thresholds on the same pair.
fn demo_strict_vs_lenient() {
    println!("Example 7: Strict vs lenient thresholds");
    let chem = Course::new("Organic Chemistry");
    let chem_lab = Course::new("Organic Chemistry Lab");
    let strict = MatchingEngine::new(MatchConfig::strict());
    let lenient = MatchingEngine::new(MatchConfig::lenient());
    let strict_result = strict.match_courses(&chem, &chem_lab);
    let lenient_result = lenient.match_courses(&chem, &chem_lab);
    println!(
        "Strict   Score: {:.2} (Match: {})",
        strict_result.score, strict_result.is_match
    );
    println!(
        "Lenient  Score: {:.2} (Match: {})\n",
        lenient_result.score, lenient_result.is_match
    );
}

/// Example 8: One-to-many ranking against a candidate set.
fn demo_rank_candidates(engine: &MatchingEngine) {
    println!("Example 8: Rank candidates against a query");
    let query = Course::new("CS101 Introduction to Computer Science");
    let candidates = vec![
        Course::new("HIS200 Tudor History"),
        Course::new("Introduction to Computer Science"),
        Course::new("CS101 Introduction to Computer Science"),
    ];
    for (idx, result) in engine.rank(&query, &candidates) {
        println!(
            "  candidate[{idx}] score {:.2} -> {}",
            result.score, candidates[idx].name
        );
    }
}
