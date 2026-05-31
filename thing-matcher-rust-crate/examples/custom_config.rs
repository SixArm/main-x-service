//! Example showing custom matching configuration.
//!
//! Walks the same pair of things through several tunings of
//! [`MatchConfig`] to show how the individual field weights change the
//! verdict.

use thing_matcher::{MatchConfig, MatchingEngine, SimilarityAlgorithm, Thing};

fn main() {
    println!("=== Custom Configuration Example ===\n");

    let t1 = Thing::builder()
        .name("Cafe Centrale")
        .description("A small cafe near the river.")
        .url("https://example.org/cafe-centrale")
        .build();
    let t2 = Thing::builder()
        .name("Cafe Central")
        .description("A small cafe near the river.")
        .url("https://example.org/cafe-central")
        .build();

    println!("Thing 1: Cafe Centrale  url=cafe-centrale");
    println!("Thing 2: Cafe Central   url=cafe-central");
    println!();

    // Default preset.
    let default = MatchingEngine::default_config();
    let r = default.match_things(&t1, &t2);
    println!("Default config:");
    println!("  score: {:.3}  is_match: {}", r.score, r.is_match);
    println!("  name_score:        {:?}", r.breakdown.name_score);
    println!("  description_score: {:?}", r.breakdown.description_score);
    println!("  url_score:         {:?}", r.breakdown.url_score);
    println!();

    // Raise the name weight: a tiny name typo punishes the score more.
    let name_heavy = MatchConfig {
        name_weight: 0.70,
        ..MatchConfig::default()
    };
    let r = MatchingEngine::new(name_heavy).match_things(&t1, &t2);
    println!("name_weight=0.70:");
    println!("  score: {:.3}  is_match: {}", r.score, r.is_match);
    println!();

    // Down-weight the URL: distinct URLs hurt less.
    let url_light = MatchConfig {
        url_weight: 0.0,
        ..MatchConfig::default()
    };
    let r = MatchingEngine::new(url_light).match_things(&t1, &t2);
    println!("url_weight=0.0:");
    println!("  score: {:.3}  is_match: {}", r.score, r.is_match);
    println!();

    // Strict and lenient presets on the same pair.
    let strict = MatchingEngine::new(MatchConfig::strict());
    let lenient = MatchingEngine::new(MatchConfig::lenient());
    let rs = strict.match_things(&t1, &t2);
    let rl = lenient.match_things(&t1, &t2);
    println!(
        "Strict preset:  score {:.3}  is_match {}",
        rs.score, rs.is_match
    );
    println!(
        "Lenient preset: score {:.3}  is_match {}",
        rl.score, rl.is_match
    );
    println!();

    // Exact-name algorithm: tiny name differences become a hard fail.
    let exact = MatchConfig {
        name_algorithm: SimilarityAlgorithm::Exact,
        ..MatchConfig::default()
    };
    let r = MatchingEngine::new(exact).match_things(&t1, &t2);
    println!(
        "Exact name algorithm: name_score={:?}  total={:.3}",
        r.breakdown.name_score, r.score
    );
}
