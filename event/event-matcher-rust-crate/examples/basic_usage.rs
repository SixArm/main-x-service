#![warn(clippy::pedantic)]

//! Basic usage example for Event matcher.

use event_matcher::{Event, EventCategory, EventId, EventIdScheme, Location, MatchingEngine};

fn main() {
    println!("=== Basic Event matcher Example ===\n");

    // Two records describing the same festival in two different naming
    // conventions, with a slight start-time drift.
    let p1 = Event::builder()
        .name("Glastonbury Festival 2024")
        .start_date("2024-06-26T09:00:00Z")
        .end_date("2024-06-30T23:59:00Z")
        .category(EventCategory::Festival)
        .country_code_as_iso_3166_1_alpha_2("GB")
        .build();

    let p2 = Event::builder()
        .name("Glasto 2024")
        .add_alternate_name("Glastonbury Festival 2024")
        .start_date("2024-06-26T09:15:00Z")
        .end_date("2024-06-30T23:59:00Z")
        .category(EventCategory::Festival)
        .country_code_as_iso_3166_1_alpha_2("GB")
        .build();

    let engine = MatchingEngine::default_config();
    let result = engine.match_events(&p1, &p2);

    println!("Overall Score: {:.2}%", result.score * 100.0);
    println!(
        "Is Match:      {}",
        if result.is_match { "YES" } else { "NO" }
    );
    println!("Confidence:    {:?}", result.confidence);
    println!();

    println!("Detailed Breakdown:");
    println!("------------------");
    if let Some(s) = result.breakdown.name_score {
        println!("Name:         {:.0}%", s * 100.0);
    }
    if let Some(s) = result.breakdown.start_date_score {
        println!("Start date:   {:.0}%", s * 100.0);
    }
    if let Some(s) = result.breakdown.end_date_score {
        println!("End date:     {:.0}%", s * 100.0);
    }
    if let Some(s) = result.breakdown.category_score {
        println!("Category:     {:.0}%", s * 100.0);
    }
    if let Some(s) = result.breakdown.country_code_score {
        println!("Country code: {:.0}%", s * 100.0);
    }
    if let Some(s) = result.breakdown.event_ids_score {
        println!("Event IDs:    {:.0}%", s * 100.0);
    }
    if let Some(s) = result.breakdown.location_score {
        println!("Location:     {:.0}%", s * 100.0);
    }
    println!();

    // Big Ben Concert: primary canonical name on one side, alternate-name
    // match on the other. The best-of cartesian product picks the matching pair.
    println!("=== Aliases (Big Ben / Elizabeth Tower Concert) ===");
    let b1 = Event::builder()
        .name("Big Ben Anniversary Concert")
        .start_date("2024-05-31T19:00:00Z")
        .build();
    let b2 = Event::builder()
        .name("Elizabeth Tower Concert")
        .add_alternate_name("Big Ben Anniversary Concert")
        .start_date("2024-05-31T19:05:00Z")
        .build();
    let r = engine.match_events(&b1, &b2);
    println!("Score: {:.2}   is_match: {}", r.score, r.is_match);

    // Conference series: same conference name in different years — should NOT match.
    println!("\n=== Same Conference Name, Different Years ===");
    let s1 = Event::builder()
        .name("RustConf")
        .start_date("2024-09-10T09:00:00Z")
        .category(EventCategory::ConferenceEvent)
        .country_code_as_iso_3166_1_alpha_2("US")
        .build();
    let s2 = Event::builder()
        .name("RustConf")
        .start_date("2023-09-12T09:00:00Z")
        .category(EventCategory::ConferenceEvent)
        .country_code_as_iso_3166_1_alpha_2("US")
        .build();
    let r = engine.match_events(&s1, &s2);
    println!("Score: {:.2}   is_match: {}", r.score, r.is_match);
    println!("Start date: {:?}", r.breakdown.start_date_score);

    // Same date and venue, different names (a rebranded event) — the
    // schedule signal pulls the score up even though the names differ.
    println!("\n=== Same Date and Venue, Different Names ===");
    let venue = Location::new()
        .with_venue_name("Worthy Farm")
        .with_latitude(51.150_3)
        .with_longitude(-2.586_2);
    let v1 = Event::builder()
        .name("Old Festival Name")
        .start_date("2024-06-26T09:00:00Z")
        .location(venue.clone())
        .build();
    let v2 = Event::builder()
        .name("New Festival Name")
        .start_date("2024-06-26T09:00:00Z")
        .location(venue)
        .build();
    let r = engine.match_events(&v1, &v2);
    println!("Score: {:.2}   is_match: {}", r.score, r.is_match);

    // Deterministic match via shared Eventbrite ID.
    println!("\n=== Deterministic Match via Event ID ===");
    let id = EventId::new(EventIdScheme::Eventbrite, "123456789").unwrap();
    let d1 = Event::builder()
        .name("RustConf 2024")
        .add_event_id(id.clone())
        .build();
    let d2 = Event::builder()
        .name("Anything Else")
        .add_event_id(id)
        .build();
    println!("Deterministic: {}", engine.deterministic_match(&d1, &d2));

    // Deterministic match via identical name + start_date (even with
    // different but equivalent timezone offsets).
    println!("\n=== Deterministic Match via Name + Start Date ===");
    let pm1 = Event::builder()
        .name("RustConf 2024")
        .start_date("2024-09-10T09:00:00Z")
        .build();
    let pm2 = Event::builder()
        .name("RustConf 2024")
        .start_date("2024-09-10T11:00:00+02:00")
        .build();
    println!("Deterministic: {}", engine.deterministic_match(&pm1, &pm2));

    // JSON serialisation.
    println!("\n=== JSON Export ===");
    let json = serde_json::to_string_pretty(&result).unwrap();
    println!("{json}");
}
