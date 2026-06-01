//! Basic usage example for person matcher

use chrono::NaiveDate;
use person_matcher::{
    Address, BloodType, Gender, MatchConfig, MatchingEngine, NicknameTable, Normalizer,
    PassportBook, Person,
};

fn main() {
    println!("=== Basic Person matcher Example ===\n");

    // Create two person records with similar information
    let person1 = Person::builder()
        .united_kingdom_national_health_service_number("123 456 7890")
        .given_name("Catherine")
        .family_name("Williams")
        .date_of_birth(NaiveDate::from_ymd_opt(1985, 3, 15).unwrap())
        .gender(Gender::Female)
        .phone("07700 900123")
        .build();

    let person2 = Person::builder()
        .united_kingdom_national_health_service_number("1234567890") // Same United Kingdom National Health Service Number, different format
        .given_name("Katherine") // Different spelling
        .family_name("Williams")
        .date_of_birth(NaiveDate::from_ymd_opt(1985, 3, 15).unwrap())
        .gender(Gender::Female)
        .phone("+44 7700 900123") // Same phone, different format
        .build();

    // Create matching engine
    let engine = MatchingEngine::default_config();

    // Match the persons
    let result = engine.match_persons(&person1, &person2);

    // Display results
    println!("Overall Match Score: {:.2}%", result.score * 100.0);
    println!(
        "Is Match: {}",
        if result.is_match { "YES ✓" } else { "NO ✗" }
    );
    println!("Confidence: {:?}", result.confidence);
    println!();

    // Show detailed breakdown
    println!("Detailed Breakdown:");
    println!("------------------");

    if let Some(score) = result.breakdown.united_kingdom_national_health_service_number_score {
        println!("UK United Kingdom National Health Service Number: {:.0}%", score * 100.0);
    }

    if let Some(score) = result.breakdown.given_name_score {
        println!("Given Name:      {:.0}%", score * 100.0);
    }

    if let Some(score) = result.breakdown.family_name_score {
        println!("Family Name:       {:.0}%", score * 100.0);
    }

    if let Some(score) = result.breakdown.date_of_birth_score {
        println!("Date of Birth:   {:.0}%", score * 100.0);
    }

    if let Some(score) = result.breakdown.gender_score {
        println!("Gender:          {:.0}%", score * 100.0);
    }

    if let Some(score) = result.breakdown.phone_score {
        println!("Phone Number:    {:.0}%", score * 100.0);
    }

    if let Some(score) = result.breakdown.phonetic_name_score {
        println!("Phonetic Names:  {:.0}%", score * 100.0);
    }

    println!();

    // Demonstrate deterministic matching
    let is_deterministic = engine.deterministic_match(&person1, &person2);
    println!(
        "Deterministic Match (United Kingdom National Health Service Number exact): {}",
        is_deterministic
    );

    // Show JSON serialization
    println!("\n=== JSON Export ===");
    let json = serde_json::to_string_pretty(&result).unwrap();
    println!("{}", json);

    // International phone-number normalisation (E.164).
    println!("\n=== International Phone Numbers (E.164) ===");
    for (label, input, default_country) in [
        ("UK   +44", "+44 7700 900123", Some("GB")),
        ("UK   0044", "0044 7700 900123", Some("GB")),
        ("UK   trunk", "07700 900123", Some("GB")),
        ("FR   +33", "+33 1 23 45 67 89", None),
        ("FR   trunk", "01 23 45 67 89", Some("FR")),
        ("ES   no-trunk", "912 345 678", Some("ES")),
        ("IE   +353", "+353 1 234 5678", None),
        ("US   NANP", "(415) 555-1234", Some("US")),
        ("DE   00", "0049 30 12345678", None),
    ] {
        let canonical = Normalizer::normalize_phone_e164(input, default_country);
        println!("  {label}: {input:<20} (default={default_country:?}) -> {canonical:?}");
    }

    // International matching: a French national-format number compared
    // against its +33 international form, with the engine configured for
    // France as the default jurisdiction.
    println!("\n=== International Phone Match (FR default) ===");
    let fr_engine = MatchingEngine::new(MatchConfig {
        phone_default_country: Some("FR".into()),
        ..MatchConfig::default()
    });
    let fr_a = Person::builder()
        .given_name("Jean")
        .family_name("Dupont")
        .phone("01 23 45 67 89")
        .build();
    let fr_b = Person::builder()
        .given_name("Jean")
        .family_name("Dupont")
        .phone("+33 1 23 45 67 89")
        .build();
    let fr_result = fr_engine.match_persons(&fr_a, &fr_b);
    println!(
        "  Phone score (FR national vs +33 form): {:?}",
        fr_result.breakdown.phone_score
    );

    // DOB transposition heuristic — DD/MM ↔ MM/DD data-entry bug catch.
    println!("\n=== DOB Transposition Heuristic ===");
    let p_jan_10 = Person::builder()
        .given_name("Thomas")
        .family_name("Price")
        .date_of_birth(NaiveDate::from_ymd_opt(1995, 1, 10).unwrap())
        .build();
    let p_oct_1 = Person::builder()
        .given_name("Thomas")
        .family_name("Price")
        .date_of_birth(NaiveDate::from_ymd_opt(1995, 10, 1).unwrap())
        .build();
    let p_unrelated = Person::builder()
        .given_name("Thomas")
        .family_name("Price")
        .date_of_birth(NaiveDate::from_ymd_opt(1980, 6, 30).unwrap())
        .build();
    let transposed = engine.match_persons(&p_jan_10, &p_oct_1);
    let unrelated = engine.match_persons(&p_jan_10, &p_unrelated);
    println!(
        "  1995-01-10 vs 1995-10-01 (DD/MM swap):  dob_score = {:?}",
        transposed.breakdown.date_of_birth_score,
    );
    println!(
        "  1995-01-10 vs 1980-06-30 (unrelated):   dob_score = {:?}",
        unrelated.breakdown.date_of_birth_score,
    );

    // Email matching — exact match on canonical form, opt-in Gmail folding.
    println!("\n=== Email Matching ===");
    let default_engine = MatchingEngine::default_config();
    let gmail_engine = MatchingEngine::new(MatchConfig {
        gmail_dot_folding: true,
        ..MatchConfig::default()
    });
    for (label, e1, e2) in [
        ("case-and-ws", "Alice@Example.ORG  ", "alice@example.org"),
        (
            "dotted gmail (default off)",
            "j.smith@gmail.com",
            "jsmith@gmail.com",
        ),
        (
            "dotted gmail (folding on)",
            "j.smith@gmail.com",
            "jsmith@gmail.com",
        ),
        (
            "plus-tag gmail (folding on)",
            "jsmith+work@gmail.com",
            "jsmith@gmail.com",
        ),
        ("malformed", "not-an-email", "also@@bad"),
    ] {
        let p1 = Person::builder()
            .given_name("X")
            .family_name("Y")
            .email(e1)
            .build();
        let p2 = Person::builder()
            .given_name("X")
            .family_name("Y")
            .email(e2)
            .build();
        let engine = if label.contains("folding on") {
            &gmail_engine
        } else {
            &default_engine
        };
        let r = engine.match_persons(&p1, &p2);
        println!(
            "  {label:<32} -> email_score = {:?}",
            r.breakdown.email_score,
        );
    }

    // Nickname dictionary — opt-in lift for given-name equivalents.
    println!("\n=== Nickname Dictionary ===");
    let nick_engine = MatchingEngine::new(MatchConfig {
        nickname_table: NicknameTable::english(),
        ..MatchConfig::default()
    });
    for (a, b) in [
        ("Michael", "Mike"),
        ("Elizabeth", "Liz"),
        ("Robert", "Bob"),
        ("Mike", "Robert"), // unrelated → no boost
    ] {
        let p1 = Person::builder().given_name(a).family_name("Smith").build();
        let p2 = Person::builder().given_name(b).family_name("Smith").build();
        let r = nick_engine.match_persons(&p1, &p2);
        println!(
            "  {a:<10} vs {b:<10} -> given_name_score = {:.2}",
            r.breakdown.given_name_score.unwrap(),
        );
    }

    // United States SSN — exact match across textual layouts.
    println!("\n=== United States SSN ===");
    let ssn_a = Person::builder()
        .us_ssn("123-45-6789")
        .given_name("Alice")
        .family_name("Anderson")
        .build();
    let ssn_b = Person::builder()
        .us_ssn("123456789") // same SSN, compact form
        .given_name("Different")
        .family_name("Spelling")
        .build();
    let ssn_result = engine.match_persons(&ssn_a, &ssn_b);
    println!("  us_ssn score: {:?}", ssn_result.breakdown.us_ssn_score);
    println!(
        "  Deterministic match (SSN drives it): {}",
        engine.deterministic_match(&ssn_a, &ssn_b),
    );
    // Structurally-invalid SSNs return None on parse:
    use person_matcher::identifiers;
    println!(
        "  parse_us_ssn(\"900-00-0000\"): {:?}",
        identifiers::parse_us_ssn("900-00-0000"),
    );

    // Blood type — weak positive signal, strong negative signal.
    println!("\n=== Blood Type ===");
    for input in ["A+", "a positive", "AB neg", "0-", "0 negative", "Bombay"] {
        let parsed = BloodType::parse(input);
        println!("  parse({input:?}) -> {parsed:?}");
    }
    let p_op = Person::builder()
        .given_name("Ada")
        .family_name("Lovelace")
        .date_of_birth(NaiveDate::from_ymd_opt(1815, 12, 10).unwrap())
        .blood_type(BloodType::OPositive)
        .build();
    let p_an = Person::builder()
        .given_name("Ada")
        .family_name("Lovelace")
        .date_of_birth(NaiveDate::from_ymd_opt(1815, 12, 10).unwrap())
        .blood_type(BloodType::ANegative)
        .build();
    let r = engine.match_persons(&p_op, &p_an);
    println!(
        "  same name+DOB, O+ vs A-  ->  blood_type_score = {:?}, overall = {:.2}",
        r.breakdown.blood_type_score, r.score,
    );

    // Multiple-birth indicator — identical-twin disambiguation.
    println!("\n=== Multiple Birth (Twin Disambiguation) ===");
    let twin1 = Person::builder()
        .given_name("Alex")
        .family_name("Smith")
        .date_of_birth(NaiveDate::from_ymd_opt(2000, 6, 15).unwrap())
        .gender(Gender::Female)
        .multiple_birth(1)
        .build();
    let twin2 = Person::builder()
        .given_name("Alex")
        .family_name("Smith")
        .date_of_birth(NaiveDate::from_ymd_opt(2000, 6, 15).unwrap())
        .gender(Gender::Female)
        .multiple_birth(2)
        .build();
    let r = engine.match_persons(&twin1, &twin2);
    println!(
        "  same name+DOB+gender, twin 1 vs twin 2  ->  multiple_birth_score = {:?}, overall = {:.2}",
        r.breakdown.multiple_birth_score, r.score,
    );

    // Place of birth — FHIR Patient.birthPlace; city + country sub-score.
    println!("\n=== Place of Birth ===");
    for (label, b1, b2) in [
        (
            "same city + country",
            Address::new().with_city("Cardiff").with_country("Wales"),
            Address::new().with_city("Cardiff").with_country("Wales"),
        ),
        (
            "diacritic-tolerant",
            Address::new()
                .with_city("Zürich")
                .with_country("Switzerland"),
            Address::new()
                .with_city("Zurich")
                .with_country("Switzerland"),
        ),
        (
            "same city, different country",
            Address::new().with_city("Paris").with_country("France"),
            Address::new().with_city("Paris").with_country("USA"),
        ),
        (
            "country only",
            Address::new().with_country("Wales"),
            Address::new().with_country("Wales"),
        ),
    ] {
        let p1 = Person::builder()
            .given_name("X")
            .family_name("Y")
            .birth_place(b1)
            .build();
        let p2 = Person::builder()
            .given_name("X")
            .family_name("Y")
            .birth_place(b2)
            .build();
        let r = engine.match_persons(&p1, &p2);
        println!(
            "  {label:<35} -> birth_place_score = {:?}",
            r.breakdown.birth_place_score,
        );
    }

    // Death date and place — FHIR Patient.deceasedDateTime; transposition
    // heuristic on the date, city+country sub-score on the place.
    println!("\n=== Death Date and Place ===");
    let alan_a = Person::builder()
        .given_name("Alan")
        .family_name("Turing")
        .date_of_birth(NaiveDate::from_ymd_opt(1912, 6, 23).unwrap())
        .death_date(NaiveDate::from_ymd_opt(1954, 6, 7).unwrap())
        .death_place(Address::new().with_city("Wilmslow").with_country("England"))
        .build();
    let alan_b = Person::builder()
        .given_name("Alan")
        .family_name("Turing")
        .date_of_birth(NaiveDate::from_ymd_opt(1912, 6, 23).unwrap())
        .death_date(NaiveDate::from_ymd_opt(1954, 6, 7).unwrap())
        .death_place(Address::new().with_city("Wilmslow").with_country("England"))
        .build();
    let r = engine.match_persons(&alan_a, &alan_b);
    println!(
        "  identical death record           -> death_date = {:?}, death_place = {:?}",
        r.breakdown.death_date_score, r.breakdown.death_place_score,
    );
    let alan_c = Person::builder()
        .given_name("Alan")
        .family_name("Turing")
        .death_date(NaiveDate::from_ymd_opt(1954, 7, 6).unwrap())
        .build();
    let r = engine.match_persons(&alan_a, &alan_c);
    println!(
        "  same year, day/month swap        -> death_date = {:?}",
        r.breakdown.death_date_score,
    );

    // Passport books — multi-country, multi-book, time-varying matching.
    println!("\n=== Passport Books ===");
    let dual_citizen = Person::builder()
        .given_name("Alice")
        .family_name("Anderson")
        .add_passport_book(
            PassportBook::new("GB", "123456789")
                .unwrap()
                .with_issued(NaiveDate::from_ymd_opt(2024, 6, 1).unwrap()),
        )
        .add_passport_book(PassportBook::new("US", "AB1234567").unwrap())
        .add_passport_book(PassportBook::new("GB", "OLD-BOOK-NUM").unwrap())
        .build();
    println!(
        "  dual citizen holds {} passport books across {} countries",
        dual_citizen.passport_books.len(),
        dual_citizen
            .passport_books
            .iter()
            .map(|b| &b.country)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
    );

    // Other system holds only the historical UK book.
    let other_system = Person::builder()
        .given_name("Alice")
        .family_name("Anderson")
        .add_passport_book(PassportBook::new("gb", " old book num ").unwrap())
        .build();
    let r = engine.match_persons(&dual_citizen, &other_system);
    println!(
        "  shared historical UK book -> passport_book_score = {:?}, deterministic = {}",
        r.breakdown.passport_book_score,
        engine.deterministic_match(&dual_citizen, &other_system),
    );

    // Cross-country with same digits never matches.
    let collision = Person::builder()
        .given_name("Alice")
        .family_name("Anderson")
        .add_passport_book(PassportBook::new("FR", "123456789").unwrap())
        .build();
    let r = engine.match_persons(&dual_citizen, &collision);
    println!(
        "  same digits, different country (FR vs GB) -> passport_book_score = {:?}",
        r.breakdown.passport_book_score,
    );

    // Batch API — screen one query against many candidates.
    println!("\n=== Batch API: rank_one_to_many ===");
    let mpi_query = Person::builder()
        .given_name("Ada")
        .family_name("Lovelace")
        .build();
    let mpi_candidates = vec![
        Person::builder()
            .given_name("Grace")
            .family_name("Hopper")
            .build(),
        Person::builder()
            .given_name("Ada")
            .family_name("Lovelace")
            .build(),
        Person::builder()
            .given_name("Alan")
            .family_name("Turing")
            .build(),
        Person::builder()
            .given_name("Alyce") // typo variant
            .family_name("Lovelace")
            .build(),
    ];
    let ranked = engine.rank_one_to_many(&mpi_query, &mpi_candidates);
    for (rank, (idx, result)) in ranked.iter().enumerate() {
        let c = &mpi_candidates[*idx];
        println!(
            "  #{rank}: candidate[{idx}] {:<10} {:<10} score={:.2} match={}",
            c.given_name.as_deref().unwrap_or(""),
            c.family_name.as_deref().unwrap_or(""),
            result.score,
            result.is_match,
        );
    }

    // Six additional national identifiers — verify each parser canonicalises.
    println!("\n=== Six Additional National Identifiers ===");
    for (label, raw, canonical) in [
        ("AU IHI", "8003 6012 3456 7894", "8003601234567894"),
        ("DE KVNR", "a 123 456 780", "A123456780"),
        ("IT CF", "rss mra 85t 10a 562s", "RSSMRA85T10A562S"),
        ("NL BSN", "111-222-333", "111222333"),
        ("SE Personnummer", "460324-3850", "4603243850"),
        ("UK CHI", "010 170 1233", "0101701233"),
    ] {
        let parsed = match label {
            "AU IHI" => identifiers::parse_au_ihi(raw),
            "DE KVNR" => identifiers::parse_de_kvnr(raw),
            "IT CF" => identifiers::parse_it_cf(raw),
            "NL BSN" => identifiers::parse_nl_bsn(raw),
            "SE Personnummer" => identifiers::parse_se_personnummer(raw),
            "UK CHI" => identifiers::parse_uk_chi_number(raw),
            _ => unreachable!(),
        };
        println!("  {label:<18} {raw:<24} -> {parsed:?} (expected {canonical:?})",);
    }

    // Seven next-batch national identifiers (T-17.1).
    println!("\n=== T-17.1: Seven Next-Batch National Identifiers ===");
    for (label, raw, canonical) in [
        ("BR CPF", "123.456.789-09", "12345678909"),
        ("CN RRN", "11010519491231002x", "11010519491231002X"),
        ("IN Aadhaar", "2341 2341 2346", "234123412346"),
        ("JP My Number", "1234 5678 9018", "123456789018"),
        ("MX CURP", "hegg560427mvzrrl04", "HEGG560427MVZRRL04"),
        ("NZ NHI", "zaa0083", "ZAA0083"),
        ("ZA ID", "800101 5009 087", "8001015009087"),
    ] {
        let parsed = match label {
            "BR CPF" => identifiers::parse_br_cpf(raw),
            "CN RRN" => identifiers::parse_cn_rrn(raw),
            "IN Aadhaar" => identifiers::parse_in_aadhaar(raw),
            "JP My Number" => identifiers::parse_jp_my_number(raw),
            "MX CURP" => identifiers::parse_mx_curp(raw),
            "NZ NHI" => identifiers::parse_nz_nhi(raw),
            "ZA ID" => identifiers::parse_za_id(raw),
            _ => unreachable!(),
        };
        println!("  {label:<14} {raw:<24} -> {parsed:?} (expected {canonical:?})",);
    }

    // Sophisticated address parsing.
    println!("\n=== Address Parsing ===");
    for input in [
        "123 High Street",
        "123 High St",
        "45 N Park Ave",
        "10A Downing St",
        "Flat 2A, 10 Downing Street",
        "Apt 5, 1600 Pennsylvania Ave",
        "Buckingham Palace",
    ] {
        let parsed = Normalizer::parse_address_line(input);
        println!(
            "  {input:<40} -> number={:?}, unit={:?}, street={:?}",
            parsed.house_number, parsed.unit, parsed.street,
        );
    }
}
