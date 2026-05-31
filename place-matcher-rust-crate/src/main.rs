use place_matcher::{
    Address, MatchConfig, MatchingEngine, Place, PlaceCategory, PlaceId, PlaceIdScheme,
};

fn main() {
    println!("Place matcher");
    println!("================\n");

    let engine = MatchingEngine::default_config();

    // Example 1: identical clone — perfect match.
    println!("Example 1: Perfect Match (identical clone)");
    let eiffel = Place::builder()
        .name("Eiffel Tower")
        .latitude(48.858_222)
        .longitude(2.294_500)
        .category(PlaceCategory::Monument)
        .country_code_as_iso_3166_1_alpha_2("FR")
        .build();
    let r1 = engine.match_places(&eiffel, &eiffel.clone());
    println!("Score: {:.2}   is_match: {}", r1.score, r1.is_match);

    // Example 2: alternate name + slightly different coords.
    println!("\nExample 2: Aliases + Fuzzy Coordinates");
    let eiffel_fr = Place::builder()
        .name("La Tour Eiffel")
        .add_alternate_name("Tour Eiffel")
        .latitude(48.858_3)
        .longitude(2.294_5)
        .category(PlaceCategory::Monument)
        .country_code_as_iso_3166_1_alpha_2("FR")
        .build();
    let r2 = engine.match_places(&eiffel, &eiffel_fr);
    println!("Score: {:.2}   is_match: {}", r2.score, r2.is_match);
    println!("Name:        {:?}", r2.breakdown.name_score);
    println!("Coordinates: {:?}", r2.breakdown.coordinates_score);
    println!("Country:     {:?}", r2.breakdown.country_code_score);

    // Example 3: deterministic match via a shared Wikidata ID.
    println!("\nExample 3: Deterministic Match via Place ID");
    let id = PlaceId::new(PlaceIdScheme::Wikidata, "Q243").unwrap();
    let a = Place::builder()
        .name("Eiffel Tower")
        .add_place_id(id.clone())
        .build();
    let b = Place::builder()
        .name("Wholly Different Name")
        .add_place_id(id)
        .build();
    println!(
        "Deterministic: {}",
        engine.deterministic_match(&a, &b),
    );

    // Example 4: deterministic match via identical name + postcode.
    println!("\nExample 4: Deterministic Match via Name + Postcode");
    let addr = Address::new()
        .with_line1("10 Downing Street")
        .with_postcode("SW1A 2AA");
    let no10_a = Place::builder()
        .name("10 Downing Street")
        .address(addr.clone())
        .build();
    let no10_b = Place::builder()
        .name("10 Downing Street")
        .address(addr)
        .build();
    println!(
        "Deterministic: {}",
        engine.deterministic_match(&no10_a, &no10_b),
    );

    // Example 5: category and country-code disagreement lowers the score.
    println!("\nExample 5: Category Mismatch");
    let same_name_hotel = Place::builder()
        .name("The Grand")
        .latitude(40.7)
        .longitude(-74.0)
        .category(PlaceCategory::Hotel)
        .country_code_as_iso_3166_1_alpha_2("US")
        .build();
    let same_name_cafe = Place::builder()
        .name("The Grand")
        .latitude(40.7)
        .longitude(-74.0)
        .category(PlaceCategory::Cafe)
        .country_code_as_iso_3166_1_alpha_2("US")
        .build();
    let r5 = engine.match_places(&same_name_hotel, &same_name_cafe);
    println!("Score: {:.2}   is_match: {}", r5.score, r5.is_match);
    println!("Category:    {:?}", r5.breakdown.category_score);

    // Example 6: lenient preset on a partial fuzzy match.
    println!("\nExample 6: Strict vs Lenient");
    let strict = MatchingEngine::new(MatchConfig::strict());
    let lenient = MatchingEngine::new(MatchConfig::lenient());
    let p1 = Place::builder()
        .name("Big Ben")
        .latitude(51.500_7)
        .longitude(-0.124_7)
        .build();
    let p2 = Place::builder()
        .name("Elizabeth Tower")
        .add_alternate_name("Big Ben")
        .latitude(51.500_72)
        .longitude(-0.124_65)
        .build();
    let rs = strict.match_places(&p1, &p2);
    let rl = lenient.match_places(&p1, &p2);
    println!("Strict:  {:.2} (match: {})", rs.score, rs.is_match);
    println!("Lenient: {:.2} (match: {})", rl.score, rl.is_match);
}
