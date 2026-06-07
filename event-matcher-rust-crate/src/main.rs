#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use event_matcher::{
    Address, Event, EventCategory, EventId, EventIdScheme, Location, MatchConfig, MatchingEngine,
};

fn main() {
    println!("Event matcher");
    println!("================\n");

    let engine = MatchingEngine::default_config();

    // Example 1: identical clone — perfect match.
    println!("Example 1: Perfect Match (identical clone)");
    let glasto = Event::builder()
        .name("Glastonbury Festival 2024")
        .start_date("2024-06-26T09:00:00Z")
        .end_date("2024-06-30T23:59:00Z")
        .category(EventCategory::Festival)
        .country_code_as_iso_3166_1_alpha_2("GB")
        .build();
    let r1 = engine.match_events(&glasto, &glasto.clone());
    println!("Score: {:.2}   is_match: {}", r1.score, r1.is_match);

    // Example 2: alternate name + slight time shift.
    println!("\nExample 2: Aliases + Fuzzy Start Time");
    let glasto_alias = Event::builder()
        .name("Glasto 2024")
        .add_alternate_name("Glastonbury Festival 2024")
        .start_date("2024-06-26T09:15:00Z")
        .end_date("2024-06-30T23:59:00Z")
        .category(EventCategory::Festival)
        .country_code_as_iso_3166_1_alpha_2("GB")
        .build();
    let r2 = engine.match_events(&glasto, &glasto_alias);
    println!("Score: {:.2}   is_match: {}", r2.score, r2.is_match);
    println!("Name:        {:?}", r2.breakdown.name_score);
    println!("Start date:  {:?}", r2.breakdown.start_date_score);
    println!("Country:     {:?}", r2.breakdown.country_code_score);

    // Example 3: deterministic match via a shared Eventbrite ID.
    println!("\nExample 3: Deterministic Match via Event ID");
    let id = EventId::new(EventIdScheme::Eventbrite, "123456789").unwrap();
    let a = Event::builder()
        .name("Glastonbury Festival 2024")
        .add_event_id(id.clone())
        .build();
    let b = Event::builder()
        .name("Wholly Different Name")
        .add_event_id(id)
        .build();
    println!("Deterministic: {}", engine.deterministic_match(&a, &b));

    // Example 4: deterministic match via identical name + start date.
    println!("\nExample 4: Deterministic Match via Name + Start Date");
    let conf_a = Event::builder()
        .name("RustConf 2024")
        .start_date("2024-09-10T09:00:00Z")
        .build();
    let conf_b = Event::builder()
        .name("RustConf 2024")
        .start_date("2024-09-10T11:00:00+02:00") // same instant, different offset
        .build();
    println!(
        "Deterministic: {}",
        engine.deterministic_match(&conf_a, &conf_b),
    );

    // Example 5: category disagreement lowers the score.
    println!("\nExample 5: Category Mismatch");
    let venue = Location::new()
        .with_venue_name("Madison Square Garden")
        .with_address(Address::new().with_city("New York").with_postcode("10001"))
        .with_latitude(40.750_5)
        .with_longitude(-73.993_4);
    let same_name_music = Event::builder()
        .name("The Grand")
        .start_date("2024-12-01T20:00:00Z")
        .location(venue.clone())
        .category(EventCategory::MusicEvent)
        .country_code_as_iso_3166_1_alpha_2("US")
        .build();
    let same_name_comedy = Event::builder()
        .name("The Grand")
        .start_date("2024-12-01T20:00:00Z")
        .location(venue)
        .category(EventCategory::ComedyEvent)
        .country_code_as_iso_3166_1_alpha_2("US")
        .build();
    let r5 = engine.match_events(&same_name_music, &same_name_comedy);
    println!("Score: {:.2}   is_match: {}", r5.score, r5.is_match);
    println!("Category:    {:?}", r5.breakdown.category_score);

    // Example 6: lenient preset on a partial fuzzy match.
    println!("\nExample 6: Strict vs Lenient");
    let strict = MatchingEngine::new(MatchConfig::strict());
    let lenient = MatchingEngine::new(MatchConfig::lenient());
    let e1 = Event::builder()
        .name("Big Ben Anniversary Concert")
        .start_date("2024-05-31T19:00:00Z")
        .build();
    let e2 = Event::builder()
        .name("Elizabeth Tower Concert")
        .add_alternate_name("Big Ben Anniversary Concert")
        .start_date("2024-05-31T19:05:00Z")
        .build();
    let rs = strict.match_events(&e1, &e2);
    let rl = lenient.match_events(&e1, &e2);
    println!("Strict:  {:.2} (match: {})", rs.score, rs.is_match);
    println!("Lenient: {:.2} (match: {})", rl.score, rl.is_match);
}
