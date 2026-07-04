#![warn(clippy::pedantic)]

//! Example focused on the two most subtle behaviours in the crate:
//! the `location` sub-score blend (spec §6.4) and strict-mode rejection
//! (spec §11.3).
//!
//! Run with: `cargo run --example location_matching`

use event_matcher::{Address, Event, EventCategory, Location, MatchConfig, MatchingEngine};

fn main() {
    location_coordinates();
    location_address();
    location_neutral_fallback();
    strict_mode_rejection();
}

/// Coordinates dominate the location blend (sub-weight 0.5) and decay with a
/// Gaussian over metres at the default 100 m scale.
fn location_coordinates() {
    println!("=== Location: coordinates sub-score (scale = 100 m) ===\n");

    // Worthy Farm, ~30 m apart.
    let near = pair(
        Location::new()
            .with_latitude(51.150_30)
            .with_longitude(-2.586_20),
        Location::new()
            .with_latitude(51.150_55)
            .with_longitude(-2.586_20),
    );
    // London vs Paris — hundreds of km apart.
    let far = pair(
        Location::new()
            .with_latitude(51.507_22)
            .with_longitude(-0.127_5),
        Location::new()
            .with_latitude(48.853_0)
            .with_longitude(2.349_2),
    );

    let engine = MatchingEngine::default_config();
    let r_near = engine.match_events(&near.0, &near.1);
    let r_far = engine.match_events(&far.0, &far.1);
    println!(
        "  ~30 m apart : location_score = {:?}",
        r_near.breakdown.location_score
    );
    println!(
        "  London/Paris: location_score = {:?}",
        r_far.breakdown.location_score
    );
    println!();
}

/// The address blend mixes postcode (0.5), city (0.3) and line 1 (0.2);
/// line 1 itself weights house-number equality at 0.4 against street
/// similarity at 0.6.
fn location_address() {
    println!("=== Location: address sub-score blend ===\n");

    let same_house = pair(
        Location::new().with_address(Address::new().with_line1("10 Main Street")),
        Location::new().with_address(Address::new().with_line1("10 Main Street")),
    );
    let diff_house = pair(
        Location::new().with_address(Address::new().with_line1("10 Main Street")),
        Location::new().with_address(Address::new().with_line1("22 Main Street")),
    );

    let engine = MatchingEngine::default_config();
    println!(
        "  same street + same house number : {:?}",
        engine
            .match_events(&same_house.0, &same_house.1)
            .breakdown
            .location_score
    );
    println!(
        "  same street + diff house number : {:?}",
        engine
            .match_events(&diff_house.0, &diff_house.1)
            .breakdown
            .location_score
    );
    println!();
}

/// Both sides carry a `Location` but share no populated sub-component: the
/// blend falls back to a neutral 0.5 rather than `None`.
fn location_neutral_fallback() {
    println!("=== Location: neutral 0.5 fallback (no shared sub-component) ===\n");

    let p = pair(
        Location::new()
            .with_latitude(51.150_3)
            .with_longitude(-2.586_2),
        Location::new().with_venue_name("Worthy Farm"),
    );
    let r = MatchingEngine::default_config().match_events(&p.0, &p.1);
    println!(
        "  coords-only vs venue-only: location_score = {:?}",
        r.breakdown.location_score
    );
    println!();
}

/// Strict mode (spec §11.3) tightens only `is_match`: it additionally
/// requires `deterministic_match`. A close-but-not-equal fuzzy pair clears
/// the default threshold yet is rejected by strict mode, while `score` and
/// `confidence` are identical between the two configurations.
fn strict_mode_rejection() {
    println!("=== Strict mode: fuzzy match without deterministic agreement ===\n");

    let a = Event::builder()
        .name("Cafe Centrale Concert")
        .start_date("2024-09-10T09:00:00Z")
        .category(EventCategory::MusicEvent)
        .build();
    let b = Event::builder()
        .name("Cafe Central Concert") // close, but not equal under normalisation
        .start_date("2024-09-10T09:00:00Z")
        .category(EventCategory::MusicEvent)
        .build();

    let default = MatchingEngine::default_config();
    let strict = MatchingEngine::new(MatchConfig::strict());
    let r_default = default.match_events(&a, &b);
    let r_strict = strict.match_events(&a, &b);

    println!(
        "  default: score {:.3}  is_match {}",
        r_default.score, r_default.is_match
    );
    println!(
        "  strict : score {:.3}  is_match {}",
        r_strict.score, r_strict.is_match
    );
    println!(
        "  deterministic_match = {} (no shared EventId; names differ under normalisation)",
        strict.deterministic_match(&a, &b)
    );
}

/// Build two minimal events that differ only in their `location`.
fn pair(l1: Location, l2: Location) -> (Event, Event) {
    (
        Event::builder().name("X").location(l1).build(),
        Event::builder().name("X").location(l2).build(),
    )
}
