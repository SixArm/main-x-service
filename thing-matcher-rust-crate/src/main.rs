// Always start with high quality coding conventions.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]

// When we build for MUSL static, use faster memory allocator.
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use thing_matcher::{Identifier, MatchConfig, MatchingEngine, Thing};

fn main() {
    println!("Thing matcher");
    println!("================\n");

    let engine = MatchingEngine::default_config();

    // Example 1: identical clone — perfect match.
    println!("Example 1: Perfect Match (identical clone)");
    let eiffel = Thing::builder()
        .name("Eiffel Tower")
        .url("https://www.toureiffel.paris/")
        .add_additional_type("https://schema.org/Landmark")
        .build();
    let r1 = engine.match_things(&eiffel, &eiffel.clone());
    println!("Score: {:.2}   is_match: {}", r1.score, r1.is_match);

    // Example 2: alternate name + shared canonical URL.
    println!("\nExample 2: Aliases + Shared URL");
    let eiffel_fr = Thing::builder()
        .name("La Tour Eiffel")
        .add_alternate_name("Tour Eiffel")
        .url("https://www.toureiffel.paris/")
        .add_additional_type("https://schema.org/Landmark")
        .build();
    let r2 = engine.match_things(&eiffel, &eiffel_fr);
    println!("Score: {:.2}   is_match: {}", r2.score, r2.is_match);
    println!("Name: {:?}", r2.breakdown.name_score);
    println!("URL:  {:?}", r2.breakdown.url_score);

    // Example 3: deterministic match via a shared Wikidata identifier.
    println!("\nExample 3: Deterministic Match via Identifier");
    let id = Identifier::new("wikidata", "Q243").unwrap();
    let a = Thing::builder()
        .name("Eiffel Tower")
        .add_identifier(id.clone())
        .build();
    let b = Thing::builder()
        .name("Wholly Different Name")
        .add_identifier(id)
        .build();
    println!("Deterministic: {}", engine.deterministic_match(&a, &b));

    // Example 4: deterministic match via shared sameAs.
    println!("\nExample 4: Deterministic Match via sameAs");
    let same_as = "https://www.wikidata.org/wiki/Q243";
    let no10_a = Thing::builder()
        .name("Eiffel Tower")
        .add_same_as(same_as)
        .build();
    let no10_b = Thing::builder()
        .name("Tour Eiffel")
        .add_same_as(same_as)
        .build();
    println!(
        "Deterministic: {}",
        engine.deterministic_match(&no10_a, &no10_b),
    );

    // Example 5: same name, different identifiers — identifiers veto.
    println!("\nExample 5: Same Name, Different Identifiers");
    let same_name_x = Thing::builder()
        .name("The Grand")
        .add_identifier(Identifier::new("wikidata", "Q1").unwrap())
        .build();
    let same_name_y = Thing::builder()
        .name("The Grand")
        .add_identifier(Identifier::new("wikidata", "Q2").unwrap())
        .build();
    let r5 = engine.match_things(&same_name_x, &same_name_y);
    println!("Score: {:.2}   is_match: {}", r5.score, r5.is_match);
    println!("Identifiers: {:?}", r5.breakdown.identifiers_score);

    // Example 6: strict vs lenient on a fuzzy pair.
    println!("\nExample 6: Strict vs Lenient");
    let strict = MatchingEngine::new(MatchConfig::strict());
    let lenient = MatchingEngine::new(MatchConfig::lenient());
    let p1 = Thing::builder().name("Big Ben").build();
    let p2 = Thing::builder()
        .name("Elizabeth Tower")
        .add_alternate_name("Big Ben")
        .build();
    let rs = strict.match_things(&p1, &p2);
    let rl = lenient.match_things(&p1, &p2);
    println!("Strict:  {:.2} (match: {})", rs.score, rs.is_match);
    println!("Lenient: {:.2} (match: {})", rl.score, rl.is_match);
}
