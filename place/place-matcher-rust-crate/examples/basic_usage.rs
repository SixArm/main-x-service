#![warn(clippy::pedantic)]

//! Basic usage example for place matcher.

use place_matcher::{Address, MatchingEngine, Place, PlaceCategory, PlaceId, PlaceIdScheme};

fn main() {
    println!("=== Basic Place matcher Example ===\n");

    // Two records describing the same monument in two languages, with
    // slightly different coordinates.
    let p1 = Place::builder()
        .name("Eiffel Tower")
        .latitude(48.858_222)
        .longitude(2.294_500)
        .category(PlaceCategory::Monument)
        .country_code_as_iso_3166_1_alpha_2("FR")
        .build();

    let p2 = Place::builder()
        .name("La Tour Eiffel")
        .add_alternate_name("Tour Eiffel")
        .latitude(48.858_3)
        .longitude(2.294_5)
        .category(PlaceCategory::Monument)
        .country_code_as_iso_3166_1_alpha_2("FR")
        .build();

    let engine = MatchingEngine::default_config();
    let result = engine.match_places(&p1, &p2);

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
    if let Some(s) = result.breakdown.coordinates_score {
        println!("Coordinates:  {:.0}%", s * 100.0);
    }
    if let Some(s) = result.breakdown.category_score {
        println!("Category:     {:.0}%", s * 100.0);
    }
    if let Some(s) = result.breakdown.country_code_score {
        println!("Country code: {:.0}%", s * 100.0);
    }
    if let Some(s) = result.breakdown.place_ids_score {
        println!("Place IDs:    {:.0}%", s * 100.0);
    }
    if let Some(s) = result.breakdown.address_score {
        println!("Address:      {:.0}%", s * 100.0);
    }
    println!();

    // Big Ben / Elizabeth Tower — primary canonical name on one side,
    // alternate-name match on the other. The best-of cartesian product
    // picks the matching pair.
    println!("=== Aliases (Big Ben / Elizabeth Tower) ===");
    let b1 = Place::builder()
        .name("Big Ben")
        .latitude(51.500_7)
        .longitude(-0.124_7)
        .build();
    let b2 = Place::builder()
        .name("Elizabeth Tower")
        .add_alternate_name("Big Ben")
        .latitude(51.500_72)
        .longitude(-0.124_65)
        .build();
    let r = engine.match_places(&b1, &b2);
    println!("Score: {:.2}   is_match: {}", r.score, r.is_match);

    // Chain stores: same name, different cities — should NOT match.
    println!("\n=== Chain Stores: Same Name, Different Cities ===");
    let s1 = Place::builder()
        .name("Starbucks")
        .latitude(40.7589)
        .longitude(-73.9851)
        .category(PlaceCategory::Cafe)
        .country_code_as_iso_3166_1_alpha_2("US")
        .build();
    let s2 = Place::builder()
        .name("Starbucks")
        .latitude(34.0522)
        .longitude(-118.2437)
        .category(PlaceCategory::Cafe)
        .country_code_as_iso_3166_1_alpha_2("US")
        .build();
    let r = engine.match_places(&s1, &s2);
    println!("Score: {:.2}   is_match: {}", r.score, r.is_match);
    println!("Coordinates: {:?}", r.breakdown.coordinates_score);

    // Same coordinates but different names (a venue rebrand, perhaps) —
    // the geographic signal pulls the score up even though the names differ.
    println!("\n=== Same Coordinates, Different Names ===");
    let v1 = Place::builder()
        .name("Old Cafe Name")
        .latitude(48.858_222)
        .longitude(2.294_500)
        .build();
    let v2 = Place::builder()
        .name("New Cafe Name")
        .latitude(48.858_222)
        .longitude(2.294_500)
        .build();
    let r = engine.match_places(&v1, &v2);
    println!("Score: {:.2}   is_match: {}", r.score, r.is_match);

    // Deterministic match via shared Wikidata ID.
    println!("\n=== Deterministic Match via Place ID ===");
    let id = PlaceId::new(PlaceIdScheme::Wikidata, "Q243").unwrap();
    let d1 = Place::builder()
        .name("Eiffel Tower")
        .add_place_id(id.clone())
        .build();
    let d2 = Place::builder()
        .name("Anything Else")
        .add_place_id(id)
        .build();
    println!("Deterministic: {}", engine.deterministic_match(&d1, &d2));

    // Deterministic match via identical name + postcode.
    println!("\n=== Deterministic Match via Name + Postcode ===");
    let addr = Address::new()
        .with_line1("10 Downing Street")
        .with_postcode("SW1A 2AA");
    let pm1 = Place::builder()
        .name("10 Downing Street")
        .address(addr.clone())
        .build();
    let pm2 = Place::builder()
        .name("10 Downing Street")
        .address(addr)
        .build();
    println!("Deterministic: {}", engine.deterministic_match(&pm1, &pm2));

    // JSON serialisation.
    println!("\n=== JSON Export ===");
    let json = serde_json::to_string_pretty(&result).unwrap();
    println!("{json}");
}
