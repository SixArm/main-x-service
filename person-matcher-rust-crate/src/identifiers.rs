//! National healthcare identifier parsing and validation.
//!
//! This module exposes parsers for the national-level healthcare identifiers
//! that the crate compares deterministically and probabilistically.
//!
//! Function names follow the convention `parse_<cc>_<scheme>`, where `<cc>`
//! is the ISO 3166-1 alpha-2 country code (lower-cased) and `<scheme>` is
//! the short identifier name. This keeps related schemes alphabetised within
//! a country, and makes new countries easy to slot in.
//!
//! | Jurisdiction | Identifier | Parser |
//! |---|---|---|
//! | United Kingdom — England, Wales, Isle of Man | United Kingdom National Health Service Number | [`parse_united_kingdom_national_health_service_number`] |
//! | France | NIR (*Numéro d'Inscription au Répertoire*) | [`parse_fr_nir`] |
//! | España (Spain) | TSI (*Tarjeta Sanitaria Individual*) / CIP-SNS | [`parse_es_tsi`] |
//! | Éire (Ireland) | IHI (Individual Health Identifier) | [`parse_ie_ihi`] |
//! | United Kingdom — Northern Ireland | H&C Number (Health and Care Number) | [`parse_uk_hc_number`] |
//! | United Kingdom — Scotland | CHI (Community Health Index) | [`parse_uk_chi_number`] |
//! | United Kingdom | NINO (National Insurance Number) | [`parse_uk_nino`] |
//! | United States | SSN (Social Security Number) | [`parse_us_ssn`] |
//! | Germany | KVNR (Krankenversichertennummer) | [`parse_de_kvnr`] |
//! | Italy | *Codice Fiscale* (CF) | [`parse_it_cf`] |
//! | Netherlands | BSN (*Burgerservicenummer*) | [`parse_nl_bsn`] |
//! | Sweden | *Personnummer* | [`parse_se_personnummer`] |
//! | Australia | IHI (Individual Healthcare Identifier) | [`parse_au_ihi`] |
//! | Belgium | National Number (*Rijksregisternummer*) | [`parse_be_nn`] |
//! | Bulgaria | EGN (*Edinen grazhdanski nomer*) | [`parse_bg_egn`] |
//! | Czech Republic | *Rodné číslo* | [`parse_cz_rc`] |
//! | Denmark | CPR (*Centrale Personregister*) | [`parse_dk_cpr`] |
//! | Estonia | *Isikukood* | [`parse_ee_ik`] |
//! | España (Spain) | DNI/NIE | [`parse_es_dni`] |
//! | Finland | HETU (*Henkilötunnus*) | [`parse_fi_hetu`] |
//! | Croatia | OIB (*Osobni identifikacijski broj*) | [`parse_hr_oib`] |
//! | Iceland | *Kennitala* | [`parse_is_kt`] |
//! | Lithuania | *Asmens kodas* | [`parse_lt_ak`] |
//! | Latvia | *Personas kods* | [`parse_lv_pk`] |
//! | Malta | National ID | [`parse_mt_id`] |
//! | Norway | *Fødselsnummer* | [`parse_no_fnr`] |
//! | Poland | PESEL | [`parse_pl_pesel`] |
//! | Romania | CNP (*Cod Numeric Personal*) | [`parse_ro_cnp`] |
//! | Slovenia | EMŠO (*Enotna Matična Številka Občana*) | [`parse_si_emso`] |
//! | Slovakia | *Rodné číslo* | [`parse_sk_rc`] |
//! | Greece | DSS investor share | [`parse_gr_dss`] |
//! | Liechtenstein | National Identity Card Number | [`parse_li_id`] |
//! | Netherlands | National Identity Card Number | [`parse_nl_id`] |
//! | Poland | NIP (*Numer Identyfikacji Podatkowej*) | [`parse_pl_nip`] |
//! | Portugal | NIF (*Número de Identificação Fiscal*) | [`parse_pt_nif`] |
//! | Brazil | CPF (*Cadastro de Pessoas Físicas*) | [`parse_br_cpf`] |
//! | China | RRN (*居民身份证*) 18-digit | [`parse_cn_rrn`] |
//! | India | Aadhaar | [`parse_in_aadhaar`] |
//! | Japan | My Number (*個人番号*) | [`parse_jp_my_number`] |
//! | Mexico | CURP (*Clave Única de Registro de Población*) | [`parse_mx_curp`] |
//! | New Zealand | NHI (National Health Index) — original 7-char form | [`parse_nz_nhi`] |
//! | South Africa | ID Number | [`parse_za_id`] |
//!
//! ## Passport-number format validators
//!
//! Passport book numbers are not stable across renewals, and a person
//! may hold passports from several countries simultaneously — see
//! [`crate::PassportBook`] for the canonical multi-country, multi-book,
//! time-varying model used by the matcher. The following per-country
//! parsers are pure **format validators** that consumers can call before
//! constructing a `PassportBook` (or as a smell test in their own
//! ingestion code). They do NOT have a corresponding `Person` field;
//! they exist so a country-specific passport number can be canonicalised
//! and rejected at the system boundary.
//!
//! | Jurisdiction | Format | Parser |
//! |---|---|---|
//! | Cyprus | `E` + 6 digits (pre-2010) or `K` + 8 digits | [`parse_cy_passport`] |
//! | Czech Republic | 8 to 12 digits | [`parse_cz_passport`] |
//! | Liechtenstein | 1 letter + 5 digits | [`parse_li_passport`] |
//! | Lithuania | 8 digits | [`parse_lt_passport`] |
//! | Malta | 7 digits | [`parse_mt_passport`] |
//! | Netherlands | same shape as the NL ID card | [`parse_nl_passport`] |
//! | Portugal | 1 letter + 6 digits | [`parse_pt_passport`] |
//! | Romania | 2 letters + 6 digits | [`parse_ro_passport`] |
//! | Slovakia | 2 letters + 7 digits | [`parse_sk_passport`] |
//!
//! Each parser takes a `&str` and returns `Option<String>`:
//!
//! - `Some(canonical)` — the input parses for the identifier scheme. The
//!   returned string is a canonical form (whitespace stripped, letters
//!   uppercased) suitable for byte-equality comparison.
//! - `None` — the input fails the scheme's structural or check-digit test.
//!
//! Two inputs that represent the same identifier in different textual
//! layouts always canonicalise to the same string. Consumers compare the
//! canonical forms for equality; the matching engine does exactly this.
//!
//! ## Design notes
//!
//! - Parsing is **format-only** unless the scheme has an integral check
//!   digit (NIR has a Modulus-97 key; H&C / United Kingdom National Health
//!   Service structurally accept any 10-digit number through the
//!   `united-kingdom-national-health-service-number` crate's `FromStr`).
//! - These parsers do not consult external registries; they verify only
//!   what can be derived from the identifier's own structure.
//! - Country-specific semantic ranges (e.g. valid French department codes,
//!   valid Spanish autonomous-community prefixes) are deliberately NOT
//!   enforced to avoid rejecting edge-case-but-legitimate values.
//!
//! ## Example
//!
//! ```
//! use person_matcher::identifiers;
//!
//! // UK United Kingdom National Health Service Number — accepts the
//! // canonical "XXX XXX XXXX" layout.
//! assert_eq!(
//!     identifiers::parse_united_kingdom_national_health_service_number("943 476 5919"),
//!     Some("9434765919".to_string()),
//! );
//!
//! // Anything that does not match the United Kingdom National Health Service
//! // Number layout returns None.
//! assert_eq!(identifiers::parse_united_kingdom_national_health_service_number("not-a-number"), None);
//! ```

use united_kingdom_national_health_service_number::NHSNumber as UnitedKingdomNationalHealthServiceNumber;
use std::str::FromStr;

/// Parse a United Kingdom National Health Service Number (England, Wales,
/// Isle of Man).
///
/// Wraps [`united_kingdom_national_health_service_number::NHSNumber::from_str`],
/// which accepts the 10-digit compact layout (`"9434765919"`) and the spaced
/// layout (`"943 476 5919"`). On success, the canonical 10-digit form is
/// returned.
///
/// The United Kingdom National Health Service Number applies to England,
/// Wales, and the Isle of Man. Northern Ireland uses a separate H&C Number
/// that follows the same Modulus-11 algorithm — see [`parse_uk_hc_number`].
///
/// # Examples
///
/// ```
/// use person_matcher::identifiers::parse_united_kingdom_national_health_service_number;
///
/// assert_eq!(parse_united_kingdom_national_health_service_number("9434765919"),   Some("9434765919".to_string()));
/// assert_eq!(parse_united_kingdom_national_health_service_number("943 476 5919"), Some("9434765919".to_string()));
/// assert_eq!(parse_united_kingdom_national_health_service_number("ABCDEFGHIJ"),   None);
/// assert_eq!(parse_united_kingdom_national_health_service_number("123"),          None);
/// ```
pub fn parse_united_kingdom_national_health_service_number(s: &str) -> Option<String> {
    let parsed = UnitedKingdomNationalHealthServiceNumber::from_str(s).ok()?;
    let mut canonical = String::with_capacity(10);
    for &d in &parsed.digits {
        canonical.push(char::from_digit(d as u32, 10)?);
    }
    Some(canonical)
}

/// Parse a France NIR (*Numéro d'Inscription au Répertoire*).
///
/// The NIR — also known as the INSEE number or *Numéro de Sécurité Sociale*
/// — is France's national social-security identifier and the de-facto unique
/// healthcare identifier. Its structure is:
///
/// ```text
/// S YY MM DD CCC NNN KK
/// │ │  │  │  │   │   └─ 2-digit check key (Mod-97)
/// │ │  │  │  │   └───── 3-digit municipal birth-order number
/// │ │  │  │  └───────── 3-digit commune code
/// │ │  │  └──────────── 2-digit département (or "2A"/"2B" for Corsica)
/// │ │  └─────────────── 2-digit month of birth
/// │ └────────────────── 2-digit year of birth
/// └──────────────────── sex (1=male, 2=female, plus special values)
/// ```
///
/// Total length is exactly 15 characters. The check key K satisfies
/// `K = 97 - (N mod 97)`, where N is the 13-digit body. For Corsica, the
/// department letters are remapped before computing N: `"2A" → "19"`,
/// `"2B" → "18"`.
///
/// Whitespace in the input is stripped before parsing, so the formal layout
/// `"1 80 12 75 123 456 42"` and the compact `"180127512345642"` both
/// canonicalise to the same 15-character upper-case string.
///
/// # Examples
///
/// A canonical, syntactically valid NIR round-trips:
///
/// ```
/// use person_matcher::identifiers::parse_fr_nir;
///
/// // 13-digit body with department 75 (Paris), key computed as 97 - (N mod 97).
/// let valid = "180127512345642";
/// assert_eq!(parse_fr_nir(valid), Some(valid.to_string()));
/// ```
///
/// Whitespace is tolerated:
///
/// ```
/// # use person_matcher::identifiers::parse_fr_nir;
/// assert_eq!(
///     parse_fr_nir("1 80 12 75 123 456 42"),
///     Some("180127512345642".to_string()),
/// );
/// ```
///
/// An invalid check key rejects:
///
/// ```
/// # use person_matcher::identifiers::parse_fr_nir;
/// assert_eq!(parse_fr_nir("180127512345699"), None);  // wrong key
/// assert_eq!(parse_fr_nir("12345"),           None);  // wrong length
/// assert_eq!(parse_fr_nir(""),                None);
/// ```
pub fn parse_fr_nir(s: &str) -> Option<String> {
    let cleaned: String = s
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_uppercase();

    if !cleaned.is_ascii() || cleaned.len() != 15 {
        return None;
    }

    let dept = &cleaned[5..7];
    let numeric_body = match dept {
        "2A" => format!("{}19{}", &cleaned[0..5], &cleaned[7..13]),
        "2B" => format!("{}18{}", &cleaned[0..5], &cleaned[7..13]),
        _ => cleaned[0..13].to_string(),
    };

    if !numeric_body.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let key_str = &cleaned[13..15];
    if !key_str.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    let n: u64 = numeric_body.parse().ok()?;
    let key: u64 = key_str.parse().ok()?;

    if 97 - (n % 97) == key {
        Some(cleaned)
    } else {
        None
    }
}

/// Parse a España (Spain) TSI (*Tarjeta Sanitaria Individual*) / CIP-SNS identifier.
///
/// Spain's healthcare identification is fragmented across 17 autonomous
/// communities, each of which issues its own TSI card with a region-specific
/// format. The national-level *Código de Identificación Personal del Sistema
/// Nacional de Salud* (CIP-SNS) provides a uniform 16-character code with
/// the canonical structure `LLLLDDDDDDXXXXXX` (4 letters + 6 digits + 6
/// alphanumerics), but regional formats are also encountered in practice.
///
/// To accept the full population of legitimate identifiers without
/// privileging any region, this parser is **format-only** and lenient:
///
/// 1. Whitespace and ASCII hyphens are stripped.
/// 2. Letters are uppercased.
/// 3. The remaining string must contain only ASCII alphanumerics.
/// 4. The length must be in `10..=20`.
///
/// No check-digit calculation is performed because the schemes vary by
/// community. A consumer that needs stronger validation should layer a
/// community-specific check on top of this canonical form.
///
/// # Examples
///
/// ```
/// use person_matcher::identifiers::parse_es_tsi;
///
/// // 16-character CIP-SNS national code:
/// assert_eq!(
///     parse_es_tsi("ABCD123456XY1234"),
///     Some("ABCD123456XY1234".to_string()),
/// );
///
/// // Whitespace and hyphens are stripped, letters uppercased:
/// assert_eq!(
///     parse_es_tsi("abcd 123 456-xy1234"),
///     Some("ABCD123456XY1234".to_string()),
/// );
///
/// // Too short, too long, or containing non-alphanumerics rejects:
/// assert_eq!(parse_es_tsi("ABC123"),                 None);  // 6 chars
/// assert_eq!(parse_es_tsi("ABCDEF123456XY12345678"), None);  // 22 chars
/// assert_eq!(parse_es_tsi("ABC@123!XYZ"),            None);  // bad chars
/// ```
pub fn parse_es_tsi(s: &str) -> Option<String> {
    let cleaned: String = s
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect::<String>()
        .to_uppercase();

    if !cleaned.is_ascii() {
        return None;
    }
    if !cleaned.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    if !(10..=20).contains(&cleaned.len()) {
        return None;
    }
    Some(cleaned)
}

/// Parse an Éire (Ireland) IHI (Individual Health Identifier).
///
/// Under the Health Identifiers Act 2014, every individual receiving
/// healthcare in Ireland is assigned a 7-digit IHI by the Health Identifiers
/// Service. The IHI is the unique national healthcare identifier in the
/// Republic of Ireland.
///
/// Parsing rules:
///
/// 1. All non-digit characters are stripped (spaces, hyphens, etc.).
/// 2. The remaining string must contain exactly 7 ASCII digits.
///
/// No check-digit algorithm is enforced (none is publicly specified). The
/// canonical form is the 7-digit string.
///
/// # Examples
///
/// ```
/// use person_matcher::identifiers::parse_ie_ihi;
///
/// assert_eq!(parse_ie_ihi("1234567"),    Some("1234567".to_string()));
/// assert_eq!(parse_ie_ihi("123 4567"),   Some("1234567".to_string()));
/// assert_eq!(parse_ie_ihi("123-45-67"),  Some("1234567".to_string()));
///
/// assert_eq!(parse_ie_ihi("12345"),      None);   // too short
/// assert_eq!(parse_ie_ihi("12345678"),   None);   // too long
/// assert_eq!(parse_ie_ihi("ABCDEFG"),    None);   // not digits
/// ```
pub fn parse_ie_ihi(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() == 7 {
        Some(digits)
    } else {
        None
    }
}

/// Parse a United Kingdom Northern Ireland H&C (Health and Care) Number.
///
/// The H&C Number is Northern Ireland's national healthcare identifier,
/// issued by HSC (Health and Social Care). Structurally it is a 10-digit
/// number with a Modulus-11 check digit — the same algorithm used by the
/// UK United Kingdom National Health Service Number.
///
/// This parser delegates to the same logic as
/// [`parse_united_kingdom_national_health_service_number`]: it accepts
/// either the compact 10-digit form or the spaced `"XXX XXX XXXX"` form and
/// returns the canonical 10-digit string.
///
/// The two parsers are intentionally separate so that callers track *which*
/// scheme an identifier belongs to: a number that parses successfully as
/// both a United Kingdom National Health Service Number and an H&C Number
/// still refers to two distinct people in two distinct registries.
///
/// # Examples
///
/// ```
/// use person_matcher::identifiers::parse_uk_hc_number;
///
/// assert_eq!(parse_uk_hc_number("9434765919"),   Some("9434765919".to_string()));
/// assert_eq!(parse_uk_hc_number("943 476 5919"), Some("9434765919".to_string()));
/// assert_eq!(parse_uk_hc_number("not-a-number"), None);
/// ```
pub fn parse_uk_hc_number(s: &str) -> Option<String> {
    parse_united_kingdom_national_health_service_number(s)
}

/// Parse a United States Social Security Number (SSN).
///
/// The SSN is the United States' de-facto national identifier — a 9-digit
/// number assigned by the Social Security Administration, conventionally
/// formatted as `"AAA-GG-SSSS"`:
///
/// ```text
/// AAA  - GG - SSSS
/// │      │    └──── Serial Number (4 digits, 0001..=9999)
/// │      └───────── Group Number  (2 digits, 01..=99)
/// └──────────────── Area Number   (3 digits, 001..=665, 667..=899)
/// ```
///
/// Parsing rules:
///
/// 1. Keep only ASCII digits (strip whitespace, hyphens, periods,
///    parentheses, …).
/// 2. Reject unless the result has exactly 9 digits.
/// 3. Reject structurally-impossible area numbers (`000`, `666`, and
///    `900..=999`). These have never been assigned by SSA.
/// 4. Reject group `00`.
/// 5. Reject serial `0000`.
///
/// Since SSA introduced randomised assignment in June 2011, the area
/// number no longer encodes the state of issuance, so no geographic
/// validation is attempted. The canonical form is the 9-digit compact
/// string `"AAAGGSSSS"`.
///
/// # Examples
///
/// Three textual layouts of the same SSN canonicalise identically:
///
/// ```
/// use person_matcher::identifiers::parse_us_ssn;
///
/// assert_eq!(parse_us_ssn("123-45-6789"), Some("123456789".to_string()));
/// assert_eq!(parse_us_ssn("123 45 6789"), Some("123456789".to_string()));
/// assert_eq!(parse_us_ssn("123456789"),   Some("123456789".to_string()));
/// ```
///
/// Structurally-invalid values are rejected:
///
/// ```
/// # use person_matcher::identifiers::parse_us_ssn;
/// assert_eq!(parse_us_ssn("000-12-3456"), None); // area 000 never issued
/// assert_eq!(parse_us_ssn("666-12-3456"), None); // area 666 never issued
/// assert_eq!(parse_us_ssn("900-12-3456"), None); // area 900..=999 never issued
/// assert_eq!(parse_us_ssn("123-00-4567"), None); // group 00 invalid
/// assert_eq!(parse_us_ssn("123-45-0000"), None); // serial 0000 invalid
/// assert_eq!(parse_us_ssn("12345"),       None); // too short
/// assert_eq!(parse_us_ssn("ABCDEFGHI"),   None); // not digits
/// assert_eq!(parse_us_ssn(""),            None);
/// ```
pub fn parse_us_ssn(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 9 {
        return None;
    }
    let area: u32 = digits[0..3].parse().ok()?;
    let group: u32 = digits[3..5].parse().ok()?;
    let serial: u32 = digits[5..9].parse().ok()?;
    if area == 0 || area == 666 || area >= 900 {
        return None;
    }
    if group == 0 {
        return None;
    }
    if serial == 0 {
        return None;
    }
    Some(digits)
}

/// Parse a Germany KVNR (*Krankenversichertennummer*).
///
/// The KVNR is the lifelong health-insurance number printed on the
/// German electronic health card (*elektronische Gesundheitskarte*).
/// Structure: 10 characters total, one uppercase letter followed by 9
/// digits. The final digit is a Mod-10 check digit.
///
/// Check-digit algorithm:
///
/// 1. Map the leading letter to a two-digit ordinal (`A=01`, `B=02`,
///    …, `Z=26`).
/// 2. Concatenate that 2-digit value with positions 2..=9 of the KVNR
///    (the 8 digits before the check digit) → a 10-digit string.
/// 3. Multiply each of those 10 digits by alternating weights
///    `1, 2, 1, 2, 1, 2, 1, 2, 1, 2`.
/// 4. For products `≥ 10`, replace with the digit sum (max product is
///    `9 × 2 = 18`, so subtract 9 to digit-sum).
/// 5. Sum all results; the check digit is `sum mod 10`.
///
/// Whitespace is stripped before parsing. The canonical form is the
/// 10-character uppercase string.
///
/// # Examples
///
/// ```
/// use person_matcher::identifiers::parse_de_kvnr;
///
/// // Constructed valid KVNR (A=01; alternating Mod-10 yields check digit 0).
/// assert_eq!(parse_de_kvnr("A123456780"), Some("A123456780".to_string()));
/// assert_eq!(parse_de_kvnr("a123456780"), Some("A123456780".to_string()));  // lowercase letter accepted
///
/// // Wrong check digit:
/// assert_eq!(parse_de_kvnr("A123456789"), None);
///
/// // Wrong length / shape:
/// assert_eq!(parse_de_kvnr("1234567890"), None);    // no letter
/// assert_eq!(parse_de_kvnr("A12345"),     None);
/// assert_eq!(parse_de_kvnr(""),           None);
/// ```
pub fn parse_de_kvnr(s: &str) -> Option<String> {
    let cleaned: String = s
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_uppercase();
    if !cleaned.is_ascii() || cleaned.len() != 10 {
        return None;
    }
    let mut chars = cleaned.chars();
    let first = chars.next()?;
    if !first.is_ascii_alphabetic() {
        return None;
    }
    let digit_chars: Vec<char> = chars.collect();
    if !digit_chars.iter().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let letter_ord = (first as u32) - ('A' as u32) + 1;
    let mut combined: Vec<u32> = vec![letter_ord / 10, letter_ord % 10];
    for c in &digit_chars[..8] {
        combined.push(c.to_digit(10)?);
    }
    let mut total: u32 = 0;
    for (i, d) in combined.iter().enumerate() {
        let weight = if i % 2 == 0 { 1 } else { 2 };
        let product = d * weight;
        total += if product >= 10 { product - 9 } else { product };
    }
    let expected = digit_chars[8].to_digit(10)?;
    if total % 10 == expected {
        Some(cleaned)
    } else {
        None
    }
}

/// Per-position lookup table for the Italy *Codice Fiscale* check
/// character.
///
/// "Odd" positions are the 1st, 3rd, 5th, …, 15th characters
/// (1-indexed); they map per a specific table that intentionally
/// scatters values so single-character typos are likely to shift the
/// resulting check character.
fn cf_odd_value(c: char) -> Option<u32> {
    Some(match c {
        '0' | 'A' => 1,
        '1' | 'B' => 0,
        '2' | 'C' => 5,
        '3' | 'D' => 7,
        '4' | 'E' => 9,
        '5' | 'F' => 13,
        '6' | 'G' => 15,
        '7' | 'H' => 17,
        '8' | 'I' => 19,
        '9' | 'J' => 21,
        'K' => 2,
        'L' => 4,
        'M' => 18,
        'N' => 20,
        'O' => 11,
        'P' => 3,
        'Q' => 6,
        'R' => 8,
        'S' => 12,
        'T' => 14,
        'U' => 16,
        'V' => 10,
        'W' => 22,
        'X' => 25,
        'Y' => 24,
        'Z' => 23,
        _ => return None,
    })
}

/// "Even" positions (2nd, 4th, …, 14th, 1-indexed) for the Italy
/// *Codice Fiscale* check character. Numeric values map to their digit
/// value; letters map to `A=0`, `B=1`, …, `Z=25`.
fn cf_even_value(c: char) -> Option<u32> {
    Some(match c {
        '0' | 'A' => 0,
        '1' | 'B' => 1,
        '2' | 'C' => 2,
        '3' | 'D' => 3,
        '4' | 'E' => 4,
        '5' | 'F' => 5,
        '6' | 'G' => 6,
        '7' | 'H' => 7,
        '8' | 'I' => 8,
        '9' | 'J' => 9,
        'K' => 10,
        'L' => 11,
        'M' => 12,
        'N' => 13,
        'O' => 14,
        'P' => 15,
        'Q' => 16,
        'R' => 17,
        'S' => 18,
        'T' => 19,
        'U' => 20,
        'V' => 21,
        'W' => 22,
        'X' => 23,
        'Y' => 24,
        'Z' => 25,
        _ => return None,
    })
}

/// Parse an Italy *Codice Fiscale* (CF).
///
/// The CF is a 16-character alphanumeric identifier issued by the
/// Italian tax authority and used as the de-facto national healthcare
/// identifier. It encodes a coded form of the holder's name, date of
/// birth, sex, and commune of birth, followed by a Mod-26 check
/// character.
///
/// Check-character algorithm:
///
/// 1. For each of the first 15 characters, compute a numeric value
///    using two lookup tables — "odd" positions (1, 3, 5, …, 15;
///    1-indexed) use the scattered table; "even" positions (2, 4, …,
///    14) map digits and letters to their natural value.
/// 2. Sum the 15 values, take mod 26.
/// 3. Map `0..=25` to `A..=Z`. The result MUST equal the 16th
///    character.
///
/// Whitespace is stripped and letters are uppercased before parsing.
/// The canonical form is the 16-character uppercase string.
///
/// # Examples
///
/// ```
/// use person_matcher::identifiers::parse_it_cf;
///
/// // Synthetic CF with verified check character (sum 122, mod 26 = 18, 18→'S').
/// assert_eq!(
///     parse_it_cf("RSSMRA85T10A562S"),
///     Some("RSSMRA85T10A562S".to_string()),
/// );
/// // Lowercase and whitespace tolerated:
/// assert_eq!(
///     parse_it_cf("rss mra 85t 10a 562s"),
///     Some("RSSMRA85T10A562S".to_string()),
/// );
///
/// // Wrong check character:
/// assert_eq!(parse_it_cf("RSSMRA85T10A562X"), None);
///
/// // Wrong length:
/// assert_eq!(parse_it_cf("RSSMRA85T10A562"),  None);
/// assert_eq!(parse_it_cf("RSSMRA85T10A562SS"), None);
/// // Non-alphanumeric content:
/// assert_eq!(parse_it_cf("RSSMRA85T10A562!"), None);
/// assert_eq!(parse_it_cf(""),                  None);
/// ```
pub fn parse_it_cf(s: &str) -> Option<String> {
    let cleaned: String = s
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_uppercase();
    if !cleaned.is_ascii() || cleaned.len() != 16 {
        return None;
    }
    if !cleaned.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    let chars: Vec<char> = cleaned.chars().collect();
    let mut total: u32 = 0;
    for (i, c) in chars.iter().take(15).enumerate() {
        // 1-indexed position parity: index 0 is position 1 (odd).
        let value = if i % 2 == 0 {
            cf_odd_value(*c)?
        } else {
            cf_even_value(*c)?
        };
        total += value;
    }
    let expected_check = (b'A' + (total % 26) as u8) as char;
    if chars[15] == expected_check {
        Some(cleaned)
    } else {
        None
    }
}

/// Parse a Netherlands BSN (*Burgerservicenummer*).
///
/// The BSN is a 9-digit citizen-service number used by all Dutch
/// authorities, including healthcare providers. It carries an
/// "11-test" check rule originally derived from the bank account
/// number validation.
///
/// Check rule (the "11-test"):
///
/// `9·d₁ + 8·d₂ + 7·d₃ + 6·d₄ + 5·d₅ + 4·d₆ + 3·d₇ + 2·d₈ − d₉ ≡ 0 (mod 11)`
///
/// Non-digit characters are stripped before validation (so spaces or
/// hyphens used for readability are tolerated). The all-zero string
/// `000000000` is rejected even though it satisfies the arithmetic.
/// The canonical form is the 9-digit compact string.
///
/// # Examples
///
/// ```
/// use person_matcher::identifiers::parse_nl_bsn;
///
/// // 111222333: 9·1 + 8·1 + 7·1 + 6·2 + 5·2 + 4·2 + 3·3 + 2·3 − 3 = 66; 66 mod 11 = 0.
/// assert_eq!(parse_nl_bsn("111222333"), Some("111222333".to_string()));
/// assert_eq!(parse_nl_bsn("111 222 333"), Some("111222333".to_string()));
///
/// // Wrong check (final digit changed):
/// assert_eq!(parse_nl_bsn("111222334"), None);
///
/// // Wrong length, non-digits, all-zeros, empty:
/// assert_eq!(parse_nl_bsn("12345"),     None);
/// assert_eq!(parse_nl_bsn("ABCDEFGHI"), None);
/// assert_eq!(parse_nl_bsn("000000000"), None);
/// assert_eq!(parse_nl_bsn(""),          None);
/// ```
pub fn parse_nl_bsn(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 9 {
        return None;
    }
    if digits.chars().all(|c| c == '0') {
        return None;
    }
    let weights: [i32; 9] = [9, 8, 7, 6, 5, 4, 3, 2, -1];
    let mut sum: i32 = 0;
    for (i, c) in digits.chars().enumerate() {
        sum += (c.to_digit(10)? as i32) * weights[i];
    }
    if sum % 11 == 0 { Some(digits) } else { None }
}

/// Parse a Sweden *Personnummer*.
///
/// The Swedish personal identity number is the national identifier
/// used for taxation, healthcare, banking, and similar purposes. It
/// comes in two textual layouts:
///
/// - 10-digit form: `YYMMDDNNNC` (or with a `-` / `+` separator
///   between the date and the serial, e.g. `460324-3850`). The `+`
///   separator indicates the holder is over 100 years old.
/// - 12-digit form: `YYYYMMDDNNNC` (or `19460324-3850`).
///
/// `Y`/`M`/`D` are the birth-date digits, `NNN` is a 3-digit serial
/// (odd = male, even = female under the historical convention), and
/// `C` is the Luhn check digit computed over the 10 digits of the
/// 10-digit form.
///
/// Non-digit characters are stripped before validation. The Luhn
/// check uses left-to-right weights `2, 1, 2, 1, 2, 1, 2, 1, 2, 1`;
/// products `≥ 10` are reduced by digit-sum; the total mod 10 must be
/// `0`.
///
/// The canonical form preserves the input length: 10-digit input
/// returns a 10-character string; 12-digit input returns a 12-character
/// string. Records using mixed layouts will not match deterministically
/// on this field, but they will still produce the correct Luhn
/// validation.
///
/// # Examples
///
/// ```
/// use person_matcher::identifiers::parse_se_personnummer;
///
/// // Synthetic 10-digit personnummer with verified Luhn (sum 40, mod 10 = 0).
/// assert_eq!(
///     parse_se_personnummer("4603243850"),
///     Some("4603243850".to_string()),
/// );
/// assert_eq!(
///     parse_se_personnummer("460324-3850"),
///     Some("4603243850".to_string()),
/// );
///
/// // 12-digit form canonicalises with the century preserved.
/// assert_eq!(
///     parse_se_personnummer("19460324-3850"),
///     Some("194603243850".to_string()),
/// );
///
/// // Wrong Luhn:
/// assert_eq!(parse_se_personnummer("4603243851"), None);
///
/// // Wrong length, non-digits, empty:
/// assert_eq!(parse_se_personnummer("12345"),       None);
/// assert_eq!(parse_se_personnummer("ABCDEFGHIJ"),  None);
/// assert_eq!(parse_se_personnummer(""),            None);
/// ```
pub fn parse_se_personnummer(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    let luhn_digits: &str = match digits.len() {
        10 => &digits,
        12 => &digits[2..],
        _ => return None,
    };
    let mut sum: u32 = 0;
    for (i, c) in luhn_digits.chars().enumerate() {
        let d = c.to_digit(10)?;
        let weight = if i % 2 == 0 { 2 } else { 1 };
        let product = d * weight;
        sum += if product >= 10 { product - 9 } else { product };
    }
    if sum.is_multiple_of(10) {
        Some(digits)
    } else {
        None
    }
}

/// Parse an Australia IHI (Individual Healthcare Identifier).
///
/// The IHI is the unique 16-digit identifier issued by the Healthcare
/// Identifiers Service (HI Service) of the Australian Digital Health
/// Agency. It conforms to ISO/IEC 7812-1 with a Luhn check digit.
///
/// Non-digit characters are stripped before validation. The Luhn
/// check uses left-to-right weights `2, 1, 2, 1, …` over all 16
/// digits (the rightmost digit is the check); products `≥ 10` are
/// reduced by digit-sum; the total mod 10 must be `0`. The structural
/// convention that real IHIs begin with `800360` is **not** enforced
/// here so test and migration data with other prefixes parse cleanly.
///
/// # Examples
///
/// ```
/// use person_matcher::identifiers::parse_au_ihi;
///
/// // Synthetic 16-digit IHI with verified Luhn.
/// assert_eq!(
///     parse_au_ihi("8003601234567894"),
///     Some("8003601234567894".to_string()),
/// );
/// assert_eq!(
///     parse_au_ihi("8003 6012 3456 7894"),
///     Some("8003601234567894".to_string()),
/// );
///
/// // Wrong Luhn / wrong length / non-digits:
/// assert_eq!(parse_au_ihi("8003601234567890"), None);
/// assert_eq!(parse_au_ihi("12345"),            None);
/// assert_eq!(parse_au_ihi("ABCDEFGHIJKLMNOP"), None);
/// assert_eq!(parse_au_ihi(""),                 None);
/// ```
pub fn parse_au_ihi(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 16 {
        return None;
    }
    let mut sum: u32 = 0;
    for (i, c) in digits.chars().enumerate() {
        let d = c.to_digit(10)?;
        let weight = if i % 2 == 0 { 2 } else { 1 };
        let product = d * weight;
        sum += if product >= 10 { product - 9 } else { product };
    }
    if sum.is_multiple_of(10) {
        Some(digits)
    } else {
        None
    }
}

/// Parse a Scotland CHI (Community Health Index) Number.
///
/// The CHI Number is the unique person identifier used by NHS
/// Scotland. Structure: 10 digits formatted `DDMMYYSSSC`, where
/// `DDMMYY` is the holder's date of birth, `SSS` is a 3-digit
/// sequence with the third digit encoding sex (odd = male, even =
/// female), and `C` is a Mod-11 check digit computed in the same
/// fashion as the UK United Kingdom National Health Service Number.
///
/// Check rule (Mod-11):
///
/// 1. Multiply each of the first 9 digits by the weights
///    `10, 9, 8, 7, 6, 5, 4, 3, 2`.
/// 2. Sum, take mod 11.
/// 3. The check digit is `(11 − (sum mod 11)) mod 11`. A computed
///    check of `10` indicates an invalid identifier and is rejected.
///
/// Non-digit characters are stripped before validation. The canonical
/// form is the 10-digit compact string. Although the United Kingdom
/// National Health Service Number and the CHI Number share the same
/// Mod-11 algorithm, the two are
/// **scheme-local** in this crate and never cross-match (per spec
/// FR-13 / §12.1).
///
/// # Examples
///
/// ```
/// use person_matcher::identifiers::parse_uk_chi_number;
///
/// // Synthetic CHI with verified Mod-11 (sum 74, check = 3).
/// assert_eq!(
///     parse_uk_chi_number("0101701233"),
///     Some("0101701233".to_string()),
/// );
/// assert_eq!(
///     parse_uk_chi_number("010 170 1233"),
///     Some("0101701233".to_string()),
/// );
///
/// // Wrong check / length / non-digits:
/// assert_eq!(parse_uk_chi_number("0101701234"), None);
/// assert_eq!(parse_uk_chi_number("12345"),      None);
/// assert_eq!(parse_uk_chi_number("ABCDEFGHIJ"), None);
/// assert_eq!(parse_uk_chi_number(""),           None);
/// ```
pub fn parse_uk_chi_number(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 10 {
        return None;
    }
    let chars: Vec<u32> = digits
        .chars()
        .map(|c| c.to_digit(10).expect("filtered to digits"))
        .collect();
    let weights = [10u32, 9, 8, 7, 6, 5, 4, 3, 2];
    let sum: u32 = chars
        .iter()
        .take(9)
        .zip(weights.iter())
        .map(|(d, w)| d * w)
        .sum();
    let check = (11 - (sum % 11)) % 11;
    if check == 10 {
        return None;
    }
    if check == chars[9] {
        Some(digits)
    } else {
        None
    }
}

// ----------------------------------------------------------------------------
// Additional national personal identifiers (T-27).
//
// Each parser canonicalises whitespace + (where applicable) case, and verifies
// the scheme's check digit / check character. Parsers return Option<String>;
// `Some(canonical)` is suitable for byte-equality comparison.
// ----------------------------------------------------------------------------

/// Parse a Belgium *Rijksregisternummer* (National Number).
///
/// 11 digits: `YYMMDD` + 3-digit serial + 2-digit Mod-97 check.
/// Pre-2000 births: check = `97 − (first-9-digits mod 97)`.
/// 2000-and-later births: a `"2"` is prepended before the modulo step.
/// The parser tries both and accepts either.
///
/// ```
/// use person_matcher::identifiers::parse_be_nn;
/// assert_eq!(parse_be_nn("80010100107"), Some("80010100107".to_string()));
/// assert_eq!(parse_be_nn("80.01.01-001.07"), Some("80010100107".to_string()));
/// assert_eq!(parse_be_nn("80010100100"), None);   // wrong check
/// assert_eq!(parse_be_nn("12345"), None);         // wrong length
/// ```
pub fn parse_be_nn(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 11 {
        return None;
    }
    let body: u64 = digits[..9].parse().ok()?;
    let check: u64 = digits[9..11].parse().ok()?;
    let pre2000 = 97 - body % 97;
    let post2000_body: u64 = format!("2{}", &digits[..9]).parse().ok()?;
    let post2000 = 97 - post2000_body % 97;
    if check == pre2000 || check == post2000 {
        Some(digits)
    } else {
        None
    }
}

/// Parse a Bulgaria EGN (*Edinen grazhdanski nomer*).
///
/// 10 digits: `YYMMDD` (with month-offset for century) + 3-digit area/serial
/// + 1 check digit. Check uses weights `[2,4,8,5,10,9,7,3,6]` mod 11 (10 → 0).
///
/// ```
/// use person_matcher::identifiers::parse_bg_egn;
/// assert_eq!(parse_bg_egn("8001010013"), Some("8001010013".to_string()));
/// assert_eq!(parse_bg_egn("8001010014"), None);
/// assert_eq!(parse_bg_egn(""), None);
/// ```
pub fn parse_bg_egn(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 10 {
        return None;
    }
    let weights: [u32; 9] = [2, 4, 8, 5, 10, 9, 7, 3, 6];
    let mut sum: u32 = 0;
    for (i, c) in digits.chars().take(9).enumerate() {
        sum += c.to_digit(10)? * weights[i];
    }
    let expected = if sum % 11 == 10 { 0 } else { sum % 11 };
    if digits.chars().nth(9)?.to_digit(10)? == expected {
        Some(digits)
    } else {
        None
    }
}

/// Parse a Czech Republic *Rodné číslo*.
///
/// 9 or 10 digits. The 10-digit form (post-1953) is divisible by 11 (with
/// the edge case that mod-11 = 10 collapses to a trailing 0; the resulting
/// 10-digit number may NOT be divisible by 11). The 9-digit form (pre-1954)
/// is accepted as-is.
///
/// ```
/// use person_matcher::identifiers::parse_cz_rc;
/// assert_eq!(parse_cz_rc("8001150014"), Some("8001150014".to_string()));
/// assert_eq!(parse_cz_rc("800115001"), Some("800115001".to_string())); // 9-digit pre-1954
/// assert_eq!(parse_cz_rc("8001150015"), None);
/// ```
pub fn parse_cz_rc(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    match digits.len() {
        9 => Some(digits),
        10 => {
            let n: u64 = digits.parse().ok()?;
            // Standard rule: 10-digit RČ is divisible by 11. Edge case:
            // when first-9-digit mod 11 = 10, the trailing 0 is used and
            // the full number's mod 11 is 10, not 0.
            let head: u64 = digits[..9].parse().ok()?;
            let tail = digits.chars().last()?.to_digit(10)?;
            if n.is_multiple_of(11) || (head % 11 == 10 && tail == 0) {
                Some(digits)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Parse a Denmark CPR (*Centrale Personregister*).
///
/// 10 digits `DDMMYYNNNN`. Format-only validation; the historical Modulus-11
/// check was abandoned in 2007.
///
/// ```
/// use person_matcher::identifiers::parse_dk_cpr;
/// assert_eq!(parse_dk_cpr("1501801234"), Some("1501801234".to_string()));
/// assert_eq!(parse_dk_cpr("150180-1234"), Some("1501801234".to_string()));
/// assert_eq!(parse_dk_cpr("12345"), None);
/// ```
pub fn parse_dk_cpr(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() == 10 {
        Some(digits)
    } else {
        None
    }
}

/// Cascading Mod-11 check used by Estonia and Lithuania.
fn baltic_cascade_check(digits: &str) -> Option<u32> {
    const PASS1: [u32; 10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 1];
    const PASS2: [u32; 10] = [3, 4, 5, 6, 7, 8, 9, 1, 2, 3];
    let body: Vec<u32> = digits
        .chars()
        .take(10)
        .filter_map(|c| c.to_digit(10))
        .collect();
    if body.len() != 10 {
        return None;
    }
    let s1: u32 = body.iter().zip(PASS1.iter()).map(|(d, w)| d * w).sum();
    let r1 = s1 % 11;
    if r1 < 10 {
        return Some(r1);
    }
    let s2: u32 = body.iter().zip(PASS2.iter()).map(|(d, w)| d * w).sum();
    let r2 = s2 % 11;
    if r2 < 10 { Some(r2) } else { Some(0) }
}

/// Parse an Estonia *Isikukood* (Personal Identification Code).
///
/// 11 digits `GYYMMDDNNNC`. Check digit uses a cascading Mod-11 algorithm.
///
/// ```
/// use person_matcher::identifiers::parse_ee_ik;
/// assert_eq!(parse_ee_ik("48001150011"), Some("48001150011".to_string()));
/// assert_eq!(parse_ee_ik("48001150012"), None);
/// ```
pub fn parse_ee_ik(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 11 {
        return None;
    }
    let expected = baltic_cascade_check(&digits[..10])?;
    if digits.chars().nth(10)?.to_digit(10)? == expected {
        Some(digits)
    } else {
        None
    }
}

/// Parse a Spain DNI / NIE.
///
/// 8 digits (NIE: prefixed `X`/`Y`/`Z`) + 1 control letter. The letter is
/// `"TRWAGMYFPDXBNJZSQVHLCKE"` indexed by `number mod 23`. NIE prefixes map
/// to leading digits: `X→0`, `Y→1`, `Z→2`.
///
/// ```
/// use person_matcher::identifiers::parse_es_dni;
/// assert_eq!(parse_es_dni("12345678Z"), Some("12345678Z".to_string()));
/// assert_eq!(parse_es_dni("12345678-Z"), Some("12345678Z".to_string()));
/// assert_eq!(parse_es_dni("12345678A"), None);  // wrong letter
/// ```
pub fn parse_es_dni(s: &str) -> Option<String> {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_uppercase();
    if cleaned.is_empty() {
        return None;
    }
    let last = cleaned.chars().last()?;
    if !last.is_ascii_alphabetic() {
        return None;
    }
    let body = &cleaned[..cleaned.len() - 1];
    let n: u64 = match body.chars().next()? {
        'X' => format!("0{}", &body[1..]).parse().ok()?,
        'Y' => format!("1{}", &body[1..]).parse().ok()?,
        'Z' => format!("2{}", &body[1..]).parse().ok()?,
        d if d.is_ascii_digit() => body.parse().ok()?,
        _ => return None,
    };
    const LETTERS: &[u8; 23] = b"TRWAGMYFPDXBNJZSQVHLCKE";
    let expected = LETTERS[(n % 23) as usize] as char;
    if last == expected {
        Some(cleaned)
    } else {
        None
    }
}

/// Parse a Finland HETU (*Henkilötunnus*).
///
/// 11 characters `DDMMYYCZZZK` where `C` is a century sign (`-`/`+`/`A` and
/// later additions) and `K` is a check character from
/// `"0123456789ABCDEFHJKLMNPRSTUVWXY"` indexed by `(DDMMYYZZZ as 9-digit
/// number) mod 31`.
///
/// ```
/// use person_matcher::identifiers::parse_fi_hetu;
/// assert_eq!(parse_fi_hetu("150180-999B"), Some("150180-999B".to_string()));
/// assert_eq!(parse_fi_hetu("150180-999C"), None);
/// ```
pub fn parse_fi_hetu(s: &str) -> Option<String> {
    let cleaned: String = s
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_uppercase();
    if !cleaned.is_ascii() || cleaned.len() != 11 {
        return None;
    }
    let date: &str = &cleaned[..6];
    let sign = cleaned.chars().nth(6)?;
    let serial: &str = &cleaned[7..10];
    let check = cleaned.chars().nth(10)?;
    if !date.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if !serial.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    // Accept the historically-known and FICORA-extended signs.
    if !matches!(
        sign,
        '-' | '+' | 'A' | 'B' | 'C' | 'D' | 'E' | 'F' | 'X' | 'Y'
    ) {
        return None;
    }
    let n: u64 = format!("{}{}", date, serial).parse().ok()?;
    const TABLE: &[u8; 31] = b"0123456789ABCDEFHJKLMNPRSTUVWXY";
    let expected = TABLE[(n % 31) as usize] as char;
    if check == expected {
        Some(cleaned)
    } else {
        None
    }
}

/// Parse a Croatia OIB (*Osobni identifikacijski broj*).
///
/// 11 digits. Check digit via ISO 7064 MOD 11,10.
///
/// ```
/// use person_matcher::identifiers::parse_hr_oib;
/// assert_eq!(parse_hr_oib("12345678903"), Some("12345678903".to_string()));
/// assert_eq!(parse_hr_oib("12345678901"), None);
/// ```
pub fn parse_hr_oib(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 11 {
        return None;
    }
    let mut acc: u32 = 10;
    for c in digits.chars().take(10) {
        let d = c.to_digit(10)?;
        let mut x = (d + acc) % 10;
        if x == 0 {
            x = 10;
        }
        acc = (x * 2) % 11;
    }
    let expected = (11 - acc) % 10;
    if digits.chars().nth(10)?.to_digit(10)? == expected {
        Some(digits)
    } else {
        None
    }
}

/// Parse an Iceland *Kennitala*.
///
/// 10 digits `DDMMYYRRCN`. Check digit uses weights `[3,2,7,6,5,4,3,2]`
/// over the first 8 digits; mod 11 = 10 is invalid.
///
/// ```
/// use person_matcher::identifiers::parse_is_kt;
/// assert_eq!(parse_is_kt("1501802529"), Some("1501802529".to_string()));
/// assert_eq!(parse_is_kt("1501802539"), None);  // wrong check digit
/// ```
pub fn parse_is_kt(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 10 {
        return None;
    }
    const WEIGHTS: [u32; 8] = [3, 2, 7, 6, 5, 4, 3, 2];
    let mut sum: u32 = 0;
    for (i, c) in digits.chars().take(8).enumerate() {
        sum += c.to_digit(10)? * WEIGHTS[i];
    }
    let r = sum % 11;
    if r == 10 {
        return None;
    }
    let expected = (11 - r) % 11;
    if digits.chars().nth(8)?.to_digit(10)? == expected {
        Some(digits)
    } else {
        None
    }
}

/// Parse a Lithuania *Asmens kodas*.
///
/// 11 digits `GYYMMDDNNNC` with the same cascading Mod-11 check as Estonia.
///
/// ```
/// use person_matcher::identifiers::parse_lt_ak;
/// assert_eq!(parse_lt_ak("48001150011"), Some("48001150011".to_string()));
/// assert_eq!(parse_lt_ak("48001150012"), None);
/// ```
pub fn parse_lt_ak(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 11 {
        return None;
    }
    let expected = baltic_cascade_check(&digits[..10])?;
    if digits.chars().nth(10)?.to_digit(10)? == expected {
        Some(digits)
    } else {
        None
    }
}

/// Parse a Latvia *Personas kods*.
///
/// 11 digits `DDMMYYCZZZK`. Check uses weights `[1,6,3,7,9,10,5,8,4,2]`
/// over the first 10 digits; `check = ((1101 − Σ) mod 11) mod 10`.
///
/// ```
/// use person_matcher::identifiers::parse_lv_pk;
/// assert_eq!(parse_lv_pk("15018010007"), Some("15018010007".to_string()));
/// assert_eq!(parse_lv_pk("15018010008"), None);
/// ```
pub fn parse_lv_pk(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 11 {
        return None;
    }
    const WEIGHTS: [i32; 10] = [1, 6, 3, 7, 9, 10, 5, 8, 4, 2];
    let mut sum: i32 = 0;
    for (i, c) in digits.chars().take(10).enumerate() {
        sum += (c.to_digit(10)? as i32) * WEIGHTS[i];
    }
    let expected = ((1101 - sum).rem_euclid(11)) % 10;
    if digits.chars().nth(10)?.to_digit(10)? as i32 == expected {
        Some(digits)
    } else {
        None
    }
}

/// Parse a Malta National ID.
///
/// 7 digits + 1 letter from `{M, G, A, P, L, H, B, Z}`. Format-only — the
/// suffix letter encodes geographic / registration provenance and is not a
/// check digit.
///
/// ```
/// use person_matcher::identifiers::parse_mt_id;
/// assert_eq!(parse_mt_id("1234567M"), Some("1234567M".to_string()));
/// assert_eq!(parse_mt_id("1234567X"), None);  // X not in valid letter set
/// ```
pub fn parse_mt_id(s: &str) -> Option<String> {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_uppercase();
    if cleaned.len() != 8 {
        return None;
    }
    let last = cleaned.chars().last()?;
    if !matches!(last, 'M' | 'G' | 'A' | 'P' | 'L' | 'H' | 'B' | 'Z') {
        return None;
    }
    if !cleaned[..7].chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(cleaned)
}

/// Parse a Norway *Fødselsnummer*.
///
/// 11 digits with two Mod-11 check digits. Check 1 weights:
/// `[3,7,6,1,8,9,4,5,2]` over the first 9 digits. Check 2 weights:
/// `[5,4,3,2,7,6,5,4,3,2]` over the first 10. mod 11 = 10 is invalid.
///
/// ```
/// use person_matcher::identifiers::parse_no_fnr;
/// assert_eq!(parse_no_fnr("15018012399"), Some("15018012399".to_string()));
/// assert_eq!(parse_no_fnr("15018012390"), None);
/// ```
pub fn parse_no_fnr(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 11 {
        return None;
    }
    const W1: [u32; 9] = [3, 7, 6, 1, 8, 9, 4, 5, 2];
    const W2: [u32; 10] = [5, 4, 3, 2, 7, 6, 5, 4, 3, 2];
    let body: Vec<u32> = digits.chars().filter_map(|c| c.to_digit(10)).collect();
    if body.len() != 11 {
        return None;
    }
    let s1: u32 = body.iter().take(9).zip(W1.iter()).map(|(d, w)| d * w).sum();
    let r1 = s1 % 11;
    if r1 == 10 {
        return None;
    }
    let c1 = (11 - r1) % 11;
    if c1 != body[9] {
        return None;
    }
    let s2: u32 = body
        .iter()
        .take(10)
        .zip(W2.iter())
        .map(|(d, w)| d * w)
        .sum();
    let r2 = s2 % 11;
    if r2 == 10 {
        return None;
    }
    let c2 = (11 - r2) % 11;
    if c2 != body[10] {
        return None;
    }
    Some(digits)
}

/// Parse a Poland PESEL.
///
/// 11 digits `YYMMDDZZZZK` with century-encoded month. Check uses weights
/// `[1,3,7,9,1,3,7,9,1,3]` over the first 10 digits;
/// `check = (10 − (Σ mod 10)) mod 10`.
///
/// ```
/// use person_matcher::identifiers::parse_pl_pesel;
/// assert_eq!(parse_pl_pesel("80011500014"), Some("80011500014".to_string()));
/// assert_eq!(parse_pl_pesel("80011500015"), None);
/// ```
pub fn parse_pl_pesel(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 11 {
        return None;
    }
    const WEIGHTS: [u32; 10] = [1, 3, 7, 9, 1, 3, 7, 9, 1, 3];
    let mut sum: u32 = 0;
    for (i, c) in digits.chars().take(10).enumerate() {
        sum += c.to_digit(10)? * WEIGHTS[i];
    }
    let expected = (10 - (sum % 10)) % 10;
    if digits.chars().nth(10)?.to_digit(10)? == expected {
        Some(digits)
    } else {
        None
    }
}

/// Parse a Romania CNP (*Cod Numeric Personal*).
///
/// 13 digits `SYYMMDDJJNNNK`. Check uses weights "279146358279" (`[2,7,9,1,
/// 4,6,3,5,8,2,7,9]`) over the first 12 digits; `r = Σ mod 11`; check is
/// `1` if `r == 10`, else `r`.
///
/// ```
/// use person_matcher::identifiers::parse_ro_cnp;
/// assert_eq!(parse_ro_cnp("1800115400012"), Some("1800115400012".to_string()));
/// assert_eq!(parse_ro_cnp("1800115400015"), None);
/// ```
pub fn parse_ro_cnp(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 13 {
        return None;
    }
    const WEIGHTS: [u32; 12] = [2, 7, 9, 1, 4, 6, 3, 5, 8, 2, 7, 9];
    let mut sum: u32 = 0;
    for (i, c) in digits.chars().take(12).enumerate() {
        sum += c.to_digit(10)? * WEIGHTS[i];
    }
    let r = sum % 11;
    let expected = if r == 10 { 1 } else { r };
    if digits.chars().nth(12)?.to_digit(10)? == expected {
        Some(digits)
    } else {
        None
    }
}

/// Parse a Slovenia EMŠO (*Enotna Matična Številka Občana*).
///
/// 13 digits `DDMMYYYRRGGGK`. Check uses weights `[7,6,5,4,3,2,7,6,5,4,3,2]`
/// over the first 12 digits; `r = Σ mod 11`; check is `0` if `r == 0`,
/// else `11 − r` (rejected if 10).
///
/// ```
/// use person_matcher::identifiers::parse_si_emso;
/// assert_eq!(parse_si_emso("1501980500015"), Some("1501980500015".to_string()));
/// assert_eq!(parse_si_emso("1501980500014"), None);
/// ```
pub fn parse_si_emso(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 13 {
        return None;
    }
    const WEIGHTS: [u32; 12] = [7, 6, 5, 4, 3, 2, 7, 6, 5, 4, 3, 2];
    let mut sum: u32 = 0;
    for (i, c) in digits.chars().take(12).enumerate() {
        sum += c.to_digit(10)? * WEIGHTS[i];
    }
    let r = sum % 11;
    let expected = if r == 0 { 0 } else { 11 - r };
    if expected == 10 {
        return None;
    }
    if digits.chars().nth(12)?.to_digit(10)? == expected {
        Some(digits)
    } else {
        None
    }
}

/// Parse a Slovakia *Rodné číslo*. Same algorithm as Czech RČ.
///
/// ```
/// use person_matcher::identifiers::parse_sk_rc;
/// assert_eq!(parse_sk_rc("8051150019"), Some("8051150019".to_string()));
/// assert_eq!(parse_sk_rc("8051150010"), None);
/// ```
pub fn parse_sk_rc(s: &str) -> Option<String> {
    parse_cz_rc(s)
}

/// Parse a United Kingdom National Insurance Number (NINO).
///
/// Format `AA999999A`: 2 prefix letters + 6 digits + 1 suffix letter.
/// Banned first prefix letters: `D F I Q U V`.
/// Banned second prefix letters: `D F I O Q U V`.
/// Banned admin prefixes: `OO CR FY MW NC PP PZ TN`.
/// Suffix MUST be one of `A B C D`. Format-only; no checksum.
///
/// ```
/// use person_matcher::identifiers::parse_uk_nino;
/// assert_eq!(parse_uk_nino("AB123456A"), Some("AB123456A".to_string()));
/// assert_eq!(parse_uk_nino("ab 12 34 56 a"), Some("AB123456A".to_string()));
/// assert_eq!(parse_uk_nino("DA123456A"), None);  // banned first letter
/// assert_eq!(parse_uk_nino("ABCDEFGHI"), None);
/// ```
pub fn parse_uk_nino(s: &str) -> Option<String> {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_uppercase();
    if cleaned.len() != 9 {
        return None;
    }
    let bytes = cleaned.as_bytes();
    let p1 = bytes[0] as char;
    let p2 = bytes[1] as char;
    if !p1.is_ascii_alphabetic() || !p2.is_ascii_alphabetic() {
        return None;
    }
    if matches!(p1, 'D' | 'F' | 'I' | 'Q' | 'U' | 'V') {
        return None;
    }
    if matches!(p2, 'D' | 'F' | 'I' | 'O' | 'Q' | 'U' | 'V') {
        return None;
    }
    let prefix = &cleaned[..2];
    if matches!(
        prefix,
        "OO" | "CR" | "FY" | "MW" | "NC" | "PP" | "PZ" | "TN"
    ) {
        return None;
    }
    if !cleaned[2..8].chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let suffix = bytes[8] as char;
    if !matches!(suffix, 'A' | 'B' | 'C' | 'D') {
        return None;
    }
    Some(cleaned)
}

// ----------------------------------------------------------------------------
// Five additional personal national identifiers (T-28).
// ----------------------------------------------------------------------------

/// Parse a Greece DSS (Dematerialised Securities System) investor share code.
///
/// 10-digit investor identifier issued by the Hellenic Central Securities
/// Depository (ATHEXCSD). Format-only validation: 10 ASCII digits.
///
/// ```
/// use person_matcher::identifiers::parse_gr_dss;
/// assert_eq!(parse_gr_dss("1234567890"), Some("1234567890".to_string()));
/// assert_eq!(parse_gr_dss("12345"), None);
/// assert_eq!(parse_gr_dss("ABCDEFGHIJ"), None);
/// ```
pub fn parse_gr_dss(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() == 10 {
        Some(digits)
    } else {
        None
    }
}

/// Parse a Liechtenstein National Identity Card Number.
///
/// Combination of 2 letters and 8 digits (the example in the spec
/// `ID022143586` shows a 9-digit run, so the parser accepts 8 *or* 9
/// trailing digits — total length 10 or 11). Note: per Liechtenstein
/// practice the number is **regenerated on each renewal**, so for
/// cross-renewal matching consumers should prefer
/// [`crate::PassportBook`] with `country = "LI"`.
///
/// ```
/// use person_matcher::identifiers::parse_li_id;
/// assert_eq!(parse_li_id("ID12345678"), Some("ID12345678".to_string()));
/// assert_eq!(parse_li_id("ID022143586"), Some("ID022143586".to_string()));
/// assert_eq!(parse_li_id("12 34 56 78"), None);  // missing letters
/// assert_eq!(parse_li_id(""), None);
/// ```
pub fn parse_li_id(s: &str) -> Option<String> {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_uppercase();
    if !(10..=11).contains(&cleaned.len()) {
        return None;
    }
    let chars: Vec<char> = cleaned.chars().collect();
    if !chars[0].is_ascii_alphabetic() || !chars[1].is_ascii_alphabetic() {
        return None;
    }
    if !chars[2..].iter().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(cleaned)
}

/// Parse a Netherlands National Identity Card Number.
///
/// 9 characters: positions 1 and 2 are uppercase letters `[A-Z]` except
/// `O`; positions 3–8 are alphanumeric `[A-Z0-9]` except `O`; position 9
/// is a digit `[0-9]`. The character `O` is disallowed (to avoid
/// confusion with `0`), but the digit `0` is allowed.
///
/// ```
/// use person_matcher::identifiers::parse_nl_id;
/// assert_eq!(parse_nl_id("AB1234567"), Some("AB1234567".to_string()));
/// assert_eq!(parse_nl_id("ab 12 34 567"), Some("AB1234567".to_string()));
/// assert_eq!(parse_nl_id("AO1234567"), None);   // O is banned
/// assert_eq!(parse_nl_id("AB12345AB"), None);   // last must be digit
/// assert_eq!(parse_nl_id("12345AB67"), None);   // leading must be letters
/// ```
pub fn parse_nl_id(s: &str) -> Option<String> {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_uppercase();
    if cleaned.len() != 9 {
        return None;
    }
    let chars: Vec<char> = cleaned.chars().collect();
    for c in chars.iter().take(2) {
        if !c.is_ascii_alphabetic() || *c == 'O' {
            return None;
        }
    }
    for c in chars.iter().take(8).skip(2) {
        if !c.is_ascii_alphanumeric() || *c == 'O' {
            return None;
        }
    }
    if !chars[8].is_ascii_digit() {
        return None;
    }
    Some(cleaned)
}

/// Parse a Poland NIP (*Numer Identyfikacji Podatkowej*).
///
/// 10 digits with a weighted Mod-11 check. Weights for the first 9
/// digits: `[6, 5, 7, 2, 3, 4, 5, 6, 7]`. `r = Σ mod 11`; a remainder
/// of 10 indicates an invalid NIP; otherwise the 10th digit MUST equal
/// `r`.
///
/// ```
/// use person_matcher::identifiers::parse_pl_nip;
/// assert_eq!(parse_pl_nip("1234567802"), Some("1234567802".to_string()));
/// assert_eq!(parse_pl_nip("123-456-78-02"), Some("1234567802".to_string()));
/// assert_eq!(parse_pl_nip("1234567803"), None);    // wrong check
/// assert_eq!(parse_pl_nip("1234567890"), None);    // r = 10 — invalid by spec
/// ```
pub fn parse_pl_nip(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 10 {
        return None;
    }
    const WEIGHTS: [u32; 9] = [6, 5, 7, 2, 3, 4, 5, 6, 7];
    let mut sum: u32 = 0;
    for (i, c) in digits.chars().take(9).enumerate() {
        sum += c.to_digit(10)? * WEIGHTS[i];
    }
    let r = sum % 11;
    if r == 10 {
        return None;
    }
    if digits.chars().nth(9)?.to_digit(10)? == r {
        Some(digits)
    } else {
        None
    }
}

/// Parse a Portugal NIF (*Número de Identificação Fiscal*).
///
/// 9 digits with a weighted Mod-11 check. Weights for the first 8
/// digits: `[9, 8, 7, 6, 5, 4, 3, 2]`. `r = Σ mod 11`; check is `0` if
/// `r < 2`, else `11 − r`. The 9th digit MUST equal the check.
///
/// ```
/// use person_matcher::identifiers::parse_pt_nif;
/// assert_eq!(parse_pt_nif("123456789"), Some("123456789".to_string()));
/// assert_eq!(parse_pt_nif("123 456 789"), Some("123456789".to_string()));
/// assert_eq!(parse_pt_nif("123456780"), None);
/// ```
pub fn parse_pt_nif(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 9 {
        return None;
    }
    const WEIGHTS: [u32; 8] = [9, 8, 7, 6, 5, 4, 3, 2];
    let mut sum: u32 = 0;
    for (i, c) in digits.chars().take(8).enumerate() {
        sum += c.to_digit(10)? * WEIGHTS[i];
    }
    let r = sum % 11;
    let expected = if r < 2 { 0 } else { 11 - r };
    if digits.chars().nth(8)?.to_digit(10)? == expected {
        Some(digits)
    } else {
        None
    }
}

// ----------------------------------------------------------------------------
// T-17.1 — seven next-batch national identifier schemes.
//
// Per the T-17 spike (§21.4 / §23.2): one parser per jurisdiction the
// crate already supports phones for but not national IDs.
// ----------------------------------------------------------------------------

/// Parse a Brazil CPF (*Cadastro de Pessoas Físicas*).
///
/// The CPF is 11 digits, often formatted `NNN.NNN.NNN-DD`. The last two
/// digits are computed check digits using weighted Mod-11. The parser
/// strips non-digits, requires exactly 11 digits, rejects all-equal
/// sequences (the canonical sentinel / test vectors a real CPF MUST
/// NOT take), and validates both check digits.
///
/// ```
/// use person_matcher::identifiers::parse_br_cpf;
/// assert_eq!(parse_br_cpf("123.456.789-09"), Some("12345678909".to_string()));
/// assert_eq!(parse_br_cpf("12345678909"),    Some("12345678909".to_string()));
/// assert_eq!(parse_br_cpf("12345678900"),    None);             // wrong D2
/// assert_eq!(parse_br_cpf("11111111111"),    None);             // all-equal sentinel
/// assert_eq!(parse_br_cpf("1234567890"),     None);             // too short
/// ```
pub fn parse_br_cpf(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 11 {
        return None;
    }
    let bytes = digits.as_bytes();
    if bytes.iter().all(|&b| b == bytes[0]) {
        return None;
    }
    let d = |i: usize| (bytes[i] - b'0') as u32;
    let mut sum1: u32 = 0;
    for i in 0..9 {
        sum1 += d(i) * (10 - i as u32);
    }
    let r1 = sum1 % 11;
    let exp1 = if r1 < 2 { 0 } else { 11 - r1 };
    if d(9) != exp1 {
        return None;
    }
    let mut sum2: u32 = 0;
    for i in 0..10 {
        sum2 += d(i) * (11 - i as u32);
    }
    let r2 = sum2 % 11;
    let exp2 = if r2 < 2 { 0 } else { 11 - r2 };
    if d(10) != exp2 {
        return None;
    }
    Some(digits)
}

/// Parse a China Resident Identity Card number (*居民身份证*, 18-digit
/// 1999 reform).
///
/// 18 characters: 17 digits + a check character (digit or `X`). The
/// substring at positions 6..14 encodes the date of birth (`YYYYMMDD`)
/// and MUST be a valid calendar date. The check character is computed
/// from a weighted Mod-11 sum over the 17 leading digits with the
/// lookup `1,0,X,9,8,7,6,5,4,3,2`. Lowercase `x` is accepted and
/// canonicalised to uppercase. The 15-digit pre-1999 form is NOT
/// accepted; consumers MUST migrate to the 18-digit form before
/// matching (the conversion is well-documented but jurisdiction-locked
/// and out of scope for this parser).
///
/// ```
/// use person_matcher::identifiers::parse_cn_rrn;
/// assert_eq!(
///     parse_cn_rrn("11010519491231002X"),
///     Some("11010519491231002X".to_string()),
/// );
/// assert_eq!(
///     parse_cn_rrn("11010519491231002x"),
///     Some("11010519491231002X".to_string()),
/// );
/// assert_eq!(parse_cn_rrn("11010519491231002Y"), None);          // wrong check
/// assert_eq!(parse_cn_rrn("11010513491231002X"), None);          // invalid month
/// assert_eq!(parse_cn_rrn("110105194912310020"), None);          // wrong check
/// assert_eq!(parse_cn_rrn("11010519491231"),     None);          // too short
/// ```
pub fn parse_cn_rrn(s: &str) -> Option<String> {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if cleaned.len() != 18 {
        return None;
    }
    let bytes = cleaned.as_bytes();
    for &b in &bytes[..17] {
        if !b.is_ascii_digit() {
            return None;
        }
    }
    if !bytes[17].is_ascii_digit() && bytes[17] != b'X' {
        return None;
    }
    let yyyy: i32 = cleaned[6..10].parse().ok()?;
    let mm: u32 = cleaned[10..12].parse().ok()?;
    let dd: u32 = cleaned[12..14].parse().ok()?;
    jiff::civil::Date::new(yyyy as i16, mm as i8, dd as i8).ok()?;
    const WEIGHTS: [u32; 17] = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2];
    const CHECK: [u8; 11] = [
        b'1', b'0', b'X', b'9', b'8', b'7', b'6', b'5', b'4', b'3', b'2',
    ];
    let mut sum: u32 = 0;
    for i in 0..17 {
        sum += u32::from(bytes[i] - b'0') * WEIGHTS[i];
    }
    if bytes[17] != CHECK[(sum % 11) as usize] {
        return None;
    }
    Some(cleaned)
}

/// Parse an India Aadhaar number (12 digits, Verhoeff check digit).
///
/// The Verhoeff algorithm uses two precomputed lookup tables (the
/// dihedral-group multiplication table `D` and the permutation table
/// `P`) and runs in linear time over the input. The parser strips
/// non-digits, requires exactly 12 digits, rejects all-equal sequences
/// and the UIDAI-test-prefix ranges (numbers beginning with `0` or
/// `1`, which UIDAI guidance reserves and never issues to real
/// citizens), and validates the Verhoeff check digit at the rightmost
/// position.
///
/// ```
/// use person_matcher::identifiers::parse_in_aadhaar;
/// assert_eq!(parse_in_aadhaar("234123412346"),   Some("234123412346".to_string()));
/// assert_eq!(parse_in_aadhaar("2341 2341 2346"), Some("234123412346".to_string()));
/// assert_eq!(parse_in_aadhaar("234123412347"),   None);  // wrong check
/// assert_eq!(parse_in_aadhaar("234123412"),      None);  // too short
/// assert_eq!(parse_in_aadhaar("222222222222"),   None);  // all-equal sentinel
/// assert_eq!(parse_in_aadhaar("034123412346"),   None);  // reserved prefix
/// ```
pub fn parse_in_aadhaar(s: &str) -> Option<String> {
    const VERHOEFF_D: [[u8; 10]; 10] = [
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        [1, 2, 3, 4, 0, 6, 7, 8, 9, 5],
        [2, 3, 4, 0, 1, 7, 8, 9, 5, 6],
        [3, 4, 0, 1, 2, 8, 9, 5, 6, 7],
        [4, 0, 1, 2, 3, 9, 5, 6, 7, 8],
        [5, 9, 8, 7, 6, 0, 4, 3, 2, 1],
        [6, 5, 9, 8, 7, 1, 0, 4, 3, 2],
        [7, 6, 5, 9, 8, 2, 1, 0, 4, 3],
        [8, 7, 6, 5, 9, 3, 2, 1, 0, 4],
        [9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
    ];
    const VERHOEFF_P: [[u8; 10]; 8] = [
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        [1, 5, 7, 6, 2, 8, 3, 0, 9, 4],
        [5, 8, 0, 3, 7, 9, 6, 1, 4, 2],
        [8, 9, 1, 6, 0, 4, 3, 5, 2, 7],
        [9, 4, 5, 3, 1, 2, 6, 8, 7, 0],
        [4, 2, 8, 6, 5, 7, 3, 9, 0, 1],
        [2, 7, 9, 3, 8, 0, 6, 4, 1, 5],
        [7, 0, 4, 6, 9, 1, 3, 2, 5, 8],
    ];
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 12 {
        return None;
    }
    let bytes = digits.as_bytes();
    if bytes.iter().all(|&b| b == bytes[0]) {
        return None;
    }
    if bytes[0] == b'0' || bytes[0] == b'1' {
        return None;
    }
    let mut c: u8 = 0;
    for i in 0..12 {
        let d = bytes[11 - i] - b'0';
        c = VERHOEFF_D[c as usize][VERHOEFF_P[i % 8][d as usize] as usize];
    }
    if c == 0 { Some(digits) } else { None }
}

/// Parse a Japan My Number (個人番号, 12 digits).
///
/// The check digit is computed by a weighted Mod-11 sum over the
/// first 11 digits using the weights `[6, 5, 4, 3, 2, 7, 6, 5, 4, 3, 2]`
/// (per the Japanese e-Gov Cabinet Order specification). If the
/// remainder is `< 2`, the check digit is `0`; otherwise it is
/// `11 - remainder`.
///
/// ```
/// use person_matcher::identifiers::parse_jp_my_number;
/// assert_eq!(parse_jp_my_number("123456789018"),   Some("123456789018".to_string()));
/// assert_eq!(parse_jp_my_number("1234 5678 9018"), Some("123456789018".to_string()));
/// assert_eq!(parse_jp_my_number("123456789010"),   None);  // wrong check
/// assert_eq!(parse_jp_my_number("12345678901"),    None);  // too short
/// ```
pub fn parse_jp_my_number(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 12 {
        return None;
    }
    let bytes = digits.as_bytes();
    const WEIGHTS: [u32; 11] = [6, 5, 4, 3, 2, 7, 6, 5, 4, 3, 2];
    let mut sum: u32 = 0;
    for i in 0..11 {
        sum += u32::from(bytes[i] - b'0') * WEIGHTS[i];
    }
    let r = sum % 11;
    let expected = if r < 2 { 0 } else { 11 - r };
    if u32::from(bytes[11] - b'0') != expected {
        return None;
    }
    Some(digits)
}

/// Parse a Mexico CURP (*Clave Única de Registro de Población*).
///
/// 18 characters with rich internal structure: 4 letters (surname /
/// given-name initials), 6 digits (`YYMMDD`), 1 sex char (`H` or `M`),
/// 2 letters (state code), 3 letters (internal consonants), 1
/// alphanumeric (homonym discriminator), 1 check digit. The parser
/// uppercases, validates the structural shape, verifies the embedded
/// date of birth is a valid calendar date (century inferred per the
/// usual Mexican convention: `YY <= 29 → 20YY`, else `19YY`), and
/// validates the Mod-10 weighted check digit using the standard CURP
/// value table (`0..9` literal, `A..N` = 10..23, `Ñ` = 24,
/// `O..Z` = 25..36).
///
/// Ñ in the input is accepted; non-ASCII characters other than Ñ are
/// rejected.
///
/// ```
/// use person_matcher::identifiers::parse_mx_curp;
/// assert_eq!(
///     parse_mx_curp("HEGG560427MVZRRL04"),
///     Some("HEGG560427MVZRRL04".to_string()),
/// );
/// assert_eq!(
///     parse_mx_curp("hegg560427mvzrrl04"),
///     Some("HEGG560427MVZRRL04".to_string()),
/// );
/// assert_eq!(parse_mx_curp("HEGG560427MVZRRL05"), None);   // wrong check
/// assert_eq!(parse_mx_curp("HEGG561327MVZRRL04"), None);   // invalid month
/// assert_eq!(parse_mx_curp("HEGG560427"),         None);   // too short
/// ```
pub fn parse_mx_curp(s: &str) -> Option<String> {
    let cleaned: String = s
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| c.to_uppercase().next().unwrap_or(c))
        .collect();
    if cleaned.chars().count() != 18 {
        return None;
    }
    let chars: Vec<char> = cleaned.chars().collect();
    let is_letter_or_n_tilde = |c: char| c.is_ascii_uppercase() || c == 'Ñ';
    if !chars[..4].iter().copied().all(is_letter_or_n_tilde) {
        return None;
    }
    if !chars[4..10].iter().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if chars[10] != 'H' && chars[10] != 'M' {
        return None;
    }
    if !chars[11..16].iter().copied().all(is_letter_or_n_tilde) {
        return None;
    }
    if !chars[16].is_ascii_alphanumeric() && chars[16] != 'Ñ' {
        return None;
    }
    if !chars[17].is_ascii_digit() {
        return None;
    }
    let yy: i32 = cleaned[4..6].parse().ok()?;
    let mm: u32 = cleaned[6..8].parse().ok()?;
    let dd: u32 = cleaned[8..10].parse().ok()?;
    let yyyy = if yy <= 29 { 2000 + yy } else { 1900 + yy };
    jiff::civil::Date::new(yyyy as i16, mm as i8, dd as i8).ok()?;
    let value = |c: char| -> Option<u32> {
        Some(match c {
            '0'..='9' => (c as u32) - ('0' as u32),
            'A'..='N' => 10 + ((c as u32) - ('A' as u32)),
            'Ñ' => 24,
            'O'..='Z' => 25 + ((c as u32) - ('O' as u32)),
            _ => return None,
        })
    };
    let mut sum: u32 = 0;
    for (i, &c) in chars.iter().enumerate().take(17) {
        sum += value(c)? * (18 - i as u32);
    }
    let expected = (10 - (sum % 10)) % 10;
    if u32::from(chars[17] as u8 - b'0') != expected {
        return None;
    }
    Some(cleaned)
}

/// Parse a New Zealand NHI Number (original 7-character format:
/// 3 letters + 4 digits, where the last digit is a Mod-11 check).
///
/// The letter values are: `A..Z` minus `I` and `O` (excluded because
/// they collide visually with `1` and `0`), assigned consecutively:
/// `A=1, B=2, C=3, D=4, E=5, F=6, G=7, H=8, J=9, K=10, L=11, M=12,
/// N=13, P=14, Q=15, R=16, S=17, T=18, U=19, V=20, W=21, X=22, Y=23,
/// Z=24`. The weighted sum applies weights `[7, 6, 5, 4, 3, 2]` to
/// the first six positions (3 letters + 3 digits); the remainder mod
/// 11 yields the expected check digit (`0` if remainder is `0`,
/// otherwise `11 - remainder`; if the result is `10` the NHI is
/// invalid because no single decimal digit can encode `10`).
///
/// The 2019 7-character alphanumeric NHI revision (3 letters + 2
/// digits + 2 letters) uses a different algorithm and is **not**
/// supported by this parser; calls fall through to `None` for the new
/// format. Consumers handling 2019-format NHIs SHOULD validate
/// upstream and pass the value through as a third-party identifier.
///
/// ```
/// use person_matcher::identifiers::parse_nz_nhi;
/// assert_eq!(parse_nz_nhi("ZAA0083"), Some("ZAA0083".to_string()));
/// assert_eq!(parse_nz_nhi("zaa0083"), Some("ZAA0083".to_string()));
/// assert_eq!(parse_nz_nhi("ZAA0082"), None);          // wrong check
/// assert_eq!(parse_nz_nhi("ZAI0083"), None);          // I excluded
/// assert_eq!(parse_nz_nhi("ZAA008"),  None);          // too short
/// ```
pub fn parse_nz_nhi(s: &str) -> Option<String> {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if cleaned.len() != 7 {
        return None;
    }
    let bytes = cleaned.as_bytes();
    for &b in &bytes[..3] {
        if !b.is_ascii_uppercase() || b == b'I' || b == b'O' {
            return None;
        }
    }
    for &b in &bytes[3..] {
        if !b.is_ascii_digit() {
            return None;
        }
    }
    let letter_value = |b: u8| -> u32 {
        let idx = u32::from(b - b'A') + 1;
        if b > b'O' {
            idx - 2
        } else if b > b'I' {
            idx - 1
        } else {
            idx
        }
    };
    const WEIGHTS: [u32; 6] = [7, 6, 5, 4, 3, 2];
    let mut sum: u32 = 0;
    for i in 0..3 {
        sum += letter_value(bytes[i]) * WEIGHTS[i];
    }
    for i in 0..3 {
        sum += u32::from(bytes[3 + i] - b'0') * WEIGHTS[3 + i];
    }
    let r = sum % 11;
    if r == 1 {
        return None;
    }
    let expected = if r == 0 { 0 } else { 11 - r };
    if u32::from(bytes[6] - b'0') != expected {
        return None;
    }
    Some(cleaned)
}

/// Parse a South Africa ID Number (13 digits, Luhn check digit + a
/// date-of-birth substring at positions 0..6).
///
/// The first 6 digits encode `YYMMDD`; the century is conventionally
/// inferred (`YY <= 29 → 20YY`, else `19YY`). The check digit at
/// position 12 is computed by the standard Luhn algorithm over all
/// 13 digits.
///
/// The remaining substrings (sequence at positions 6..10, citizenship
/// at position 10, and the legacy race indicator at position 11) are
/// intentionally NOT validated by this parser — they are demographic
/// information the person-matcher layer does not use.
///
/// ```
/// use person_matcher::identifiers::parse_za_id;
/// assert_eq!(parse_za_id("8001015009087"),   Some("8001015009087".to_string()));
/// assert_eq!(parse_za_id("800101 5009 087"), Some("8001015009087".to_string()));
/// assert_eq!(parse_za_id("8001015009088"),   None);  // wrong Luhn
/// assert_eq!(parse_za_id("8013015009087"),   None);  // invalid month
/// assert_eq!(parse_za_id("80010150090"),     None);  // too short
/// ```
pub fn parse_za_id(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 13 {
        return None;
    }
    let yy: i32 = digits[0..2].parse().ok()?;
    let mm: u32 = digits[2..4].parse().ok()?;
    let dd: u32 = digits[4..6].parse().ok()?;
    let yyyy = if yy <= 29 { 2000 + yy } else { 1900 + yy };
    jiff::civil::Date::new(yyyy as i16, mm as i8, dd as i8).ok()?;
    let bytes = digits.as_bytes();
    let mut sum: u32 = 0;
    // Standard Luhn: process right-to-left, double every second digit
    // starting from the second-to-last (i.e. positions 1, 3, 5, … from
    // the right). For a 13-digit ID this doubles positions 11, 9, 7,
    // 5, 3, 1 (counting from the left, 0-indexed).
    for i in 0..13 {
        let mut d = u32::from(bytes[12 - i] - b'0');
        if i % 2 == 1 {
            d *= 2;
            if d > 9 {
                d -= 9;
            }
        }
        sum += d;
    }
    if !sum.is_multiple_of(10) {
        return None;
    }
    Some(digits)
}

// ----------------------------------------------------------------------------
// Nine per-country passport-number format validators (T-28).
//
// These are pure format validators that consumers may call before
// constructing a `PassportBook`. They do NOT correspond to `Person`
// fields — passport-book numbers belong to the `PassportBook` model
// because they change with each renewal and a person may carry
// passports from multiple countries simultaneously.
// ----------------------------------------------------------------------------

/// Parse a Cyprus passport number.
///
/// Pre-2010 passports: letter `E` + 6 digits (e.g. `E123456`).
/// Biometric passports issued from 13 December 2010 onwards: letter `K`
/// + 8 digits (e.g. `K12345678`).
///
/// ```
/// use person_matcher::identifiers::parse_cy_passport;
/// assert_eq!(parse_cy_passport("E123456"),   Some("E123456".to_string()));
/// assert_eq!(parse_cy_passport("k12345678"), Some("K12345678".to_string()));
/// assert_eq!(parse_cy_passport("A123456"),   None);
/// assert_eq!(parse_cy_passport("E12345"),    None);  // too short
/// ```
pub fn parse_cy_passport(s: &str) -> Option<String> {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_uppercase();
    let chars: Vec<char> = cleaned.chars().collect();
    match (chars.first(), chars.len()) {
        (Some('E'), 7) if chars[1..].iter().all(|c| c.is_ascii_digit()) => Some(cleaned),
        (Some('K'), 9) if chars[1..].iter().all(|c| c.is_ascii_digit()) => Some(cleaned),
        _ => None,
    }
}

/// Parse a Czech Republic passport number.
///
/// Usually an 8-digit number; per the TSV it may be longer. The parser
/// accepts 8 to 12 ASCII digits.
///
/// ```
/// use person_matcher::identifiers::parse_cz_passport;
/// assert_eq!(parse_cz_passport("12345678"), Some("12345678".to_string()));
/// assert_eq!(parse_cz_passport("123-456-78"), Some("12345678".to_string()));
/// assert_eq!(parse_cz_passport("123"), None);  // too short
/// ```
pub fn parse_cz_passport(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if (8..=12).contains(&digits.len()) {
        Some(digits)
    } else {
        None
    }
}

/// Parse a Liechtenstein passport number. 1 letter + 5 digits (e.g. `R00536`).
///
/// ```
/// use person_matcher::identifiers::parse_li_passport;
/// assert_eq!(parse_li_passport("R00536"), Some("R00536".to_string()));
/// assert_eq!(parse_li_passport("r00536"), Some("R00536".to_string()));
/// assert_eq!(parse_li_passport("123456"), None);
/// ```
pub fn parse_li_passport(s: &str) -> Option<String> {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_uppercase();
    if cleaned.len() != 6 {
        return None;
    }
    let chars: Vec<char> = cleaned.chars().collect();
    if !chars[0].is_ascii_alphabetic() {
        return None;
    }
    if !chars[1..].iter().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(cleaned)
}

/// Parse a Lithuania passport number. 8 ASCII digits (also used on the
/// national ID card).
///
/// ```
/// use person_matcher::identifiers::parse_lt_passport;
/// assert_eq!(parse_lt_passport("12345678"), Some("12345678".to_string()));
/// assert_eq!(parse_lt_passport("1234567"), None);
/// ```
pub fn parse_lt_passport(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() == 8 {
        Some(digits)
    } else {
        None
    }
}

/// Parse a Malta passport number. 7 ASCII digits.
///
/// ```
/// use person_matcher::identifiers::parse_mt_passport;
/// assert_eq!(parse_mt_passport("1234567"), Some("1234567".to_string()));
/// assert_eq!(parse_mt_passport("123"), None);
/// ```
pub fn parse_mt_passport(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() == 7 {
        Some(digits)
    } else {
        None
    }
}

/// Parse a Netherlands passport number. Same shape as the NL ID card
/// (see [`parse_nl_id`]).
///
/// ```
/// use person_matcher::identifiers::parse_nl_passport;
/// assert_eq!(parse_nl_passport("AB1234567"), Some("AB1234567".to_string()));
/// assert_eq!(parse_nl_passport("AO1234567"), None);  // O is banned
/// ```
pub fn parse_nl_passport(s: &str) -> Option<String> {
    parse_nl_id(s)
}

/// Parse a Portugal passport number. 1 letter + 6 digits.
///
/// ```
/// use person_matcher::identifiers::parse_pt_passport;
/// assert_eq!(parse_pt_passport("A123456"), Some("A123456".to_string()));
/// assert_eq!(parse_pt_passport("AA12345"), None);
/// ```
pub fn parse_pt_passport(s: &str) -> Option<String> {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_uppercase();
    if cleaned.len() != 7 {
        return None;
    }
    let chars: Vec<char> = cleaned.chars().collect();
    if !chars[0].is_ascii_alphabetic() {
        return None;
    }
    if !chars[1..].iter().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(cleaned)
}

/// Parse a Romania passport number. 2 letters + 6 digits.
///
/// ```
/// use person_matcher::identifiers::parse_ro_passport;
/// assert_eq!(parse_ro_passport("AB123456"), Some("AB123456".to_string()));
/// assert_eq!(parse_ro_passport("A1234567"), None);
/// ```
pub fn parse_ro_passport(s: &str) -> Option<String> {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_uppercase();
    if cleaned.len() != 8 {
        return None;
    }
    let chars: Vec<char> = cleaned.chars().collect();
    if !chars[..2].iter().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    if !chars[2..].iter().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(cleaned)
}

/// Parse a Slovakia passport number. 2 letters + 7 digits.
///
/// ```
/// use person_matcher::identifiers::parse_sk_passport;
/// assert_eq!(parse_sk_passport("AB1234567"), Some("AB1234567".to_string()));
/// assert_eq!(parse_sk_passport("AB12345"), None);
/// ```
pub fn parse_sk_passport(s: &str) -> Option<String> {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_uppercase();
    if cleaned.len() != 9 {
        return None;
    }
    let chars: Vec<char> = cleaned.chars().collect();
    if !chars[..2].iter().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    if !chars[2..].iter().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- parse_united_kingdom_national_health_service_number ----------

    #[test]
    fn united_kingdom_national_health_service_number_compact_form_parses() {
        assert_eq!(parse_united_kingdom_national_health_service_number("9434765919"), Some("9434765919".into()));
    }

    #[test]
    fn united_kingdom_national_health_service_number_spaced_form_parses_to_same_canonical() {
        assert_eq!(
            parse_united_kingdom_national_health_service_number("943 476 5919"),
            parse_united_kingdom_national_health_service_number("9434765919"),
        );
    }

    #[test]
    fn united_kingdom_national_health_service_number_rejects_letters_and_short_input() {
        assert_eq!(parse_united_kingdom_national_health_service_number("ABCDEFGHIJ"), None);
        assert_eq!(parse_united_kingdom_national_health_service_number("123"), None);
        assert_eq!(parse_united_kingdom_national_health_service_number(""), None);
    }

    // ---------- parse_fr_nir ----------

    #[test]
    fn fr_nir_round_trip_for_a_constructed_valid_value() {
        // Body 1801275123456 → key = 97 - (N mod 97) = 42. Verified by parse.
        let valid = "180127512345642";
        assert_eq!(parse_fr_nir(valid), Some(valid.into()));
    }

    #[test]
    fn fr_nir_whitespace_is_tolerated() {
        assert_eq!(
            parse_fr_nir("1 80 12 75 123 456 42"),
            Some("180127512345642".into()),
        );
    }

    #[test]
    fn fr_nir_rejects_wrong_check_key() {
        assert_eq!(parse_fr_nir("180127512345699"), None);
    }

    #[test]
    fn fr_nir_rejects_wrong_length() {
        assert_eq!(parse_fr_nir("12345"), None);
        assert_eq!(parse_fr_nir("1234567890123456"), None); // 16 chars
        assert_eq!(parse_fr_nir(""), None);
    }

    #[test]
    fn fr_nir_rejects_letters_in_digit_positions() {
        assert_eq!(parse_fr_nir("A80127512345642"), None);
    }

    #[test]
    fn fr_nir_handles_corsica_2a() {
        let body = "184032A001234";
        let numeric: u64 = "1840319001234".parse().unwrap();
        let key = 97 - (numeric % 97);
        let nir = format!("{body}{key:02}");
        assert_eq!(parse_fr_nir(&nir), Some(nir.clone()));
    }

    #[test]
    fn fr_nir_handles_corsica_2b() {
        let body = "184032B001234";
        let numeric: u64 = "1840318001234".parse().unwrap();
        let key = 97 - (numeric % 97);
        let nir = format!("{body}{key:02}");
        assert_eq!(parse_fr_nir(&nir), Some(nir.clone()));
    }

    #[test]
    fn fr_nir_canonical_form_is_uppercased() {
        let body = "184032a001234";
        let numeric: u64 = "1840319001234".parse().unwrap();
        let key = 97 - (numeric % 97);
        let nir = format!("{body}{key:02}");
        let canonical = nir.to_uppercase();
        assert_eq!(parse_fr_nir(&nir), Some(canonical));
    }

    // ---------- parse_es_tsi ----------

    #[test]
    fn es_tsi_canonical_cip_sns_parses() {
        assert_eq!(
            parse_es_tsi("ABCD123456XY1234"),
            Some("ABCD123456XY1234".into()),
        );
    }

    #[test]
    fn es_tsi_whitespace_and_hyphens_stripped() {
        assert_eq!(
            parse_es_tsi("abcd 123 456-xy1234"),
            Some("ABCD123456XY1234".into()),
        );
    }

    #[test]
    fn es_tsi_rejects_too_short_or_too_long() {
        assert_eq!(parse_es_tsi("ABC123"), None);
        assert_eq!(parse_es_tsi("ABCDEF123456XY12345678"), None);
    }

    #[test]
    fn es_tsi_rejects_non_alphanumerics() {
        assert_eq!(parse_es_tsi("ABC@123!XYZ"), None);
    }

    #[test]
    fn es_tsi_rejects_non_ascii() {
        assert_eq!(parse_es_tsi("ABCDÑ12345XYZ"), None);
    }

    // ---------- parse_ie_ihi ----------

    #[test]
    fn ie_ihi_seven_digits_parses() {
        assert_eq!(parse_ie_ihi("1234567"), Some("1234567".into()));
    }

    #[test]
    fn ie_ihi_punctuation_and_spaces_stripped() {
        assert_eq!(parse_ie_ihi("123 4567"), Some("1234567".into()));
        assert_eq!(parse_ie_ihi("123-45-67"), Some("1234567".into()));
    }

    #[test]
    fn ie_ihi_rejects_wrong_digit_count() {
        assert_eq!(parse_ie_ihi("123456"), None);
        assert_eq!(parse_ie_ihi("12345678"), None);
        assert_eq!(parse_ie_ihi(""), None);
    }

    #[test]
    fn ie_ihi_rejects_when_no_digits_present() {
        assert_eq!(parse_ie_ihi("ABCDEFG"), None);
    }

    // ---------- parse_uk_hc_number ----------

    #[test]
    fn uk_hc_number_matches_united_kingdom_national_health_service_number_semantics() {
        assert_eq!(
            parse_uk_hc_number("9434765919"),
            parse_united_kingdom_national_health_service_number("9434765919"),
        );
        assert_eq!(
            parse_uk_hc_number("943 476 5919"),
            parse_united_kingdom_national_health_service_number("943 476 5919"),
        );
    }

    #[test]
    fn uk_hc_number_rejects_letters() {
        assert_eq!(parse_uk_hc_number("ABCDEFGHIJ"), None);
    }

    // ---------- parse_us_ssn ----------

    #[test]
    fn us_ssn_canonical_compact_form_parses() {
        assert_eq!(parse_us_ssn("123456789"), Some("123456789".into()));
    }

    #[test]
    fn us_ssn_hyphenated_form_parses_to_same_canonical() {
        assert_eq!(parse_us_ssn("123-45-6789"), parse_us_ssn("123456789"),);
    }

    #[test]
    fn us_ssn_whitespace_variants_canonicalise_identically() {
        assert_eq!(parse_us_ssn("123 45 6789"), Some("123456789".into()),);
        assert_eq!(parse_us_ssn(" 123  45 6789 "), Some("123456789".into()),);
    }

    #[test]
    fn us_ssn_rejects_invalid_area_numbers() {
        assert_eq!(parse_us_ssn("000-12-3456"), None);
        assert_eq!(parse_us_ssn("666-12-3456"), None);
        assert_eq!(parse_us_ssn("900-12-3456"), None);
        assert_eq!(parse_us_ssn("987-65-4321"), None); // 987 is in 900..=999
        assert_eq!(parse_us_ssn("999-99-9999"), None);
    }

    #[test]
    fn us_ssn_accepts_boundary_areas() {
        // 001 and 899 are the lowest and highest valid area numbers.
        assert_eq!(parse_us_ssn("001-23-4567"), Some("001234567".into()));
        assert_eq!(parse_us_ssn("899-23-4567"), Some("899234567".into()));
        // 665 just below the 666 carve-out; 667 just above.
        assert_eq!(parse_us_ssn("665-23-4567"), Some("665234567".into()));
        assert_eq!(parse_us_ssn("667-23-4567"), Some("667234567".into()));
    }

    #[test]
    fn us_ssn_rejects_zero_group() {
        assert_eq!(parse_us_ssn("123-00-4567"), None);
    }

    #[test]
    fn us_ssn_rejects_zero_serial() {
        assert_eq!(parse_us_ssn("123-45-0000"), None);
    }

    #[test]
    fn us_ssn_rejects_wrong_length() {
        assert_eq!(parse_us_ssn("12345"), None);
        assert_eq!(parse_us_ssn("1234567890"), None);
        assert_eq!(parse_us_ssn(""), None);
    }

    #[test]
    fn us_ssn_rejects_letters() {
        assert_eq!(parse_us_ssn("ABC-DE-FGHI"), None);
        assert_eq!(parse_us_ssn("ABCDEFGHI"), None);
    }

    #[test]
    fn us_ssn_strips_arbitrary_punctuation() {
        assert_eq!(parse_us_ssn("(123).45.6789"), Some("123456789".into()),);
    }

    // ---------- parse_de_kvnr ----------

    #[test]
    fn de_kvnr_canonical_form_parses() {
        assert_eq!(parse_de_kvnr("A123456780"), Some("A123456780".into()));
    }

    #[test]
    fn de_kvnr_accepts_lowercase_letter_canonicalises_to_upper() {
        assert_eq!(parse_de_kvnr("a123456780"), Some("A123456780".into()));
    }

    #[test]
    fn de_kvnr_accepts_internal_whitespace() {
        assert_eq!(parse_de_kvnr("A 123 456 780"), Some("A123456780".into()));
    }

    #[test]
    fn de_kvnr_second_valid_vector() {
        assert_eq!(parse_de_kvnr("B987654320"), Some("B987654320".into()));
    }

    #[test]
    fn de_kvnr_rejects_wrong_check_digit() {
        assert_eq!(parse_de_kvnr("A123456789"), None);
    }

    #[test]
    fn de_kvnr_rejects_missing_letter() {
        assert_eq!(parse_de_kvnr("1234567890"), None);
    }

    #[test]
    fn de_kvnr_rejects_wrong_length() {
        assert_eq!(parse_de_kvnr("A12345"), None);
        assert_eq!(parse_de_kvnr("A1234567890"), None);
        assert_eq!(parse_de_kvnr(""), None);
    }

    #[test]
    fn de_kvnr_rejects_non_digit_in_body() {
        assert_eq!(parse_de_kvnr("A12345A780"), None);
    }

    // ---------- parse_it_cf ----------

    #[test]
    fn it_cf_canonical_form_parses() {
        assert_eq!(
            parse_it_cf("RSSMRA85T10A562S"),
            Some("RSSMRA85T10A562S".into()),
        );
    }

    #[test]
    fn it_cf_accepts_lowercase_and_whitespace() {
        assert_eq!(
            parse_it_cf("rss mra 85t 10a 562s"),
            Some("RSSMRA85T10A562S".into()),
        );
    }

    #[test]
    fn it_cf_second_valid_vector() {
        assert_eq!(
            parse_it_cf("MNRMRC75H17H501I"),
            Some("MNRMRC75H17H501I".into()),
        );
    }

    #[test]
    fn it_cf_rejects_wrong_check_character() {
        assert_eq!(parse_it_cf("RSSMRA85T10A562X"), None);
    }

    #[test]
    fn it_cf_rejects_wrong_length() {
        assert_eq!(parse_it_cf("RSSMRA85T10A562"), None);
        assert_eq!(parse_it_cf("RSSMRA85T10A562SS"), None);
        assert_eq!(parse_it_cf(""), None);
    }

    #[test]
    fn it_cf_rejects_non_alphanumeric() {
        assert_eq!(parse_it_cf("RSSMRA85T10A562!"), None);
        assert_eq!(parse_it_cf("RSSMRA-85T-10A562S"), None);
    }

    // ---------- parse_nl_bsn ----------

    #[test]
    fn nl_bsn_canonical_form_parses() {
        assert_eq!(parse_nl_bsn("111222333"), Some("111222333".into()));
    }

    #[test]
    fn nl_bsn_second_valid_vector() {
        assert_eq!(parse_nl_bsn("123456782"), Some("123456782".into()));
    }

    #[test]
    fn nl_bsn_strips_separators() {
        assert_eq!(parse_nl_bsn("111 222 333"), Some("111222333".into()));
        assert_eq!(parse_nl_bsn("111-222-333"), Some("111222333".into()));
    }

    #[test]
    fn nl_bsn_rejects_wrong_eleven_test() {
        assert_eq!(parse_nl_bsn("111222334"), None);
    }

    #[test]
    fn nl_bsn_rejects_all_zeros() {
        assert_eq!(parse_nl_bsn("000000000"), None);
    }

    #[test]
    fn nl_bsn_rejects_wrong_length() {
        assert_eq!(parse_nl_bsn("12345"), None);
        assert_eq!(parse_nl_bsn("1234567890"), None);
        assert_eq!(parse_nl_bsn(""), None);
    }

    #[test]
    fn nl_bsn_rejects_letters() {
        assert_eq!(parse_nl_bsn("ABCDEFGHI"), None);
    }

    // ---------- parse_se_personnummer ----------

    #[test]
    fn se_pnr_ten_digit_form_parses() {
        assert_eq!(
            parse_se_personnummer("4603243850"),
            Some("4603243850".into()),
        );
    }

    #[test]
    fn se_pnr_with_separator_canonicalises_to_ten_digit() {
        assert_eq!(
            parse_se_personnummer("460324-3850"),
            Some("4603243850".into()),
        );
        assert_eq!(
            parse_se_personnummer("460324+3850"),
            Some("4603243850".into()),
        );
    }

    #[test]
    fn se_pnr_twelve_digit_form_preserves_century() {
        assert_eq!(
            parse_se_personnummer("19460324-3850"),
            Some("194603243850".into()),
        );
        assert_eq!(
            parse_se_personnummer("194603243850"),
            Some("194603243850".into()),
        );
    }

    #[test]
    fn se_pnr_second_valid_vector() {
        assert_eq!(
            parse_se_personnummer("8112310092"),
            Some("8112310092".into()),
        );
    }

    #[test]
    fn se_pnr_rejects_wrong_luhn() {
        assert_eq!(parse_se_personnummer("4603243851"), None);
    }

    #[test]
    fn se_pnr_rejects_wrong_length() {
        assert_eq!(parse_se_personnummer("12345"), None);
        assert_eq!(parse_se_personnummer("12345678901"), None);
        assert_eq!(parse_se_personnummer(""), None);
    }

    #[test]
    fn se_pnr_rejects_letters() {
        assert_eq!(parse_se_personnummer("ABCDEFGHIJ"), None);
    }

    // ---------- parse_au_ihi ----------

    #[test]
    fn au_ihi_canonical_form_parses() {
        assert_eq!(
            parse_au_ihi("8003601234567894"),
            Some("8003601234567894".into()),
        );
    }

    #[test]
    fn au_ihi_strips_whitespace() {
        assert_eq!(
            parse_au_ihi("8003 6012 3456 7894"),
            Some("8003601234567894".into()),
        );
    }

    #[test]
    fn au_ihi_second_valid_vector() {
        assert_eq!(
            parse_au_ihi("8003619876543213"),
            Some("8003619876543213".into()),
        );
    }

    #[test]
    fn au_ihi_rejects_wrong_luhn() {
        assert_eq!(parse_au_ihi("8003601234567890"), None);
    }

    #[test]
    fn au_ihi_rejects_wrong_length() {
        assert_eq!(parse_au_ihi("12345"), None);
        assert_eq!(parse_au_ihi("80036012345678941"), None);
        assert_eq!(parse_au_ihi(""), None);
    }

    #[test]
    fn au_ihi_rejects_letters() {
        assert_eq!(parse_au_ihi("ABCDEFGHIJKLMNOP"), None);
    }

    // ---------- parse_uk_chi_number ----------

    #[test]
    fn uk_chi_canonical_form_parses() {
        assert_eq!(parse_uk_chi_number("0101701233"), Some("0101701233".into()),);
    }

    #[test]
    fn uk_chi_strips_whitespace() {
        assert_eq!(
            parse_uk_chi_number("010 170 1233"),
            Some("0101701233".into()),
        );
    }

    #[test]
    fn uk_chi_second_valid_vector() {
        assert_eq!(parse_uk_chi_number("0101701241"), Some("0101701241".into()),);
    }

    #[test]
    fn uk_chi_rejects_wrong_check_digit() {
        assert_eq!(parse_uk_chi_number("0101701234"), None);
    }

    #[test]
    fn uk_chi_rejects_wrong_length() {
        assert_eq!(parse_uk_chi_number("12345"), None);
        assert_eq!(parse_uk_chi_number("01017012339"), None);
        assert_eq!(parse_uk_chi_number(""), None);
    }

    #[test]
    fn uk_chi_rejects_letters() {
        assert_eq!(parse_uk_chi_number("ABCDEFGHIJ"), None);
    }

    // ----------------------------------------------------------------------
    // Eighteen additional national personal identifiers (T-27).
    // ----------------------------------------------------------------------

    // ---------- parse_be_nn ----------

    #[test]
    fn be_nn_canonical_form_parses() {
        assert_eq!(parse_be_nn("80010100107"), Some("80010100107".into()));
    }
    #[test]
    fn be_nn_strips_punctuation() {
        assert_eq!(parse_be_nn("80.01.01-001.07"), Some("80010100107".into()),);
    }
    #[test]
    fn be_nn_rejects_wrong_check() {
        assert_eq!(parse_be_nn("80010100100"), None);
    }
    #[test]
    fn be_nn_rejects_wrong_length() {
        assert_eq!(parse_be_nn("12345"), None);
        assert_eq!(parse_be_nn(""), None);
    }

    // ---------- parse_bg_egn ----------

    #[test]
    fn bg_egn_canonical_form_parses() {
        assert_eq!(parse_bg_egn("8001010013"), Some("8001010013".into()));
    }
    #[test]
    fn bg_egn_rejects_wrong_check() {
        assert_eq!(parse_bg_egn("8001010014"), None);
    }
    #[test]
    fn bg_egn_rejects_wrong_length() {
        assert_eq!(parse_bg_egn("80010100"), None);
        assert_eq!(parse_bg_egn(""), None);
    }

    // ---------- parse_cz_rc ----------

    #[test]
    fn cz_rc_ten_digit_divisible_by_eleven() {
        assert_eq!(parse_cz_rc("8001150014"), Some("8001150014".into()));
    }
    #[test]
    fn cz_rc_nine_digit_pre_1954_accepted_as_is() {
        assert_eq!(parse_cz_rc("800115001"), Some("800115001".into()));
    }
    #[test]
    fn cz_rc_rejects_wrong_check() {
        assert_eq!(parse_cz_rc("8001150015"), None);
    }
    #[test]
    fn cz_rc_rejects_bad_length() {
        assert_eq!(parse_cz_rc("12345"), None);
        assert_eq!(parse_cz_rc("12345678901"), None);
    }

    // ---------- parse_dk_cpr ----------

    #[test]
    fn dk_cpr_canonical_parses() {
        assert_eq!(parse_dk_cpr("1501801234"), Some("1501801234".into()));
    }
    #[test]
    fn dk_cpr_strips_separator() {
        assert_eq!(parse_dk_cpr("150180-1234"), Some("1501801234".into()));
    }
    #[test]
    fn dk_cpr_rejects_bad_length() {
        assert_eq!(parse_dk_cpr("12345"), None);
        assert_eq!(parse_dk_cpr(""), None);
    }

    // ---------- parse_ee_ik ----------

    #[test]
    fn ee_ik_canonical_form_parses() {
        assert_eq!(parse_ee_ik("48001150011"), Some("48001150011".into()));
    }
    #[test]
    fn ee_ik_rejects_wrong_check() {
        assert_eq!(parse_ee_ik("48001150012"), None);
    }
    #[test]
    fn ee_ik_rejects_bad_length() {
        assert_eq!(parse_ee_ik("4800115001"), None);
    }

    // ---------- parse_es_dni ----------

    #[test]
    fn es_dni_canonical_form_parses() {
        assert_eq!(parse_es_dni("12345678Z"), Some("12345678Z".into()));
    }
    #[test]
    fn es_dni_rejects_wrong_letter() {
        assert_eq!(parse_es_dni("12345678A"), None);
    }
    #[test]
    fn es_dni_lowercase_letter_canonicalises_upper() {
        assert_eq!(parse_es_dni("12345678z"), Some("12345678Z".into()));
    }
    #[test]
    fn es_dni_handles_nie_prefix_x() {
        // NIE X1234567L → number is "01234567" mod 23 = (01234567 % 23).
        // 1234567 mod 23: 23 × 53676 = 1234548. 1234567 - 1234548 = 19.
        // LETTERS[19] = 'L'. So "X1234567L" is valid.
        assert_eq!(parse_es_dni("X1234567L"), Some("X1234567L".into()));
    }

    // ---------- parse_fi_hetu ----------

    #[test]
    fn fi_hetu_canonical_form_parses() {
        assert_eq!(parse_fi_hetu("150180-999B"), Some("150180-999B".into()));
    }
    #[test]
    fn fi_hetu_rejects_wrong_check() {
        assert_eq!(parse_fi_hetu("150180-999C"), None);
    }
    #[test]
    fn fi_hetu_rejects_bad_length() {
        assert_eq!(parse_fi_hetu("12345"), None);
    }

    // ---------- parse_hr_oib ----------

    #[test]
    fn hr_oib_canonical_form_parses() {
        assert_eq!(parse_hr_oib("12345678903"), Some("12345678903".into()));
    }
    #[test]
    fn hr_oib_rejects_wrong_check() {
        assert_eq!(parse_hr_oib("12345678901"), None);
    }
    #[test]
    fn hr_oib_rejects_bad_length() {
        assert_eq!(parse_hr_oib("123456789"), None);
    }

    // ---------- parse_is_kt ----------

    #[test]
    fn is_kt_canonical_form_parses() {
        assert_eq!(parse_is_kt("1501802529"), Some("1501802529".into()));
    }
    #[test]
    fn is_kt_rejects_wrong_check() {
        assert_eq!(parse_is_kt("1501802539"), None);
    }
    #[test]
    fn is_kt_rejects_bad_length() {
        assert_eq!(parse_is_kt("12345"), None);
    }

    // ---------- parse_lt_ak ----------

    #[test]
    fn lt_ak_canonical_form_parses() {
        assert_eq!(parse_lt_ak("48001150011"), Some("48001150011".into()));
    }
    #[test]
    fn lt_ak_rejects_wrong_check() {
        assert_eq!(parse_lt_ak("48001150012"), None);
    }

    // ---------- parse_lv_pk ----------

    #[test]
    fn lv_pk_canonical_form_parses() {
        assert_eq!(parse_lv_pk("15018010007"), Some("15018010007".into()));
    }
    #[test]
    fn lv_pk_rejects_wrong_check() {
        assert_eq!(parse_lv_pk("15018010008"), None);
    }
    #[test]
    fn lv_pk_rejects_bad_length() {
        assert_eq!(parse_lv_pk("1501801000"), None);
    }

    // ---------- parse_mt_id ----------

    #[test]
    fn mt_id_canonical_form_parses() {
        assert_eq!(parse_mt_id("1234567M"), Some("1234567M".into()));
    }
    #[test]
    fn mt_id_accepts_all_valid_letters() {
        for letter in ['M', 'G', 'A', 'P', 'L', 'H', 'B', 'Z'] {
            let s = format!("1234567{letter}");
            assert!(parse_mt_id(&s).is_some(), "letter {letter} should be valid");
        }
    }
    #[test]
    fn mt_id_rejects_invalid_letter() {
        assert_eq!(parse_mt_id("1234567X"), None);
        assert_eq!(parse_mt_id("1234567K"), None);
    }
    #[test]
    fn mt_id_rejects_bad_length() {
        assert_eq!(parse_mt_id("12345M"), None);
    }

    // ---------- parse_no_fnr ----------

    #[test]
    fn no_fnr_canonical_form_parses() {
        assert_eq!(parse_no_fnr("15018012399"), Some("15018012399".into()));
    }
    #[test]
    fn no_fnr_rejects_wrong_check() {
        assert_eq!(parse_no_fnr("15018012390"), None);
        assert_eq!(parse_no_fnr("15018012398"), None);
    }
    #[test]
    fn no_fnr_rejects_bad_length() {
        assert_eq!(parse_no_fnr("12345"), None);
    }

    // ---------- parse_pl_pesel ----------

    #[test]
    fn pl_pesel_canonical_form_parses() {
        assert_eq!(parse_pl_pesel("80011500014"), Some("80011500014".into()));
    }
    #[test]
    fn pl_pesel_rejects_wrong_check() {
        assert_eq!(parse_pl_pesel("80011500015"), None);
    }
    #[test]
    fn pl_pesel_rejects_bad_length() {
        assert_eq!(parse_pl_pesel("1234"), None);
    }

    // ---------- parse_ro_cnp ----------

    #[test]
    fn ro_cnp_canonical_form_parses() {
        assert_eq!(parse_ro_cnp("1800115400012"), Some("1800115400012".into()));
    }
    #[test]
    fn ro_cnp_rejects_wrong_check() {
        assert_eq!(parse_ro_cnp("1800115400015"), None);
    }
    #[test]
    fn ro_cnp_rejects_bad_length() {
        assert_eq!(parse_ro_cnp("180011540001"), None);
    }

    // ---------- parse_si_emso ----------

    #[test]
    fn si_emso_canonical_form_parses() {
        assert_eq!(parse_si_emso("1501980500015"), Some("1501980500015".into()));
    }
    #[test]
    fn si_emso_rejects_wrong_check() {
        assert_eq!(parse_si_emso("1501980500014"), None);
    }

    // ---------- parse_sk_rc ----------

    #[test]
    fn sk_rc_canonical_form_parses() {
        assert_eq!(parse_sk_rc("8051150019"), Some("8051150019".into()));
    }
    #[test]
    fn sk_rc_rejects_wrong_check() {
        assert_eq!(parse_sk_rc("8051150010"), None);
    }

    // ---------- parse_uk_nino ----------

    #[test]
    fn uk_nino_canonical_form_parses() {
        assert_eq!(parse_uk_nino("AB123456A"), Some("AB123456A".into()));
    }
    #[test]
    fn uk_nino_accepts_lowercase_and_whitespace() {
        assert_eq!(parse_uk_nino("ab 12 34 56 a"), Some("AB123456A".into()),);
    }
    #[test]
    fn uk_nino_rejects_banned_first_letter() {
        for ch in ['D', 'F', 'I', 'Q', 'U', 'V'] {
            let s = format!("{ch}A123456A");
            assert!(parse_uk_nino(&s).is_none(), "letter {ch} should be banned");
        }
    }
    #[test]
    fn uk_nino_rejects_banned_admin_prefix() {
        for prefix in ["OO", "CR", "FY", "MW", "NC", "PP", "PZ", "TN"] {
            let s = format!("{prefix}123456A");
            assert!(
                parse_uk_nino(&s).is_none(),
                "prefix {prefix} should be banned"
            );
        }
    }
    #[test]
    fn uk_nino_rejects_bad_suffix() {
        for ch in ['E', 'F', 'X', 'Z'] {
            let s = format!("AB123456{ch}");
            assert!(parse_uk_nino(&s).is_none(), "suffix {ch} should be invalid");
        }
    }
    #[test]
    fn uk_nino_rejects_bad_length() {
        assert_eq!(parse_uk_nino("AB12345A"), None);
    }

    // ----------------------------------------------------------------------
    // T-28: Five additional personal identifiers.
    // ----------------------------------------------------------------------

    // ---------- parse_gr_dss ----------

    #[test]
    fn gr_dss_canonical_form_parses() {
        assert_eq!(parse_gr_dss("1234567890"), Some("1234567890".into()));
    }
    #[test]
    fn gr_dss_strips_punctuation() {
        assert_eq!(parse_gr_dss("12 34-56 78 90"), Some("1234567890".into()));
    }
    #[test]
    fn gr_dss_rejects_bad_length() {
        assert_eq!(parse_gr_dss("12345"), None);
        assert_eq!(parse_gr_dss("12345678901"), None);
        assert_eq!(parse_gr_dss(""), None);
    }
    #[test]
    fn gr_dss_rejects_letters() {
        assert_eq!(parse_gr_dss("ABCDEFGHIJ"), None);
    }

    // ---------- parse_li_id ----------

    #[test]
    fn li_id_eight_digit_form_parses() {
        assert_eq!(parse_li_id("ID12345678"), Some("ID12345678".into()));
    }
    #[test]
    fn li_id_nine_digit_example_from_spec_parses() {
        assert_eq!(parse_li_id("ID022143586"), Some("ID022143586".into()));
    }
    #[test]
    fn li_id_lowercase_letters_uppercased() {
        assert_eq!(parse_li_id("id12345678"), Some("ID12345678".into()));
    }
    #[test]
    fn li_id_rejects_missing_letters() {
        assert_eq!(parse_li_id("1234567890"), None);
        assert_eq!(parse_li_id("I12345678"), None); // only one leading letter
    }
    #[test]
    fn li_id_rejects_bad_length() {
        assert_eq!(parse_li_id(""), None);
        assert_eq!(parse_li_id("ID1234"), None);
        assert_eq!(parse_li_id("ID123456789012"), None);
    }

    // ---------- parse_nl_id ----------

    #[test]
    fn nl_id_canonical_form_parses() {
        assert_eq!(parse_nl_id("AB1234567"), Some("AB1234567".into()));
    }
    #[test]
    fn nl_id_lowercase_and_whitespace_canonicalise() {
        assert_eq!(parse_nl_id("ab 12 34 567"), Some("AB1234567".into()));
    }
    #[test]
    fn nl_id_rejects_letter_o_in_disallowed_positions() {
        assert_eq!(parse_nl_id("AO1234567"), None);
        assert_eq!(parse_nl_id("OB1234567"), None);
        assert_eq!(parse_nl_id("ABO234567"), None);
    }
    #[test]
    fn nl_id_allows_digit_zero() {
        assert_eq!(parse_nl_id("AB0234567"), Some("AB0234567".into()));
    }
    #[test]
    fn nl_id_rejects_bad_shape() {
        assert_eq!(parse_nl_id("12345AB67"), None);
        assert_eq!(parse_nl_id("AB12345AB"), None);
        assert_eq!(parse_nl_id(""), None);
    }

    // ---------- parse_pl_nip ----------

    #[test]
    fn pl_nip_canonical_form_parses() {
        assert_eq!(parse_pl_nip("1234567802"), Some("1234567802".into()));
    }
    #[test]
    fn pl_nip_strips_separators() {
        assert_eq!(parse_pl_nip("123-456-78-02"), Some("1234567802".into()));
    }
    #[test]
    fn pl_nip_rejects_wrong_check() {
        assert_eq!(parse_pl_nip("1234567803"), None);
    }
    #[test]
    fn pl_nip_rejects_check_value_ten_per_spec() {
        // For "123456789" body the weighted sum mod 11 is 10, which the
        // Polish NIP spec defines as invalid.
        assert_eq!(parse_pl_nip("1234567890"), None);
    }
    #[test]
    fn pl_nip_rejects_bad_length() {
        assert_eq!(parse_pl_nip("12345"), None);
    }

    // ---------- parse_pt_nif ----------

    #[test]
    fn pt_nif_canonical_form_parses() {
        assert_eq!(parse_pt_nif("123456789"), Some("123456789".into()));
    }
    #[test]
    fn pt_nif_rejects_wrong_check() {
        assert_eq!(parse_pt_nif("123456780"), None);
    }
    #[test]
    fn pt_nif_rejects_bad_length() {
        assert_eq!(parse_pt_nif("12345"), None);
        assert_eq!(parse_pt_nif("1234567890"), None);
    }

    // ----------------------------------------------------------------------
    // T-17.1: Seven next-batch national identifier schemes.
    // ----------------------------------------------------------------------

    // ---------- parse_br_cpf ----------
    #[test]
    fn br_cpf_canonical_form_parses() {
        assert_eq!(parse_br_cpf("12345678909"), Some("12345678909".into()));
    }
    #[test]
    fn br_cpf_formatted_input_strips_punctuation() {
        assert_eq!(parse_br_cpf("123.456.789-09"), Some("12345678909".into()));
    }
    #[test]
    fn br_cpf_rejects_wrong_check() {
        assert_eq!(parse_br_cpf("12345678900"), None);
    }
    #[test]
    fn br_cpf_rejects_all_equal_sequences() {
        for d in '0'..='9' {
            let s: String = std::iter::repeat_n(d, 11).collect();
            assert_eq!(parse_br_cpf(&s), None, "{s}");
        }
    }
    #[test]
    fn br_cpf_rejects_bad_length() {
        assert_eq!(parse_br_cpf("1234567890"), None);
        assert_eq!(parse_br_cpf("123456789090"), None);
    }
    #[test]
    fn br_cpf_rejects_non_digit_only_input() {
        assert_eq!(parse_br_cpf("abcdefghijk"), None);
    }

    // ---------- parse_cn_rrn ----------
    #[test]
    fn cn_rrn_canonical_form_parses() {
        assert_eq!(
            parse_cn_rrn("11010519491231002X"),
            Some("11010519491231002X".into()),
        );
    }
    #[test]
    fn cn_rrn_uppercases_lowercase_x() {
        assert_eq!(
            parse_cn_rrn("11010519491231002x"),
            Some("11010519491231002X".into()),
        );
    }
    #[test]
    fn cn_rrn_rejects_wrong_check_char() {
        assert_eq!(parse_cn_rrn("11010519491231002Y"), None);
        assert_eq!(parse_cn_rrn("110105194912310020"), None);
    }
    #[test]
    fn cn_rrn_rejects_invalid_date_substring() {
        assert_eq!(parse_cn_rrn("11010513491231002X"), None);
        assert_eq!(parse_cn_rrn("110105194913320002X"), None);
    }
    #[test]
    fn cn_rrn_rejects_bad_length() {
        assert_eq!(parse_cn_rrn("11010519491231"), None);
        assert_eq!(parse_cn_rrn("11010519491231002XY"), None);
    }
    #[test]
    fn cn_rrn_rejects_non_alnum_letters() {
        // A non-X non-digit at the check position is rejected.
        assert_eq!(parse_cn_rrn("11010519491231002A"), None);
    }

    // ---------- parse_in_aadhaar ----------
    #[test]
    fn in_aadhaar_canonical_form_parses() {
        assert_eq!(
            parse_in_aadhaar("234123412346"),
            Some("234123412346".into())
        );
    }
    #[test]
    fn in_aadhaar_strips_whitespace() {
        assert_eq!(
            parse_in_aadhaar("2341 2341 2346"),
            Some("234123412346".into()),
        );
    }
    #[test]
    fn in_aadhaar_rejects_wrong_verhoeff_check() {
        assert_eq!(parse_in_aadhaar("234123412347"), None);
        assert_eq!(parse_in_aadhaar("234123412345"), None);
    }
    #[test]
    fn in_aadhaar_rejects_all_equal_sequences() {
        for d in '2'..='9' {
            let s: String = std::iter::repeat_n(d, 12).collect();
            assert_eq!(parse_in_aadhaar(&s), None, "{s}");
        }
    }
    #[test]
    fn in_aadhaar_rejects_reserved_prefixes() {
        // UIDAI never issues numbers starting with 0 or 1.
        assert_eq!(parse_in_aadhaar("034123412346"), None);
        assert_eq!(parse_in_aadhaar("134123412346"), None);
    }
    #[test]
    fn in_aadhaar_rejects_bad_length() {
        assert_eq!(parse_in_aadhaar("234123412"), None);
        assert_eq!(parse_in_aadhaar("2341234123466"), None);
    }

    // ---------- parse_jp_my_number ----------
    #[test]
    fn jp_my_number_canonical_form_parses() {
        assert_eq!(
            parse_jp_my_number("123456789018"),
            Some("123456789018".into()),
        );
    }
    #[test]
    fn jp_my_number_strips_whitespace() {
        assert_eq!(
            parse_jp_my_number("1234 5678 9018"),
            Some("123456789018".into()),
        );
    }
    #[test]
    fn jp_my_number_rejects_wrong_check() {
        assert_eq!(parse_jp_my_number("123456789010"), None);
        assert_eq!(parse_jp_my_number("123456789019"), None);
    }
    #[test]
    fn jp_my_number_rejects_bad_length() {
        assert_eq!(parse_jp_my_number("12345678901"), None);
        assert_eq!(parse_jp_my_number("1234567890123"), None);
    }
    #[test]
    fn jp_my_number_rejects_non_digit_only_input() {
        assert_eq!(parse_jp_my_number("abcdefghijkl"), None);
    }

    // ---------- parse_mx_curp ----------
    #[test]
    fn mx_curp_canonical_form_parses() {
        assert_eq!(
            parse_mx_curp("HEGG560427MVZRRL04"),
            Some("HEGG560427MVZRRL04".into()),
        );
    }
    #[test]
    fn mx_curp_uppercases_input() {
        assert_eq!(
            parse_mx_curp("hegg560427mvzrrl04"),
            Some("HEGG560427MVZRRL04".into()),
        );
    }
    #[test]
    fn mx_curp_rejects_wrong_check() {
        assert_eq!(parse_mx_curp("HEGG560427MVZRRL05"), None);
    }
    #[test]
    fn mx_curp_rejects_invalid_date_substring() {
        assert_eq!(parse_mx_curp("HEGG561327MVZRRL04"), None);
        assert_eq!(parse_mx_curp("HEGG569927MVZRRL04"), None);
    }
    #[test]
    fn mx_curp_rejects_bad_sex_char() {
        assert_eq!(parse_mx_curp("HEGG560427XVZRRL04"), None);
    }
    #[test]
    fn mx_curp_rejects_bad_length() {
        assert_eq!(parse_mx_curp("HEGG560427"), None);
        assert_eq!(parse_mx_curp("HEGG560427MVZRRL04EXTRA"), None);
    }

    // ---------- parse_nz_nhi ----------
    #[test]
    fn nz_nhi_canonical_form_parses() {
        assert_eq!(parse_nz_nhi("ZAA0083"), Some("ZAA0083".into()));
    }
    #[test]
    fn nz_nhi_uppercases_input() {
        assert_eq!(parse_nz_nhi("zaa0083"), Some("ZAA0083".into()));
    }
    #[test]
    fn nz_nhi_rejects_wrong_check() {
        assert_eq!(parse_nz_nhi("ZAA0082"), None);
    }
    #[test]
    fn nz_nhi_rejects_excluded_letters_i_and_o() {
        assert_eq!(parse_nz_nhi("ZAI0083"), None);
        assert_eq!(parse_nz_nhi("ZAO0083"), None);
        assert_eq!(parse_nz_nhi("IZA0083"), None);
    }
    #[test]
    fn nz_nhi_rejects_bad_length() {
        assert_eq!(parse_nz_nhi("ZAA008"), None);
        assert_eq!(parse_nz_nhi("ZAA00830"), None);
    }
    #[test]
    fn nz_nhi_rejects_non_letter_prefix() {
        assert_eq!(parse_nz_nhi("Z1A0083"), None);
    }

    // ---------- parse_za_id ----------
    #[test]
    fn za_id_canonical_form_parses() {
        assert_eq!(parse_za_id("8001015009087"), Some("8001015009087".into()));
    }
    #[test]
    fn za_id_strips_whitespace() {
        assert_eq!(parse_za_id("800101 5009 087"), Some("8001015009087".into()),);
    }
    #[test]
    fn za_id_rejects_wrong_luhn() {
        assert_eq!(parse_za_id("8001015009088"), None);
        assert_eq!(parse_za_id("8001015009086"), None);
    }
    #[test]
    fn za_id_rejects_invalid_date_substring() {
        assert_eq!(parse_za_id("8013015009087"), None);
        assert_eq!(parse_za_id("8002305009087"), None);
    }
    #[test]
    fn za_id_rejects_bad_length() {
        assert_eq!(parse_za_id("80010150090"), None);
        assert_eq!(parse_za_id("80010150090870"), None);
    }

    // ----------------------------------------------------------------------
    // T-28: Nine per-country passport-number format validators.
    // ----------------------------------------------------------------------

    #[test]
    fn cy_passport_pre_2010_form_parses() {
        assert_eq!(parse_cy_passport("E123456"), Some("E123456".into()));
    }
    #[test]
    fn cy_passport_biometric_form_parses() {
        assert_eq!(parse_cy_passport("K12345678"), Some("K12345678".into()));
    }
    #[test]
    fn cy_passport_rejects_wrong_prefix() {
        assert_eq!(parse_cy_passport("A123456"), None);
        assert_eq!(parse_cy_passport("Z12345678"), None);
    }
    #[test]
    fn cy_passport_rejects_bad_length() {
        assert_eq!(parse_cy_passport("E12345"), None);
        assert_eq!(parse_cy_passport("K1234567"), None);
    }

    #[test]
    fn cz_passport_eight_digit_form_parses() {
        assert_eq!(parse_cz_passport("12345678"), Some("12345678".into()));
    }
    #[test]
    fn cz_passport_accepts_longer_forms() {
        assert_eq!(
            parse_cz_passport("123456789012"),
            Some("123456789012".into())
        );
    }
    #[test]
    fn cz_passport_rejects_short_forms() {
        assert_eq!(parse_cz_passport("1234567"), None);
        assert_eq!(parse_cz_passport(""), None);
    }

    #[test]
    fn li_passport_canonical_form_parses() {
        assert_eq!(parse_li_passport("R00536"), Some("R00536".into()));
    }
    #[test]
    fn li_passport_lowercases_to_upper() {
        assert_eq!(parse_li_passport("r00536"), Some("R00536".into()));
    }
    #[test]
    fn li_passport_rejects_bad_format() {
        assert_eq!(parse_li_passport("RR0536"), None);
        assert_eq!(parse_li_passport("123456"), None);
    }

    #[test]
    fn lt_passport_eight_digit_parses() {
        assert_eq!(parse_lt_passport("12345678"), Some("12345678".into()));
    }
    #[test]
    fn lt_passport_rejects_wrong_length() {
        assert_eq!(parse_lt_passport("1234567"), None);
        assert_eq!(parse_lt_passport("123456789"), None);
    }

    #[test]
    fn mt_passport_seven_digit_parses() {
        assert_eq!(parse_mt_passport("1234567"), Some("1234567".into()));
    }
    #[test]
    fn mt_passport_rejects_letters() {
        assert_eq!(parse_mt_passport("123456M"), None);
    }

    #[test]
    fn nl_passport_uses_nl_id_format() {
        assert_eq!(parse_nl_passport("AB1234567"), Some("AB1234567".into()));
        assert_eq!(parse_nl_passport("AO1234567"), None);
    }

    #[test]
    fn pt_passport_canonical_form_parses() {
        assert_eq!(parse_pt_passport("A123456"), Some("A123456".into()));
    }
    #[test]
    fn pt_passport_rejects_bad_shape() {
        assert_eq!(parse_pt_passport("AA12345"), None);
        assert_eq!(parse_pt_passport("1234567"), None);
    }

    #[test]
    fn ro_passport_canonical_form_parses() {
        assert_eq!(parse_ro_passport("AB123456"), Some("AB123456".into()));
    }
    #[test]
    fn ro_passport_rejects_bad_shape() {
        assert_eq!(parse_ro_passport("A1234567"), None);
        assert_eq!(parse_ro_passport("ABC12345"), None);
    }

    #[test]
    fn sk_passport_canonical_form_parses() {
        assert_eq!(parse_sk_passport("AB1234567"), Some("AB1234567".into()));
    }
    #[test]
    fn sk_passport_rejects_bad_shape() {
        assert_eq!(parse_sk_passport("ABC123456"), None);
        assert_eq!(parse_sk_passport("AB12345"), None);
    }
}
