//! Field-level validation for incoming `Organization` payloads.
//!
//! The service stores the matcher's `Organization` verbatim (JSONB), so
//! payload validation is the *service's* responsibility — the matcher is a
//! pure scoring library and deliberately performs no validation. These
//! checks return human-readable problem strings that the controller
//! surfaces as a single `422 Unprocessable Entity`.
//!
//! ## Rules
//!
//! - **`name`** — required; must not be blank.
//! - **`identifiers[i].value`** — must not be blank.
//! - **`url`, `same_as[i]`** — when present, must begin with `http://` or
//!   `https://` (ORG-T4) — the same lightweight scheme check every sibling
//!   entity crate applies.
//! - **`address.country`, `jurisdiction`** — when present, must be a
//!   currently-assigned ISO 3166-1 alpha-2 or alpha-3 country code,
//!   case-insensitively (ORG-T4). Existence in-list is checked; per-country
//!   sub-national detail is out of scope.
//! - **Input-size caps (SEC-M1)** — every scalar text field, array
//!   cardinality, and per-entry string length is bounded, so a single huge
//!   string or huge array cannot be used as a CPU/memory `DoS` against the
//!   matcher's O(n·m) Jaro-Winkler / Levenshtein / Jaccard scoring
//!   (amplified across the `check-duplicates` scan). Oversized input is
//!   rejected with a `422` *before* the record is stored or matched.

use organization_matcher::{IdentifierScheme, Organization};

/// Maximum length, in Unicode scalar values (`.chars().count()`), of any
/// single scalar text field (`name`, `legal_name`, `url`, `jurisdiction`,
/// `founding_date`, `telephone`, `email`, and each address sub-field).
/// Bounds the per-field cost of the matcher's character-level comparisons.
const MAX_TEXT_LEN: usize = 1024;

/// Maximum number of entries in any array field (`alternate_names`,
/// `identifiers`, `same_as`, `keywords`). Bounds the O(n·m) Jaccard /
/// overlap work the matcher does over arrays.
const MAX_ARRAY_LEN: usize = 256;

/// Maximum length, in Unicode scalar values (`.chars().count()`), of any
/// single string entry inside an array field.
const MAX_ITEM_LEN: usize = 512;

/// Collect every validation problem for `org`. An empty vector means the
/// payload is valid.
///
/// The controller joins these into one `422` response, so the operator
/// sees all problems at once rather than fixing them one round-trip at a
/// time.
#[must_use]
pub fn problems(org: &Organization) -> Vec<String> {
    let mut out = Vec::new();

    if org.name.trim().is_empty() {
        out.push("name is required".to_string());
    }

    // Input-size caps (SEC-M1): scalar text fields.
    check_text(&mut out, "name", &org.name);
    check_opt_text(&mut out, "legal_name", org.legal_name.as_ref());
    check_opt_text(&mut out, "url", org.url.as_ref());
    check_opt_text(&mut out, "jurisdiction", org.jurisdiction.as_ref());
    check_opt_text(&mut out, "founding_date", org.founding_date.as_ref());
    check_opt_text(&mut out, "telephone", org.telephone.as_ref());
    check_opt_text(&mut out, "email", org.email.as_ref());
    if let Some(addr) = &org.address {
        check_opt_text(
            &mut out,
            "address.street_address",
            addr.street_address.as_ref(),
        );
        check_opt_text(&mut out, "address.locality", addr.locality.as_ref());
        check_opt_text(&mut out, "address.region", addr.region.as_ref());
        check_opt_text(&mut out, "address.postal_code", addr.postal_code.as_ref());
        check_opt_text(&mut out, "address.country", addr.country.as_ref());
        if let Some(country) = &addr.country
            && !is_valid_country_code(country)
        {
            out.push(format!(
                "address.country: {country:?} is not a recognised ISO 3166-1 country code"
            ));
        }
    }

    // ORG-T4: URL well-formedness + ISO 3166 country-code format checks.
    if let Some(url) = &org.url
        && !is_valid_url(url)
    {
        out.push(format!("url: {url:?} is not a valid http(s) URL"));
    }
    if let Some(jurisdiction) = &org.jurisdiction
        && !is_valid_country_code(jurisdiction)
    {
        out.push(format!(
            "jurisdiction: {jurisdiction:?} is not a recognised ISO 3166-1 country code"
        ));
    }

    // Input-size caps (SEC-M1): array cardinality.
    check_array(&mut out, "alternate_names", org.alternate_names.len());
    check_array(&mut out, "identifiers", org.identifiers.len());
    check_array(&mut out, "same_as", org.same_as.len());
    check_array(&mut out, "keywords", org.keywords.len());

    // Per-entry blank checks (existing intent) + size caps (SEC-M1) +
    // check-digit/format validation of the deterministic schemes (SEC-M5).
    for (i, ident) in inspected(&org.identifiers) {
        if ident.value.trim().is_empty() {
            out.push(format!("identifiers[{i}]: value must not be blank"));
        } else if let Some(msg) = identifier_problem(&ident.scheme, &ident.value) {
            out.push(format!("identifiers[{i}]: {msg}"));
        }
        check_item(&mut out, "identifiers", i, &ident.value);
    }
    for (i, alt) in inspected(&org.alternate_names) {
        check_item(&mut out, "alternate_names", i, alt);
    }
    for (i, url) in inspected(&org.same_as) {
        check_item(&mut out, "same_as", i, url);
        if !is_valid_url(url) {
            out.push(format!("same_as[{i}]: {url:?} is not a valid http(s) URL"));
        }
    }
    for (i, keyword) in inspected(&org.keywords) {
        check_item(&mut out, "keywords", i, keyword);
    }

    out
}

/// The entries a per-entry check actually inspects: at most
/// [`MAX_ARRAY_LEN`] of them, paired with their index.
///
/// An array over the cardinality cap is **already rejected** by its own
/// problem, so inspecting the tail decides nothing — while reporting a
/// problem for every entry turns a small request into a large response
/// (ten thousand blank entries produced ten thousand strings, all joined
/// into one `422` body). Bounding the *report* is the same
/// input-bounding rule as bounding the work (SEC-M1/SEC-M8).
///
/// Named rather than inlined as `.take(MAX_ARRAY_LEN)` at each call site
/// so a per-entry loop added later without it reads as different from the
/// ones that have it.
fn inspected<T>(items: &[T]) -> impl Iterator<Item = (usize, &T)> {
    items.iter().take(MAX_ARRAY_LEN).enumerate()
}

/// Push a problem when a scalar text `field` exceeds [`MAX_TEXT_LEN`]
/// Unicode scalar values.
fn check_text(out: &mut Vec<String>, field: &str, value: &str) {
    if value.chars().count() > MAX_TEXT_LEN {
        out.push(format!("{field}: exceeds {MAX_TEXT_LEN} characters"));
    }
}

/// [`check_text`] for an optional field, a no-op when absent.
fn check_opt_text(out: &mut Vec<String>, field: &str, value: Option<&String>) {
    if let Some(v) = value {
        check_text(out, field, v);
    }
}

/// Push a problem when an array `field` holds more than [`MAX_ARRAY_LEN`]
/// entries.
fn check_array(out: &mut Vec<String>, field: &str, len: usize) {
    if len > MAX_ARRAY_LEN {
        out.push(format!("{field}: exceeds {MAX_ARRAY_LEN} entries"));
    }
}

/// Push a problem when the `index`-th entry of an array `field` exceeds
/// [`MAX_ITEM_LEN`] Unicode scalar values.
fn check_item(out: &mut Vec<String>, field: &str, index: usize, value: &str) {
    if value.chars().count() > MAX_ITEM_LEN {
        out.push(format!(
            "{field}[{index}]: exceeds {MAX_ITEM_LEN} characters"
        ));
    }
}

/// SEC-M5 — validate an identifier `value` against its `scheme`'s
/// check-digit / length rules, returning a problem message when malformed.
///
/// Only the **deterministic** schemes that drive the matcher's
/// short-circuit-to-`1.0` are validated (LEI, DUNS, GLN, VAT): a malformed
/// value in one of these could otherwise be stored and produce a false
/// deterministic match. Every other scheme is unconstrained here (`None`).
#[must_use]
fn identifier_problem(scheme: &IdentifierScheme, value: &str) -> Option<String> {
    match scheme {
        IdentifierScheme::Lei => {
            (!is_valid_lei(value.trim())).then(|| "invalid LEI (expect 20 alphanumeric chars with a valid ISO 7064 MOD 97-10 check)".to_string())
        }
        IdentifierScheme::Duns => (!is_valid_duns(value))
            .then(|| "invalid DUNS (expect 9 digits)".to_string()),
        IdentifierScheme::Gln => (!is_valid_gln(value))
            .then(|| "invalid GLN (expect 13 digits with a valid GS1 check digit)".to_string()),
        IdentifierScheme::Vat => (!is_valid_vat(value))
            .then(|| "invalid VAT number (expect a 2-letter country prefix followed by 2–13 alphanumerics)".to_string()),
        _ => None,
    }
}

/// Keep only the ASCII digits of `s` (drops spaces, hyphens, dots).
fn digits_only(s: &str) -> String {
    s.chars().filter(char::is_ascii_digit).collect()
}

/// LEI (ISO 17442): 20 chars, `[A-Z0-9]`, with a valid ISO 7064 MOD 97-10
/// check (the final 2 characters). Case-insensitive on input.
#[must_use]
fn is_valid_lei(s: &str) -> bool {
    let s = s.to_ascii_uppercase();
    s.len() == 20
        && s.bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
        && iso7064_mod97_10_ok(&s)
}

/// ISO 7064 MOD 97-10 checksum: letters map `A`=10 … `Z`=35; the whole
/// string, read as a base-10 number, must be `≡ 1 (mod 97)`. Computed
/// digit-by-digit so no big integer is needed.
#[must_use]
fn iso7064_mod97_10_ok(s: &str) -> bool {
    let mut rem: u32 = 0;
    for c in s.chars() {
        if c.is_ascii_digit() {
            rem = (rem * 10 + (c as u32 - '0' as u32)) % 97;
        } else if c.is_ascii_uppercase() {
            rem = (rem * 100 + (c as u32 - 'A' as u32 + 10)) % 97;
        } else {
            return false;
        }
    }
    rem == 1
}

/// DUNS: exactly 9 digits (hyphens / spaces tolerated on input). D&B
/// publishes no standard public check digit, so this is a length/charset
/// check only.
#[must_use]
fn is_valid_duns(s: &str) -> bool {
    let d = digits_only(s);
    d.len() == 9 && d.len() == s.trim().chars().filter(|c| !matches!(c, ' ' | '-')).count()
}

/// GLN (GS1): 13 digits with a valid GS1 mod-10 check digit (hyphens /
/// spaces tolerated on input).
#[must_use]
fn is_valid_gln(s: &str) -> bool {
    let d = digits_only(s);
    if d.len() != 13 || d.len() != s.trim().chars().filter(|c| !matches!(c, ' ' | '-')).count() {
        return false;
    }
    let bytes: Vec<u8> = d.bytes().map(|b| b - b'0').collect();
    gs1_check_digit(&bytes[..12]) == bytes[12]
}

/// The GS1 mod-10 check digit for the first 12 digits of a 13-digit code:
/// weight the digits `3,1,3,1,…` from the right, sum, and take
/// `(10 - sum mod 10) mod 10`.
#[must_use]
fn gs1_check_digit(first12: &[u8]) -> u8 {
    let mut sum: u32 = 0;
    for (i, d) in first12.iter().rev().enumerate() {
        let weight = if i % 2 == 0 { 3 } else { 1 };
        sum += weight * u32::from(*d);
    }
    u8::try_from((10 - (sum % 10)) % 10).unwrap_or(0)
}

/// VAT: a 2-letter ISO country prefix followed by 2–13 alphanumerics
/// (spaces tolerated). Per-country check-digit validation is deferred —
/// each member state has its own algorithm — so this is a format gate that
/// rejects obvious garbage while accepting well-formed numbers.
#[must_use]
fn is_valid_vat(s: &str) -> bool {
    let compact: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = compact.as_bytes();
    if bytes.len() < 4 || bytes.len() > 15 {
        return false;
    }
    bytes[0].is_ascii_alphabetic()
        && bytes[1].is_ascii_alphabetic()
        && bytes[2..].iter().all(u8::is_ascii_alphanumeric)
}

/// ORG-T4: true when `url` begins with `http://` or `https://` — the same
/// lightweight scheme check every sibling entity crate applies (e.g.
/// `thing-service`'s `is_http_url`, `care-pathway-service`'s
/// `is_valid_url`). No further URL structure (host, path) is checked.
#[must_use]
fn is_valid_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

/// ORG-T4: true when `code` (any surrounding whitespace trimmed) is a
/// currently-assigned ISO 3166-1 alpha-2 or alpha-3 country code,
/// case-insensitively. Used to gate `address.country` and `jurisdiction`.
///
/// This checks *format against the published registry*, not any
/// deeper geopolitical judgement: entries reflect the current edition of
/// ISO 3166-1 (249 currently-assigned codes) exactly, so a commonly-seen
/// but never-officially-assigned code (e.g. `XK` for Kosovo, which has no
/// ISO 3166-1 entry) is rejected — this is the standard's own scope, not
/// a political stance taken here.
#[must_use]
fn is_valid_country_code(code: &str) -> bool {
    let upper = code.trim().to_ascii_uppercase();
    ISO_3166_1
        .iter()
        .any(|&(alpha2, alpha3)| upper == alpha2 || upper == alpha3)
}

/// Currently-assigned ISO 3166-1 `(alpha-2, alpha-3)` country code pairs
/// (249 entries), in alpha-2 alphabetical order. See
/// [`is_valid_country_code`].
#[rustfmt::skip]
const ISO_3166_1: &[(&str, &str)] = &[
    ("AD", "AND"), ("AE", "ARE"), ("AF", "AFG"), ("AG", "ATG"), ("AI", "AIA"),
    ("AL", "ALB"), ("AM", "ARM"), ("AO", "AGO"), ("AQ", "ATA"), ("AR", "ARG"),
    ("AS", "ASM"), ("AT", "AUT"), ("AU", "AUS"), ("AW", "ABW"), ("AX", "ALA"),
    ("AZ", "AZE"),
    ("BA", "BIH"), ("BB", "BRB"), ("BD", "BGD"), ("BE", "BEL"), ("BF", "BFA"),
    ("BG", "BGR"), ("BH", "BHR"), ("BI", "BDI"), ("BJ", "BEN"), ("BL", "BLM"),
    ("BM", "BMU"), ("BN", "BRN"), ("BO", "BOL"), ("BQ", "BES"), ("BR", "BRA"),
    ("BS", "BHS"), ("BT", "BTN"), ("BV", "BVT"), ("BW", "BWA"), ("BY", "BLR"),
    ("BZ", "BLZ"),
    ("CA", "CAN"), ("CC", "CCK"), ("CD", "COD"), ("CF", "CAF"), ("CG", "COG"),
    ("CH", "CHE"), ("CI", "CIV"), ("CK", "COK"), ("CL", "CHL"), ("CM", "CMR"),
    ("CN", "CHN"), ("CO", "COL"), ("CR", "CRI"), ("CU", "CUB"), ("CV", "CPV"),
    ("CW", "CUW"), ("CX", "CXR"), ("CY", "CYP"), ("CZ", "CZE"),
    ("DE", "DEU"), ("DJ", "DJI"), ("DK", "DNK"), ("DM", "DMA"), ("DO", "DOM"),
    ("DZ", "DZA"),
    ("EC", "ECU"), ("EE", "EST"), ("EG", "EGY"), ("EH", "ESH"), ("ER", "ERI"),
    ("ES", "ESP"), ("ET", "ETH"),
    ("FI", "FIN"), ("FJ", "FJI"), ("FK", "FLK"), ("FM", "FSM"), ("FO", "FRO"),
    ("FR", "FRA"),
    ("GA", "GAB"), ("GB", "GBR"), ("GD", "GRD"), ("GE", "GEO"), ("GF", "GUF"),
    ("GG", "GGY"), ("GH", "GHA"), ("GI", "GIB"), ("GL", "GRL"), ("GM", "GMB"),
    ("GN", "GIN"), ("GP", "GLP"), ("GQ", "GNQ"), ("GR", "GRC"), ("GS", "SGS"),
    ("GT", "GTM"), ("GU", "GUM"), ("GW", "GNB"), ("GY", "GUY"),
    ("HK", "HKG"), ("HM", "HMD"), ("HN", "HND"), ("HR", "HRV"), ("HT", "HTI"),
    ("HU", "HUN"),
    ("ID", "IDN"), ("IE", "IRL"), ("IL", "ISR"), ("IM", "IMN"), ("IN", "IND"),
    ("IO", "IOT"), ("IQ", "IRQ"), ("IR", "IRN"), ("IS", "ISL"), ("IT", "ITA"),
    ("JE", "JEY"), ("JM", "JAM"), ("JO", "JOR"), ("JP", "JPN"),
    ("KE", "KEN"), ("KG", "KGZ"), ("KH", "KHM"), ("KI", "KIR"), ("KM", "COM"),
    ("KN", "KNA"), ("KP", "PRK"), ("KR", "KOR"), ("KW", "KWT"), ("KY", "CYM"),
    ("KZ", "KAZ"),
    ("LA", "LAO"), ("LB", "LBN"), ("LC", "LCA"), ("LI", "LIE"), ("LK", "LKA"),
    ("LR", "LBR"), ("LS", "LSO"), ("LT", "LTU"), ("LU", "LUX"), ("LV", "LVA"),
    ("LY", "LBY"),
    ("MA", "MAR"), ("MC", "MCO"), ("MD", "MDA"), ("ME", "MNE"), ("MF", "MAF"),
    ("MG", "MDG"), ("MH", "MHL"), ("MK", "MKD"), ("ML", "MLI"), ("MM", "MMR"),
    ("MN", "MNG"), ("MO", "MAC"), ("MP", "MNP"), ("MQ", "MTQ"), ("MR", "MRT"),
    ("MS", "MSR"), ("MT", "MLT"), ("MU", "MUS"), ("MV", "MDV"), ("MW", "MWI"),
    ("MX", "MEX"), ("MY", "MYS"), ("MZ", "MOZ"),
    ("NA", "NAM"), ("NC", "NCL"), ("NE", "NER"), ("NF", "NFK"), ("NG", "NGA"),
    ("NI", "NIC"), ("NL", "NLD"), ("NO", "NOR"), ("NP", "NPL"), ("NR", "NRU"),
    ("NU", "NIU"), ("NZ", "NZL"),
    ("OM", "OMN"),
    ("PA", "PAN"), ("PE", "PER"), ("PF", "PYF"), ("PG", "PNG"), ("PH", "PHL"),
    ("PK", "PAK"), ("PL", "POL"), ("PM", "SPM"), ("PN", "PCN"), ("PR", "PRI"),
    ("PS", "PSE"), ("PT", "PRT"), ("PW", "PLW"), ("PY", "PRY"),
    ("QA", "QAT"),
    ("RE", "REU"), ("RO", "ROU"), ("RS", "SRB"), ("RU", "RUS"), ("RW", "RWA"),
    ("SA", "SAU"), ("SB", "SLB"), ("SC", "SYC"), ("SD", "SDN"), ("SE", "SWE"),
    ("SG", "SGP"), ("SH", "SHN"), ("SI", "SVN"), ("SJ", "SJM"), ("SK", "SVK"),
    ("SL", "SLE"), ("SM", "SMR"), ("SN", "SEN"), ("SO", "SOM"), ("SR", "SUR"),
    ("SS", "SSD"), ("ST", "STP"), ("SV", "SLV"), ("SX", "SXM"), ("SY", "SYR"),
    ("SZ", "SWZ"),
    ("TC", "TCA"), ("TD", "TCD"), ("TF", "ATF"), ("TG", "TGO"), ("TH", "THA"),
    ("TJ", "TJK"), ("TK", "TKL"), ("TL", "TLS"), ("TM", "TKM"), ("TN", "TUN"),
    ("TO", "TON"), ("TR", "TUR"), ("TT", "TTO"), ("TV", "TUV"), ("TW", "TWN"),
    ("TZ", "TZA"),
    ("UA", "UKR"), ("UG", "UGA"), ("UM", "UMI"), ("US", "USA"), ("UY", "URY"),
    ("UZ", "UZB"),
    ("VA", "VAT"), ("VC", "VCT"), ("VE", "VEN"), ("VG", "VGB"), ("VI", "VIR"),
    ("VN", "VNM"), ("VU", "VUT"),
    ("WF", "WLF"), ("WS", "WSM"),
    ("YE", "YEM"), ("YT", "MYT"),
    ("ZA", "ZAF"), ("ZM", "ZMB"), ("ZW", "ZWE"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use organization_matcher::{IdentifierScheme, OrgIdentifier, PostalAddress};

    /// Build a `Wikidata`-scheme `OrgIdentifier` with the given value.
    fn ident(value: &str) -> OrgIdentifier {
        OrgIdentifier {
            scheme: IdentifierScheme::Wikidata,
            value: value.to_string(),
        }
    }

    /// A fully populated, well-formed organization yields zero problems.
    #[test]
    fn valid_organization_has_no_problems() {
        let org = Organization {
            legal_name: Some("Acme, Inc.".into()),
            alternate_names: vec!["Acme".into()],
            identifiers: vec![ident("Q4547858")],
            keywords: vec!["manufacturing".into()],
            ..Organization::new("Acme Corporation")
        };
        assert!(problems(&org).is_empty());
    }

    /// A whitespace-only name is the single `"name is required"` problem.
    #[test]
    fn blank_name_is_a_problem() {
        assert_eq!(
            problems(&Organization::new("   ")),
            vec!["name is required".to_string()]
        );
    }

    /// A blank identifier value is one problem, reported with its index.
    #[test]
    fn blank_identifier_value_is_a_problem() {
        let org = Organization {
            identifiers: vec![ident("   ")],
            ..Organization::new("Acme")
        };
        let p = problems(&org);
        assert_eq!(p.len(), 1);
        assert!(p[0].contains("identifiers[0]"));
    }

    /// SEC-M1: an oversized scalar text field is exactly one problem.
    #[test]
    fn oversized_text_field_is_a_problem() {
        let org = Organization::new("x".repeat(MAX_TEXT_LEN + 1));
        assert_eq!(
            problems(&org),
            vec![format!("name: exceeds {MAX_TEXT_LEN} characters")]
        );
    }

    /// SEC-M1: an oversized optional/nested address field is one problem.
    #[test]
    fn oversized_address_field_is_a_problem() {
        let org = Organization {
            address: Some(PostalAddress {
                locality: Some("x".repeat(MAX_TEXT_LEN + 1)),
                ..PostalAddress::default()
            }),
            ..Organization::new("Acme")
        };
        assert_eq!(
            problems(&org),
            vec![format!(
                "address.locality: exceeds {MAX_TEXT_LEN} characters"
            )]
        );
    }

    /// SEC-M1: an over-long array is exactly one problem (entries within cap).
    #[test]
    fn oversized_array_is_a_problem() {
        let org = Organization {
            keywords: vec!["ok".to_string(); MAX_ARRAY_LEN + 1],
            ..Organization::new("Acme")
        };
        assert_eq!(
            problems(&org),
            vec![format!("keywords: exceeds {MAX_ARRAY_LEN} entries")]
        );
    }

    /// SEC-M1: an oversized single entry inside an array is one problem,
    /// reported with its index.
    #[test]
    fn oversized_array_item_is_a_problem() {
        let org = Organization {
            keywords: vec!["ok".into(), "x".repeat(MAX_ITEM_LEN + 1)],
            ..Organization::new("Acme")
        };
        assert_eq!(
            problems(&org),
            vec![format!("keywords[1]: exceeds {MAX_ITEM_LEN} characters")]
        );
    }

    /// SEC-M1: a large-but-within-caps record (every field at exactly its
    /// limit) is accepted — the caps reject only what exceeds them.
    #[test]
    fn within_caps_large_record_has_no_problems() {
        let org = Organization {
            name: "x".repeat(MAX_TEXT_LEN),
            legal_name: Some("l".repeat(MAX_TEXT_LEN)),
            alternate_names: vec!["a".repeat(MAX_ITEM_LEN); MAX_ARRAY_LEN],
            identifiers: vec![ident(&"i".repeat(MAX_ITEM_LEN)); MAX_ARRAY_LEN],
            // Must still be a well-formed http(s) URL (ORG-T4) at exactly
            // the item-length cap, not just filler up to the cap.
            same_as: vec![
                format!("https://{}", "s".repeat(MAX_ITEM_LEN - "https://".len()));
                MAX_ARRAY_LEN
            ],
            keywords: vec!["k".repeat(MAX_ITEM_LEN); MAX_ARRAY_LEN],
            ..Organization::new("placeholder")
        };
        assert!(problems(&org).is_empty());
    }

    /// SEC-M5: LEI check-digit validation (ISO 7064 MOD 97-10). The
    /// synthetic all-digit values are hand-verifiable: `…098` = 98 ≡ 1
    /// (mod 97) is valid; `…097` = 97 ≡ 0 is not.
    #[test]
    fn lei_check_digit_is_validated() {
        assert!(is_valid_lei("00000000000000000098"));
        assert!(!is_valid_lei("00000000000000000097"), "97 ≡ 0 (mod 97)");
        assert!(
            !is_valid_lei("5299 0T8BM49AURSDO55"),
            "space breaks the charset"
        );
        assert!(!is_valid_lei("ABC123"), "wrong length");
    }

    /// SEC-M5: GLN GS1 mod-10 check digit. `5901234123457` is the classic
    /// valid GS1 example (check digit 7).
    #[test]
    fn gln_check_digit_is_validated() {
        assert!(is_valid_gln("5901234123457"));
        assert!(is_valid_gln("5-901234-123457"), "hyphens tolerated");
        assert!(!is_valid_gln("5901234123450"), "wrong check digit");
        assert!(!is_valid_gln("123"), "wrong length");
    }

    /// SEC-M5: DUNS is a 9-digit length/charset check (no public check digit).
    #[test]
    fn duns_length_is_validated() {
        assert!(is_valid_duns("123456789"));
        assert!(is_valid_duns("12-345-6789"), "hyphens tolerated");
        assert!(!is_valid_duns("12345"), "too short");
        assert!(!is_valid_duns("1234567890"), "too long");
        assert!(!is_valid_duns("12345678X"), "non-digit");
    }

    /// SEC-M5: VAT is a format gate (2-letter country prefix + 2–13 alnum).
    #[test]
    fn vat_format_is_validated() {
        assert!(is_valid_vat("DE123456789"));
        assert!(is_valid_vat("GB 999 9973 12"), "spaces tolerated");
        assert!(!is_valid_vat("1234567"), "no country prefix");
        assert!(!is_valid_vat("D1"), "too short");
    }

    /// ORG-T4: the registry has exactly the currently-assigned ISO 3166-1
    /// count (249 entries) — a wrong count would mean a transcription
    /// error (a dropped or duplicated row) rather than a real code.
    #[test]
    fn iso_3166_1_table_has_the_expected_entry_count() {
        assert_eq!(ISO_3166_1.len(), 249);
    }

    /// ORG-T4: alpha-2 and alpha-3 codes are both accepted,
    /// case-insensitively, with surrounding whitespace trimmed; an
    /// unrecognised code (including the commonly-seen but never
    /// officially-assigned `XK` for Kosovo) is rejected.
    #[test]
    fn country_codes_are_checked_against_iso_3166_1() {
        for v in ["US", "us", " US ", "USA", "usa", "GB", "GBR", "gb"] {
            assert!(is_valid_country_code(v), "should accept {v:?}");
        }
        for v in ["", "ZZ", "XK", "USAA", "U", "12"] {
            assert!(!is_valid_country_code(v), "should reject {v:?}");
        }
    }

    /// ORG-T4: `url` must begin with `http://` or `https://`.
    #[test]
    fn url_scheme_is_validated() {
        for v in ["https://acme.example", "http://acme.example/about"] {
            assert!(is_valid_url(v), "should accept {v:?}");
        }
        for v in ["", "acme.example", "ftp://acme.example", "not a url"] {
            assert!(!is_valid_url(v), "should reject {v:?}");
        }
    }

    /// ORG-T4: a malformed `url` is one problem; a malformed
    /// `address.country` and a malformed `jurisdiction` are each their own
    /// problem, alongside a `same_as[i]` non-URL entry.
    #[test]
    fn malformed_url_and_country_fields_are_reported() {
        let org = Organization {
            url: Some("not-a-url".into()),
            jurisdiction: Some("ZZ".into()),
            same_as: vec!["also-not-a-url".into()],
            address: Some(PostalAddress {
                country: Some("ZZ".into()),
                ..PostalAddress::default()
            }),
            ..Organization::new("Acme")
        };
        let p = problems(&org);
        assert_eq!(p.len(), 4, "problems: {p:?}");
        assert!(p.iter().any(|m| m.starts_with("url:")));
        assert!(p.iter().any(|m| m.starts_with("jurisdiction:")));
        assert!(p.iter().any(|m| m.starts_with("same_as[0]:")));
        assert!(p.iter().any(|m| m.starts_with("address.country:")));
    }

    /// ORG-T4: well-formed `url`/`same_as`/`jurisdiction`/`address.country`
    /// produce no problems (the existing-valid-records-unaffected half of
    /// the acceptance criteria).
    #[test]
    fn well_formed_url_and_country_fields_have_no_problems() {
        let org = Organization {
            url: Some("https://acme.example".into()),
            jurisdiction: Some("GB".into()),
            same_as: vec!["https://www.wikidata.org/wiki/Q4547858".into()],
            address: Some(PostalAddress {
                country: Some("gb".into()), // case-insensitive
                ..PostalAddress::default()
            }),
            ..Organization::new("Acme")
        };
        assert!(problems(&org).is_empty(), "problems: {:?}", problems(&org));
    }

    /// SEC-M5: a malformed deterministic identifier surfaces as one indexed
    /// problem; a well-formed one is accepted; a non-deterministic scheme is
    /// unconstrained.
    #[test]
    fn bad_deterministic_identifier_is_a_problem() {
        let bad = Organization {
            identifiers: vec![OrgIdentifier {
                scheme: IdentifierScheme::Gln,
                value: "5901234123450".into(), // bad GS1 check digit
            }],
            ..Organization::new("Acme")
        };
        let p = problems(&bad);
        assert_eq!(p.len(), 1);
        assert!(p[0].contains("identifiers[0]") && p[0].contains("GLN"));

        let good = Organization {
            identifiers: vec![OrgIdentifier {
                scheme: IdentifierScheme::Gln,
                value: "5901234123457".into(),
            }],
            ..Organization::new("Acme")
        };
        assert!(problems(&good).is_empty());

        // A non-deterministic scheme (Wikidata) is not check-digit validated.
        let other = Organization {
            identifiers: vec![ident("anything-goes")],
            ..Organization::new("Acme")
        };
        assert!(problems(&other).is_empty());
    }

    /// The report-everything contract: multiple problems are collected in
    /// one pass.
    #[test]
    fn problems_reports_every_issue_with_index() {
        let org = Organization {
            identifiers: vec![ident("ok"), ident(" ")], // [1] blank
            ..Organization::new("")                     // blank name
        };
        let p = problems(&org);
        assert_eq!(p.len(), 2);
        assert!(p.iter().any(|m| m.contains("name is required")));
        assert!(p.iter().any(|m| m.contains("identifiers[1]")));
    }
    /// SEC-M8: the *report* is bounded, not just the work. A caller
    /// sending ten thousand blank entries used to get ten thousand
    /// problem strings back — a small request buying a large response,
    /// and here each one also ran a check-digit validation. The
    /// cardinality problem alone is enough to reject the payload.
    #[test]
    fn an_oversized_array_of_blanks_does_not_produce_a_problem_per_entry() {
        let mut org = Organization::new("Acme Health Services");
        org.keywords = vec![String::new(); 10_000];
        org.identifiers = vec![ident(""); 10_000];
        let out = problems(&org);
        assert!(
            out.len() <= 2 * (MAX_ARRAY_LEN + 1),
            "expected at most the two cardinality problems plus one per \
             inspected entry, got {}",
            out.len()
        );
        for field in ["keywords", "identifiers"] {
            assert!(
                out.contains(&format!("{field}: exceeds {MAX_ARRAY_LEN} entries")),
                "the {field} cardinality problem must still be reported"
            );
        }
    }
}
