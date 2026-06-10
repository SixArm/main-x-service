#![warn(clippy::pedantic)]

//! Basic usage example for thing matcher.

use thing_matcher::{Identifier, MatchingEngine, Thing};

fn main() {
    println!("=== Basic Thing matcher Example ===\n");

    // Two records describing the same landmark in two languages, both
    // carrying the same canonical URL and the same Wikidata identifier.
    let p1 = Thing::builder()
        .name("Eiffel Tower")
        .url("https://www.toureiffel.paris/")
        .add_identifier(Identifier::new("wikidata", "Q243").unwrap())
        .add_additional_type("https://schema.org/Landmark")
        .build();

    let p2 = Thing::builder()
        .name("La Tour Eiffel")
        .add_alternate_name("Tour Eiffel")
        .url("https://www.toureiffel.paris/")
        .add_identifier(Identifier::new("wikidata", "Q243").unwrap())
        .add_additional_type("https://schema.org/Landmark")
        .build();

    let engine = MatchingEngine::default_config();
    let result = engine.match_things(&p1, &p2);

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
        println!("Name:                       {:.0}%", s * 100.0);
    }
    if let Some(s) = result.breakdown.description_score {
        println!("Description:                {:.0}%", s * 100.0);
    }
    if let Some(s) = result.breakdown.identifiers_score {
        println!("Identifiers:                {:.0}%", s * 100.0);
    }
    if let Some(s) = result.breakdown.url_score {
        println!("URL:                        {:.0}%", s * 100.0);
    }
    if let Some(s) = result.breakdown.same_as_score {
        println!("sameAs:                     {:.0}%", s * 100.0);
    }
    if let Some(s) = result.breakdown.additional_types_score {
        println!("additionalType:             {:.0}%", s * 100.0);
    }
    println!();

    // Big Ben / Elizabeth Tower — primary canonical name on one side,
    // alternate-name match on the other. The best-of cartesian product
    // picks the matching pair.
    println!("=== Aliases (Big Ben / Elizabeth Tower) ===");
    let b1 = Thing::builder().name("Big Ben").build();
    let b2 = Thing::builder()
        .name("Elizabeth Tower")
        .add_alternate_name("Big Ben")
        .build();
    let r = engine.match_things(&b1, &b2);
    println!("Score: {:.2}   is_match: {}", r.score, r.is_match);

    // Same name, different identifiers — identifiers veto the match.
    println!("\n=== Same Name, Different Identifiers ===");
    let s1 = Thing::builder()
        .name("Starbucks")
        .add_identifier(Identifier::new("wikidata", "Q1").unwrap())
        .build();
    let s2 = Thing::builder()
        .name("Starbucks")
        .add_identifier(Identifier::new("wikidata", "Q2").unwrap())
        .build();
    let r = engine.match_things(&s1, &s2);
    println!("Score: {:.2}   is_match: {}", r.score, r.is_match);
    println!("Identifiers: {:?}", r.breakdown.identifiers_score);

    // Deterministic match via shared Wikidata identifier.
    println!("\n=== Deterministic Match via Identifier ===");
    let id = Identifier::new("wikidata", "Q243").unwrap();
    let d1 = Thing::builder()
        .name("Eiffel Tower")
        .add_identifier(id.clone())
        .build();
    let d2 = Thing::builder()
        .name("Anything Else")
        .add_identifier(id)
        .build();
    println!("Deterministic: {}", engine.deterministic_match(&d1, &d2));

    // Deterministic match via shared sameAs URL.
    println!("\n=== Deterministic Match via sameAs ===");
    let same_as = "https://www.wikidata.org/wiki/Q243";
    let pm1 = Thing::builder()
        .name("Eiffel Tower")
        .add_same_as(same_as)
        .build();
    let pm2 = Thing::builder()
        .name("Tour Eiffel")
        .add_same_as(same_as)
        .build();
    println!("Deterministic: {}", engine.deterministic_match(&pm1, &pm2));

    // JSON serialisation.
    println!("\n=== JSON Export ===");
    let json = serde_json::to_string_pretty(&result).unwrap();
    println!("{json}");
}
