#![warn(clippy::pedantic)]

//! Example showing custom matching configuration.
//!
//! Walks the same pair of places through several tunings of
//! [`MatchConfig`] to show how `coordinates_scale_metres` and
//! `category_weight` change the verdict.

use place_matcher::{MatchConfig, MatchingEngine, Place, PlaceCategory, SimilarityAlgorithm};

fn main() {
    println!("=== Custom Configuration Example ===\n");

    // A 70-metre offset between two candidates — close enough to be
    // plausibly the same place, far enough that a tight scale will
    // penalise it noticeably.
    let p1 = Place::builder()
        .name("Cafe Centrale")
        .latitude_as_decimal_degrees(48.858_222)
        .longitude_as_decimal_degrees(2.294_500)
        .category(PlaceCategory::Cafe)
        .build();
    let p2 = Place::builder()
        .name("Cafe Central")
        .latitude_as_decimal_degrees(48.858_85) // ~70m north of p1
        .longitude_as_decimal_degrees(2.294_500)
        .category(PlaceCategory::Cafe)
        .build();

    println!("Place 1: Cafe Centrale @ (48.858222, 2.294500)");
    println!("Place 2: Cafe Central  @ (48.858850, 2.294500)");
    println!();

    // Default preset.
    let default = MatchingEngine::default_config();
    let r = default.match_places(&p1, &p2);
    println!("Default config (scale=50m):");
    println!("  score: {:.3}  is_match: {}", r.score, r.is_match);
    println!("  coordinates_score: {:?}", r.breakdown.coordinates_score);
    println!();

    // Tighter coordinate scale: places more than ~50m apart fall off fast.
    let tight = MatchConfig {
        coordinates_scale_metres: 10.0,
        ..MatchConfig::default()
    };
    let r = MatchingEngine::new(tight).match_places(&p1, &p2);
    println!("Tighter scale (10m):");
    println!("  score: {:.3}  is_match: {}", r.score, r.is_match);
    println!("  coordinates_score: {:?}", r.breakdown.coordinates_score);
    println!();

    // Wider scale: same pair clears the threshold comfortably.
    let wide = MatchConfig {
        coordinates_scale_metres: 200.0,
        ..MatchConfig::default()
    };
    let r = MatchingEngine::new(wide).match_places(&p1, &p2);
    println!("Wider scale (200m):");
    println!("  score: {:.3}  is_match: {}", r.score, r.is_match);
    println!("  coordinates_score: {:?}", r.breakdown.coordinates_score);
    println!();

    // Category disagreement is more punitive when its weight is raised.
    let p3 = Place::builder()
        .name("Cafe Centrale")
        .latitude_as_decimal_degrees(48.858_222)
        .longitude_as_decimal_degrees(2.294_500)
        .category(PlaceCategory::Hotel) // disagrees with p1's Cafe
        .build();

    let baseline = MatchingEngine::default_config().match_places(&p1, &p3);
    let upweighted = MatchingEngine::new(MatchConfig {
        category_weight: 0.40,
        ..MatchConfig::default()
    })
    .match_places(&p1, &p3);

    println!("Same name + coords, category disagrees:");
    println!(
        "  default category_weight (0.10): {:.3}  is_match: {}",
        baseline.score, baseline.is_match
    );
    println!(
        "  raised category_weight (0.40):  {:.3}  is_match: {}",
        upweighted.score, upweighted.is_match
    );
    println!();

    // Strict and lenient presets on a borderline pair.
    let strict = MatchingEngine::new(MatchConfig::strict());
    let lenient = MatchingEngine::new(MatchConfig::lenient());
    let rs = strict.match_places(&p1, &p2);
    let rl = lenient.match_places(&p1, &p2);
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
    let r = MatchingEngine::new(exact).match_places(&p1, &p2);
    println!(
        "Exact name algorithm: name_score={:?}  total={:.3}",
        r.breakdown.name_score, r.score
    );
}
