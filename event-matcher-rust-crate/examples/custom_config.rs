//! Example showing custom matching configuration.
//!
//! Walks the same pair of events through several tunings of
//! [`MatchConfig`] to show how `start_date_scale_seconds` and
//! `category_weight` change the verdict.

use event_matcher::{Event, EventCategory, MatchConfig, MatchingEngine, SimilarityAlgorithm};

fn main() {
    println!("=== Custom Configuration Example ===\n");

    // A 30-minute offset between two candidates — close enough to be
    // plausibly the same event, far enough that a tight scale will
    // penalise it noticeably.
    let p1 = Event::builder()
        .name("Open Mic Night")
        .start_date("2024-09-10T20:00:00Z")
        .category(EventCategory::ComedyEvent)
        .build();
    let p2 = Event::builder()
        .name("Open Mic Nite")
        .start_date("2024-09-10T20:30:00Z") // 30 minutes after p1
        .category(EventCategory::ComedyEvent)
        .build();

    println!("Event 1: Open Mic Night @ 2024-09-10T20:00:00Z");
    println!("Event 2: Open Mic Nite  @ 2024-09-10T20:30:00Z");
    println!();

    // Default preset.
    let default = MatchingEngine::default_config();
    let r = default.match_events(&p1, &p2);
    println!("Default config (scale=3600s = 1 hour):");
    println!("  score: {:.3}  is_match: {}", r.score, r.is_match);
    println!("  start_date_score: {:?}", r.breakdown.start_date_score);
    println!();

    // Tighter time scale: events more than ~10 minutes apart fall off fast.
    let tight = MatchConfig {
        start_date_scale_seconds: 600.0, // 10 minutes
        ..MatchConfig::default()
    };
    let r = MatchingEngine::new(tight).match_events(&p1, &p2);
    println!("Tighter scale (600s = 10 minutes):");
    println!("  score: {:.3}  is_match: {}", r.score, r.is_match);
    println!("  start_date_score: {:?}", r.breakdown.start_date_score);
    println!();

    // Wider scale: same pair clears the threshold comfortably.
    let wide = MatchConfig {
        start_date_scale_seconds: 86_400.0, // 1 day
        ..MatchConfig::default()
    };
    let r = MatchingEngine::new(wide).match_events(&p1, &p2);
    println!("Wider scale (86_400s = 1 day):");
    println!("  score: {:.3}  is_match: {}", r.score, r.is_match);
    println!("  start_date_score: {:?}", r.breakdown.start_date_score);
    println!();

    // Category disagreement is more punitive when its weight is raised.
    let p3 = Event::builder()
        .name("Open Mic Night")
        .start_date("2024-09-10T20:00:00Z")
        .category(EventCategory::MusicEvent) // disagrees with p1's ComedyEvent
        .build();

    let baseline = MatchingEngine::default_config().match_events(&p1, &p3);
    let upweighted = MatchingEngine::new(MatchConfig {
        category_weight: 0.40,
        ..MatchConfig::default()
    })
    .match_events(&p1, &p3);

    println!("Same name + start date, category disagrees:");
    println!(
        "  default category_weight (0.08): {:.3}  is_match: {}",
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
    let rs = strict.match_events(&p1, &p2);
    let rl = lenient.match_events(&p1, &p2);
    println!("Strict preset:  score {:.3}  is_match {}", rs.score, rs.is_match);
    println!("Lenient preset: score {:.3}  is_match {}", rl.score, rl.is_match);
    println!();

    // Exact-name algorithm: tiny name differences become a hard fail.
    let exact = MatchConfig {
        name_algorithm: SimilarityAlgorithm::Exact,
        ..MatchConfig::default()
    };
    let r = MatchingEngine::new(exact).match_events(&p1, &p2);
    println!(
        "Exact name algorithm: name_score={:?}  total={:.3}",
        r.breakdown.name_score, r.score
    );
}
