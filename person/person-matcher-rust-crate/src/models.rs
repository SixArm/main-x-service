//! Data models for person demographics and identifiers.
//!
//! This module is intentionally **logic-free**: it defines the types that
//! flow through the matching engine but contains no matching code itself.
//! See [`crate::matcher`] for the engine and [`crate::normalizer`] for the
//! text transformations that the matcher applies to these fields.
//!
//! All public types here are `Serialize + Deserialize` so they round-trip
//! through JSON, `MessagePack`, or any other `serde` format.
//!
//! ## Building a person
//!
//! Prefer [`Person::builder`] over constructing the struct literal — the
//! builder accepts `impl Into<String>` so call-sites can pass `&str`,
//! `String`, or owned values interchangeably.
//!
//! ```
//! use person_matcher::{Gender, Person};
//! use chrono::NaiveDate;
//!
//! let p = Person::builder()
//!     .united_kingdom_national_health_service_number("9434765919")
//!     .given_name("Dafydd")
//!     .family_name("Jones")
//!     .date_of_birth(NaiveDate::from_ymd_opt(1980, 5, 15).unwrap())
//!     .gender(Gender::Male)
//!     .build();
//!
//! assert_eq!(p.given_name.as_deref(), Some("Dafydd"));
//! assert_eq!(p.gender, Some(Gender::Male));
//! ```

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Gender/sex classification used to compare two [`Person`] records.
///
/// The four-arm enumeration mirrors common healthcare data dictionaries
/// (HL7 FHIR `AdministrativeGender`, NHS Data Dictionary `Person Gender`).
/// `Other` and `Unknown` are deliberately distinct: `Other` represents a
/// recorded non-binary value, whereas `Unknown` represents missing data.
///
/// # Example
///
/// ```
/// use person_matcher::Gender;
///
/// let g = Gender::Female;
/// assert_eq!(g, Gender::Female);
/// assert_ne!(g, Gender::Male);
/// ```
///
/// `Gender` is `Copy`, so it is cheap to pass by value.
///
/// ```
/// # use person_matcher::Gender;
/// fn describe(g: Gender) -> &'static str {
///     match g {
///         Gender::Male    => "male",
///         Gender::Female  => "female",
///         Gender::Other   => "other",
///         Gender::Unknown => "unknown",
///     }
/// }
/// assert_eq!(describe(Gender::Male), "male");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Gender {
    /// Administrative gender recorded as male.
    Male,
    /// Administrative gender recorded as female.
    Female,
    /// Recorded non-binary or otherwise specified value.
    Other,
    /// No gender recorded, or gender intentionally withheld.
    Unknown,
}

/// ABO + `RhD` blood type used as supporting evidence in person matcher.
///
/// Blood type is a **weak positive** signal and a **strong negative**
/// signal:
///
/// - Many people share a blood type (≈38% of the US population is O+),
///   so agreement alone is not strong evidence of a match.
/// - Two records with disagreeing blood types almost certainly refer
///   to **different** people — blood type does not change over a
///   lifetime (modulo bone-marrow transplant edge cases).
///
/// The matcher therefore weights blood type at the same low level as
/// gender by default (`MatchConfig::blood_type_weight = 0.05`) but the
/// per-field score in `MatchBreakdown::blood_type_score` is surfaced
/// for downstream consumers that want to flag disagreement explicitly.
///
/// Blood type is **not** an identifying field for `Person::validate`,
/// and it is **not** consulted by `deterministic_match` — disagreement
/// is a soft signal, not a binary disqualifier.
///
/// # JSON
///
/// Variants serialise as their canonical short form (`"A+"`, `"O-"`,
/// `"AB+"`, etc.) via `#[serde(rename = …)]`.
///
/// ```
/// use person_matcher::BloodType;
/// assert_eq!(serde_json::to_string(&BloodType::APositive).unwrap(), "\"A+\"");
/// let back: BloodType = serde_json::from_str("\"AB-\"").unwrap();
/// assert_eq!(back, BloodType::ABNegative);
/// ```
///
/// # Parsing
///
/// [`BloodType::parse`] accepts the canonical short forms plus the
/// most common textual layouts found in real EMR data:
///
/// ```
/// use person_matcher::BloodType;
/// assert_eq!(BloodType::parse("A+"),         Some(BloodType::APositive));
/// assert_eq!(BloodType::parse("a positive"), Some(BloodType::APositive));
/// assert_eq!(BloodType::parse("AB neg"),     Some(BloodType::ABNegative));
/// assert_eq!(BloodType::parse("O-"),         Some(BloodType::ONegative));
/// assert_eq!(BloodType::parse("0+"),         Some(BloodType::OPositive));  // zero/O confusion
/// assert_eq!(BloodType::parse(""),           None);
/// assert_eq!(BloodType::parse("Z+"),         None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BloodType {
    /// A positive (A+).
    #[serde(rename = "A+")]
    APositive,
    /// A negative (A−).
    #[serde(rename = "A-")]
    ANegative,
    /// B positive (B+).
    #[serde(rename = "B+")]
    BPositive,
    /// B negative (B−).
    #[serde(rename = "B-")]
    BNegative,
    /// AB positive (AB+). Universal red-cell recipient.
    #[serde(rename = "AB+")]
    ABPositive,
    /// AB negative (AB−). Rare; universal-plasma donor.
    #[serde(rename = "AB-")]
    ABNegative,
    /// O positive (O+). Most common worldwide.
    #[serde(rename = "O+")]
    OPositive,
    /// O negative (O−). Universal red-cell donor.
    #[serde(rename = "O-")]
    ONegative,
}

impl BloodType {
    /// Canonical short form: `"A+"`, `"A-"`, `"B+"`, `"B-"`, `"AB+"`,
    /// `"AB-"`, `"O+"`, `"O-"`.
    ///
    /// ```
    /// use person_matcher::BloodType;
    /// assert_eq!(BloodType::APositive.as_str(),  "A+");
    /// assert_eq!(BloodType::ABNegative.as_str(), "AB-");
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            BloodType::APositive => "A+",
            BloodType::ANegative => "A-",
            BloodType::BPositive => "B+",
            BloodType::BNegative => "B-",
            BloodType::ABPositive => "AB+",
            BloodType::ABNegative => "AB-",
            BloodType::OPositive => "O+",
            BloodType::ONegative => "O-",
        }
    }

    /// Parse a blood-type string, accepting canonical short forms as
    /// well as the common textual layouts seen in EMR / HL7 data.
    /// Returns `None` for unparseable, empty, or rare-phenotype input;
    /// consumers that need to preserve a rare phenotype should store
    /// the raw string elsewhere.
    ///
    /// Accepted shapes (case-insensitive, whitespace tolerated):
    ///
    /// - Canonical: `A+`, `A-`, `B+`, `B-`, `AB+`, `AB-`, `O+`, `O-`.
    /// - Word forms: `A positive`, `A pos`, `A negative`, `A neg`.
    /// - With sign-separator: `A_pos`, `A-neg`, `AB +`.
    /// - With zero/O confusion: `0+` is read as `O+`.
    ///
    /// ```
    /// use person_matcher::BloodType;
    /// assert_eq!(BloodType::parse("O Negative"), Some(BloodType::ONegative));
    /// assert_eq!(BloodType::parse("ab+"),        Some(BloodType::ABPositive));
    /// assert_eq!(BloodType::parse("Bombay"),     None); // rare phenotype, not supported
    /// ```
    #[must_use]
    pub fn parse(s: &str) -> Option<BloodType> {
        let upper: String = s
            .trim()
            .to_uppercase()
            .chars()
            .map(|c| if c == '0' { 'O' } else { c })
            .collect();
        if upper.is_empty() {
            return None;
        }
        let (group, rest): (&str, &str) = if let Some(r) = upper.strip_prefix("AB") {
            ("AB", r)
        } else if let Some(r) = upper.strip_prefix('A') {
            ("A", r)
        } else if let Some(r) = upper.strip_prefix('B') {
            ("B", r)
        } else if let Some(r) = upper.strip_prefix('O') {
            ("O", r)
        } else {
            return None;
        };
        let positive = parse_rh_sign(rest)?;
        Some(match (group, positive) {
            ("A", true) => BloodType::APositive,
            ("A", false) => BloodType::ANegative,
            ("B", true) => BloodType::BPositive,
            ("B", false) => BloodType::BNegative,
            ("AB", true) => BloodType::ABPositive,
            ("AB", false) => BloodType::ABNegative,
            ("O", true) => BloodType::OPositive,
            ("O", false) => BloodType::ONegative,
            _ => return None,
        })
    }
}

impl std::fmt::Display for BloodType {
    /// Write the canonical short form (e.g. `"O-"`), delegating to
    /// [`BloodType::as_str`] so `Display` and `as_str` never diverge.
    ///
    /// # Errors
    ///
    /// Propagates any [`std::fmt::Error`] returned by the underlying
    /// formatter's `write_str`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Parse the Rhesus-sign portion of a blood-type string after the ABO
/// group prefix has been stripped. Returns `Some(true)` for positive,
/// `Some(false)` for negative, `None` for unparseable input.
fn parse_rh_sign(s: &str) -> Option<bool> {
    let trimmed = s.trim_start_matches([' ', '\t', '_', '/']).trim();
    if trimmed.is_empty() {
        return None;
    }
    // Word-form check first: "A POS", "A-NEG", "A_POSITIVE" all reach
    // here with `trimmed` containing the word (possibly prefixed by a
    // separator like `-` or `+`). We tolerate one leading sign
    // character as a separator, since the word itself disambiguates.
    let word_candidate = trimmed.trim_start_matches(['-', '+']).trim();
    if word_candidate.starts_with("POSITIVE")
        || word_candidate.starts_with("POS")
        || word_candidate == "P"
    {
        return Some(true);
    }
    if word_candidate.starts_with("NEGATIVE")
        || word_candidate.starts_with("NEG")
        || word_candidate == "N"
    {
        return Some(false);
    }
    // Single-character sign forms (with optional `VE` suffix).
    if let Some(after) = trimmed.strip_prefix('+') {
        let tail = after.trim().trim_start_matches("VE");
        if tail.trim().is_empty() {
            return Some(true);
        }
        return None;
    }
    if let Some(after) = trimmed.strip_prefix('-') {
        let tail = after.trim().trim_start_matches("VE");
        if tail.trim().is_empty() {
            return Some(false);
        }
        return None;
    }
    None
}

/// Physical address used as supporting evidence in person matcher.
///
/// All fields are `Option<String>` so partial addresses are first-class —
/// a record with only a postcode is still useful for matching.
///
/// The matcher does **not** weight every component equally; see
/// [`crate::matcher::MatchingEngine`] for the weighted comparison rules.
///
/// # Example
///
/// ```
/// use person_matcher::Address;
///
/// let mut addr = Address::new();
/// addr.line1    = Some("10 Downing Street".into());
/// addr.city     = Some("London".into());
/// addr.postcode = Some("SW1A 2AA".into());
///
/// assert_eq!(addr.postcode.as_deref(), Some("SW1A 2AA"));
/// assert!(addr.country.is_none());
/// ```
///
/// `Address` is JSON round-trippable.
///
/// ```
/// # use person_matcher::Address;
/// let mut a = Address::new();
/// a.postcode = Some("CF10 1AA".into());
///
/// let json = serde_json::to_string(&a).unwrap();
/// let back: Address = serde_json::from_str(&json).unwrap();
/// assert_eq!(a, back);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Address {
    /// First line — typically house number and street, e.g. `"10 Downing Street"`.
    pub line1: Option<String>,
    /// Second line — typically flat, locality, or care-of details.
    pub line2: Option<String>,
    /// Town or city, e.g. `"Cardiff"`.
    pub city: Option<String>,
    /// County or administrative region, e.g. `"South Glamorgan"`.
    pub county: Option<String>,
    /// Postal code, e.g. `"CF10 1AA"`. Compared after whitespace normalisation.
    pub postcode: Option<String>,
    /// Country, e.g. `"Wales"` or `"United Kingdom"`.
    pub country: Option<String>,
}

impl Address {
    /// Construct an empty address with every field set to `None`.
    ///
    /// # Example
    ///
    /// ```
    /// use person_matcher::Address;
    ///
    /// let a = Address::new();
    /// assert!(a.line1.is_none());
    /// assert!(a.postcode.is_none());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            line1: None,
            line2: None,
            city: None,
            county: None,
            postcode: None,
            country: None,
        }
    }

    /// Fluent setter for `line1`.
    ///
    /// ```
    /// use person_matcher::Address;
    /// let a = Address::new().with_line1("10 Downing Street");
    /// assert_eq!(a.line1.as_deref(), Some("10 Downing Street"));
    /// ```
    pub fn with_line1(mut self, value: impl Into<String>) -> Self {
        self.line1 = Some(value.into());
        self
    }

    /// Fluent setter for `line2`.
    pub fn with_line2(mut self, value: impl Into<String>) -> Self {
        self.line2 = Some(value.into());
        self
    }

    /// Fluent setter for `city`.
    pub fn with_city(mut self, value: impl Into<String>) -> Self {
        self.city = Some(value.into());
        self
    }

    /// Fluent setter for `county`.
    pub fn with_county(mut self, value: impl Into<String>) -> Self {
        self.county = Some(value.into());
        self
    }

    /// Fluent setter for `postcode`.
    ///
    /// ```
    /// use person_matcher::Address;
    /// let a = Address::new().with_postcode("CF10 1AA");
    /// assert_eq!(a.postcode.as_deref(), Some("CF10 1AA"));
    /// ```
    pub fn with_postcode(mut self, value: impl Into<String>) -> Self {
        self.postcode = Some(value.into());
        self
    }

    /// Fluent setter for `country`.
    pub fn with_country(mut self, value: impl Into<String>) -> Self {
        self.country = Some(value.into());
        self
    }
}

impl Default for Address {
    /// Identical to [`Address::new`].
    fn default() -> Self {
        Self::new()
    }
}

/// A passport book — country of issue, book number, and optional
/// effective date range.
///
/// Passport data has three properties that make it a poor fit for the
/// crate's per-scheme `Option<String>` national-identifier pattern,
/// and which this type captures explicitly:
///
/// 1. **Scheme-local provenance.** A passport book number is only
///    meaningful alongside its issuing country. The book number
///    `"AB123456"` issued by the United Kingdom is a different
///    identifier from `"AB123456"` issued by the United States; the
///    matcher MUST NOT cross-match them. Provenance lives on the
///    [`PassportBook::country`] field, not on the field name.
/// 2. **Multi-country.** A single person may hold passports from
///    multiple countries simultaneously (dual / multiple citizenship).
///    A `Vec<PassportBook>` lets a [`crate::Person`] carry one entry
///    per book without privileging any particular jurisdiction.
/// 3. **Time-varying.** When a passport is renewed, the new book has
///    a different number; the old book number is no longer current
///    but the person is unchanged. Person records may carry the
///    current book, prior books, or both. Matching treats any shared
///    `(country, number)` pair across the two records as evidence
///    that the records refer to the same person, regardless of issue
///    date.
///
/// Construction via [`PassportBook::new`] canonicalises both the
/// country (trimmed, uppercased; must be exactly 2 ASCII letters) and
/// the number (whitespace stripped, letters uppercased) so two records
/// carrying different textual layouts of the same book canonicalise to
/// the same `(country, number)` key. Date fields are optional metadata
/// and are **not** used in matching — they exist for downstream
/// display and audit. Per-country structural validation is
/// intentionally not performed; passport formats vary widely and a
/// case+whitespace canonical form is sufficient for matching.
///
/// # Example
///
/// ```
/// use person_matcher::PassportBook;
/// use chrono::NaiveDate;
///
/// let book = PassportBook::new("gb", " 123 456 789 ")
///     .expect("valid book")
///     .with_issued(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap())
///     .with_expires(NaiveDate::from_ymd_opt(2030, 1, 1).unwrap());
///
/// assert_eq!(book.country, "GB");
/// assert_eq!(book.number,  "123456789");
/// assert!(book.issued.is_some());
///
/// // Rejection: country must be exactly 2 ASCII letters.
/// assert!(PassportBook::new("GBR", "123").is_none());
/// assert!(PassportBook::new("1A",  "123").is_none());
/// // Rejection: number must canonicalise to a non-empty string.
/// assert!(PassportBook::new("GB",  "   ").is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PassportBook {
    /// ISO 3166-1 alpha-2 country code of issuance, uppercased.
    pub country: String,
    /// Passport book number, whitespace-stripped and uppercased.
    pub number: String,
    /// Optional issue date (not used in matching).
    #[serde(default)]
    pub issued: Option<NaiveDate>,
    /// Optional expiry date (not used in matching).
    #[serde(default)]
    pub expires: Option<NaiveDate>,
}

impl PassportBook {
    /// Construct a passport book, validating and canonicalising the
    /// country code (trimmed + uppercased; must be exactly 2 ASCII
    /// letters) and the book number (whitespace stripped + uppercased;
    /// must be non-empty after stripping).
    ///
    /// Returns `None` for an invalid country code or an empty
    /// canonical number.
    ///
    /// ```
    /// use person_matcher::PassportBook;
    /// let b = PassportBook::new("us", "abc 123 456").unwrap();
    /// assert_eq!(b.country, "US");
    /// assert_eq!(b.number,  "ABC123456");
    /// ```
    pub fn new(country: impl AsRef<str>, number: impl AsRef<str>) -> Option<Self> {
        let country = country.as_ref().trim().to_ascii_uppercase();
        if country.len() != 2 || !country.chars().all(|c| c.is_ascii_alphabetic()) {
            return None;
        }
        // Strip common data-entry separators (whitespace, ASCII
        // hyphens, periods, slashes) and uppercase. This matches the
        // canonicalisation used by `parse_es_tsi` / `parse_ie_ihi` so
        // textual variants of the same book canonicalise identically.
        let number: String = number
            .as_ref()
            .chars()
            .filter(|c| !c.is_whitespace() && !matches!(*c, '-' | '.' | '/'))
            .collect::<String>()
            .to_uppercase();
        if number.is_empty() {
            return None;
        }
        Some(Self {
            country,
            number,
            issued: None,
            expires: None,
        })
    }

    /// Attach an issue date. The date is metadata only — it is NOT
    /// used in matching.
    ///
    /// ```
    /// use person_matcher::PassportBook;
    /// use chrono::NaiveDate;
    /// let b = PassportBook::new("GB", "123456789").unwrap()
    ///     .with_issued(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap());
    /// assert!(b.issued.is_some());
    /// ```
    #[must_use]
    pub fn with_issued(mut self, date: NaiveDate) -> Self {
        self.issued = Some(date);
        self
    }

    /// Attach an expiry date. The date is metadata only — it is NOT
    /// used in matching.
    ///
    /// ```
    /// use person_matcher::PassportBook;
    /// use chrono::NaiveDate;
    /// let b = PassportBook::new("GB", "123456789").unwrap()
    ///     .with_expires(NaiveDate::from_ymd_opt(2030, 1, 1).unwrap());
    /// assert!(b.expires.is_some());
    /// ```
    #[must_use]
    pub fn with_expires(mut self, date: NaiveDate) -> Self {
        self.expires = Some(date);
        self
    }
}

/// Core person demographic data structure.
///
/// Every field is optional. The matcher tolerates missing data field-by-field
/// — a `None` value never penalises a person. See
/// [`crate::matcher::MatchingEngine::match_persons`] for how missing fields
/// affect the weighted score.
///
/// Construct via [`Person::builder`] rather than struct literal syntax so
/// the call-site stays compact and forward-compatible if fields are added.
///
/// # Example
///
/// ```
/// use person_matcher::{Gender, Person};
/// use chrono::NaiveDate;
///
/// let p = Person::builder()
///     .given_name("Siân")
///     .family_name("Evans")
///     .date_of_birth(NaiveDate::from_ymd_opt(1990, 3, 10).unwrap())
///     .gender(Gender::Female)
///     .build();
///
/// assert_eq!(p.given_name.as_deref(), Some("Siân"));
/// assert!(p.united_kingdom_national_health_service_number.is_none());
/// ```
///
/// `Person` round-trips through `serde`.
///
/// ```
/// # use person_matcher::Person;
/// let p = Person::builder().given_name("Test").family_name("Person").build();
/// let json = serde_json::to_string(&p).unwrap();
/// let back: Person = serde_json::from_str(&json).unwrap();
/// assert_eq!(p, back);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Person {
    /// United Kingdom National Health Service Number (England, Wales, Isle of
    /// Man) — a 10-digit Modulus-11 identifier parsed via
    /// [`crate::identifiers::parse_united_kingdom_national_health_service_number`].
    /// Whitespace tolerated in the spaced `"XXX XXX XXXX"` layout.
    #[serde(default)]
    pub united_kingdom_national_health_service_number: Option<String>,

    /// France NIR (*Numéro d'Inscription au Répertoire*) — the 15-character
    /// national identifier with a Modulus-97 check key. Parsed via
    /// [`crate::identifiers::parse_fr_nir`].
    #[serde(default)]
    pub fr_nir: Option<String>,

    /// España (Spain) TSI (*Tarjeta Sanitaria Individual*) / CIP-SNS — the
    /// national healthcare identifier with regionally-varying format. Parsed
    /// via [`crate::identifiers::parse_es_tsi`].
    #[serde(default)]
    pub es_tsi: Option<String>,

    /// Éire (Ireland) IHI (Individual Health Identifier) — the 7-digit
    /// healthcare identifier issued under the Health Identifiers Act 2014.
    /// Parsed via [`crate::identifiers::parse_ie_ihi`].
    #[serde(default)]
    pub ie_ihi: Option<String>,

    /// United Kingdom Northern Ireland H&C Number (Health and Care Number)
    /// — a 10-digit Modulus-11 identifier issued by HSC. Shares the United
    /// Kingdom National Health Service Number algorithm. Parsed via
    /// [`crate::identifiers::parse_uk_hc_number`].
    #[serde(default)]
    pub uk_hc_number: Option<String>,

    /// United States Social Security Number (SSN) — a 9-digit identifier
    /// issued by the Social Security Administration. Parsed via
    /// [`crate::identifiers::parse_us_ssn`].
    #[serde(default)]
    pub us_ssn: Option<String>,

    /// Australia IHI (Individual Healthcare Identifier) — 16-digit
    /// identifier issued by the Healthcare Identifiers Service. Parsed
    /// via [`crate::identifiers::parse_au_ihi`].
    #[serde(default)]
    pub au_ihi: Option<String>,

    /// Germany KVNR (*Krankenversichertennummer*) — 10-character
    /// (letter + 9 digits) lifelong health-insurance number. Parsed via
    /// [`crate::identifiers::parse_de_kvnr`].
    #[serde(default)]
    pub de_kvnr: Option<String>,

    /// Italy *Codice Fiscale* (CF) — 16-character alphanumeric
    /// identifier issued by the tax authority. Parsed via
    /// [`crate::identifiers::parse_it_cf`].
    #[serde(default)]
    pub it_cf: Option<String>,

    /// Netherlands BSN (*Burgerservicenummer*) — 9-digit citizen-service
    /// number used by Dutch authorities and healthcare providers. Parsed
    /// via [`crate::identifiers::parse_nl_bsn`].
    #[serde(default)]
    pub nl_bsn: Option<String>,

    /// Sweden *Personnummer* — 10- or 12-digit personal identity number
    /// (`YYMMDDNNNC` or `YYYYMMDDNNNC` with optional `-` / `+`
    /// separator). Parsed via
    /// [`crate::identifiers::parse_se_personnummer`].
    #[serde(default)]
    pub se_personnummer: Option<String>,

    /// United Kingdom (Scotland) CHI Number (Community Health Index) —
    /// 10-digit identifier used by United Kingdom National Health Service Scotland. Shares the Mod-11
    /// algorithm of the United Kingdom National Health Service Number but is
    /// scheme-local. Parsed via [`crate::identifiers::parse_uk_chi_number`].
    #[serde(default)]
    pub uk_chi_number: Option<String>,

    /// Belgium National Number (*Rijksregisternummer*). 11 digits, Mod-97.
    /// Parsed via [`crate::identifiers::parse_be_nn`].
    #[serde(default)]
    pub be_nn: Option<String>,

    /// Bulgaria EGN (*Edinen grazhdanski nomer*). 10 digits, weighted Mod-11.
    /// Parsed via [`crate::identifiers::parse_bg_egn`].
    #[serde(default)]
    pub bg_egn: Option<String>,

    /// Czech Republic *Rodné číslo*. 9 or 10 digits (10-digit divisible by 11).
    /// Parsed via [`crate::identifiers::parse_cz_rc`].
    #[serde(default)]
    pub cz_rc: Option<String>,

    /// Denmark CPR (*Centrale Personregister*). 10 digits, format only.
    /// Parsed via [`crate::identifiers::parse_dk_cpr`].
    #[serde(default)]
    pub dk_cpr: Option<String>,

    /// Estonia *Isikukood* (Personal Identification Code). 11 digits, cascading Mod-11.
    /// Parsed via [`crate::identifiers::parse_ee_ik`].
    #[serde(default)]
    pub ee_ik: Option<String>,

    /// Spain DNI / NIE. 8 digits + Mod-23 control letter (NIE prefixed X/Y/Z).
    /// Parsed via [`crate::identifiers::parse_es_dni`].
    #[serde(default)]
    pub es_dni: Option<String>,

    /// Finland HETU (*Henkilötunnus*). 11 chars with century sign + Mod-31 check.
    /// Parsed via [`crate::identifiers::parse_fi_hetu`].
    #[serde(default)]
    pub fi_hetu: Option<String>,

    /// Croatia OIB (*Osobni identifikacijski broj*). 11 digits, ISO 7064 MOD 11,10.
    /// Parsed via [`crate::identifiers::parse_hr_oib`].
    #[serde(default)]
    pub hr_oib: Option<String>,

    /// Iceland *Kennitala*. 10 digits, weighted Mod-11.
    /// Parsed via [`crate::identifiers::parse_is_kt`].
    #[serde(default)]
    pub is_kt: Option<String>,

    /// Lithuania *Asmens kodas*. 11 digits, cascading Mod-11 (same algorithm as Estonia).
    /// Parsed via [`crate::identifiers::parse_lt_ak`].
    #[serde(default)]
    pub lt_ak: Option<String>,

    /// Latvia *Personas kods*. 11 digits, weighted Mod-11.
    /// Parsed via [`crate::identifiers::parse_lv_pk`].
    #[serde(default)]
    pub lv_pk: Option<String>,

    /// Malta National ID. 7 digits + letter in `{M, G, A, P, L, H, B, Z}`.
    /// Parsed via [`crate::identifiers::parse_mt_id`].
    #[serde(default)]
    pub mt_id: Option<String>,

    /// Norway *Fødselsnummer*. 11 digits, dual Mod-11.
    /// Parsed via [`crate::identifiers::parse_no_fnr`].
    #[serde(default)]
    pub no_fnr: Option<String>,

    /// Poland PESEL. 11 digits, weighted Mod-10.
    /// Parsed via [`crate::identifiers::parse_pl_pesel`].
    #[serde(default)]
    pub pl_pesel: Option<String>,

    /// Romania CNP (*Cod Numeric Personal*). 13 digits, weighted Mod-11.
    /// Parsed via [`crate::identifiers::parse_ro_cnp`].
    #[serde(default)]
    pub ro_cnp: Option<String>,

    /// Slovenia EMŠO (*Enotna Matična Številka Občana*). 13 digits, weighted Mod-11.
    /// Parsed via [`crate::identifiers::parse_si_emso`].
    #[serde(default)]
    pub si_emso: Option<String>,

    /// Slovakia *Rodné číslo*. 9 or 10 digits (same algorithm as Czech RČ).
    /// Parsed via [`crate::identifiers::parse_sk_rc`].
    #[serde(default)]
    pub sk_rc: Option<String>,

    /// United Kingdom National Insurance Number (NINO). Format `AA999999A`
    /// with banned prefixes and `{A,B,C,D}` suffix.
    /// Parsed via [`crate::identifiers::parse_uk_nino`].
    #[serde(default)]
    pub uk_nino: Option<String>,

    /// Greece DSS (Dematerialised Securities System) investor share code.
    /// 10-digit identifier issued by the Hellenic Central Securities
    /// Depository (ATHEXCSD). Parsed via
    /// [`crate::identifiers::parse_gr_dss`].
    #[serde(default)]
    pub gr_dss: Option<String>,

    /// Liechtenstein National Identity Card Number. 2 letters + 8 digits
    /// (per the spec) or 2 letters + 9 digits (per the spec's example).
    /// Note: the LI ID card number is **regenerated on each renewal**, so
    /// consumers that need stable cross-renewal matching should prefer
    /// [`PassportBook`] with `country = "LI"`. Parsed via
    /// [`crate::identifiers::parse_li_id`].
    #[serde(default)]
    pub li_id: Option<String>,

    /// Netherlands National Identity Card Number. 9 characters: positions
    /// 1–2 are uppercase letters except `O`; positions 3–8 are
    /// alphanumeric except `O`; position 9 is a digit. Distinct from the
    /// BSN (citizen-service number), which is permanent — this ID-card
    /// number changes with each renewed card.
    /// Parsed via [`crate::identifiers::parse_nl_id`].
    #[serde(default)]
    pub nl_id: Option<String>,

    /// Poland NIP (*Numer Identyfikacji Podatkowej*) tax identification
    /// number. 10 digits, weighted Mod-11 check. Parsed via
    /// [`crate::identifiers::parse_pl_nip`].
    #[serde(default)]
    pub pl_nip: Option<String>,

    /// Portugal NIF (*Número de Identificação Fiscal*) tax identification
    /// number. 9 digits, weighted Mod-11 check. Parsed via
    /// [`crate::identifiers::parse_pt_nif`].
    #[serde(default)]
    pub pt_nif: Option<String>,

    /// Brazil CPF (*Cadastro de Pessoas Físicas*). 11-digit national tax /
    /// identification number with two Mod-11 check digits. Parsed at match
    /// time via [`crate::identifiers::parse_br_cpf`].
    #[serde(default)]
    pub br_cpf: Option<String>,

    /// China Resident Identity Card number (*居民身份证*) — 18-character
    /// 1999 reform format (17 digits + check character). Parsed at match
    /// time via [`crate::identifiers::parse_cn_rrn`].
    #[serde(default)]
    pub cn_rrn: Option<String>,

    /// India Aadhaar number. 12 digits with Verhoeff check digit. Parsed
    /// at match time via [`crate::identifiers::parse_in_aadhaar`].
    #[serde(default)]
    pub in_aadhaar: Option<String>,

    /// Japan My Number (*個人番号*). 12-digit personal identification
    /// number with weighted Mod-11 check digit. Parsed at match time via
    /// [`crate::identifiers::parse_jp_my_number`].
    #[serde(default)]
    pub jp_my_number: Option<String>,

    /// Mexico CURP (*Clave Única de Registro de Población*). 18-character
    /// alphanumeric identifier encoding name initials, date of birth,
    /// sex, state, and a check digit. Parsed at match time via
    /// [`crate::identifiers::parse_mx_curp`].
    #[serde(default)]
    pub mx_curp: Option<String>,

    /// New Zealand NHI (National Health Index) number. Original 7-character
    /// format (3 letters + 4 digits, Mod-11 check digit). The 2019
    /// alphanumeric NHI revision is not supported by the parser. Parsed at
    /// match time via [`crate::identifiers::parse_nz_nhi`].
    #[serde(default)]
    pub nz_nhi: Option<String>,

    /// South Africa ID Number. 13 digits encoding date of birth, sequence,
    /// citizenship, and a Luhn check digit. Parsed at match time via
    /// [`crate::identifiers::parse_za_id`].
    #[serde(default)]
    pub za_id: Option<String>,

    /// Given name (sometimes called "first name" or "forename").
    pub given_name: Option<String>,

    /// Middle name(s). Currently unused in scoring — see spec OQ-1.
    pub middle_name: Option<String>,

    /// Family name (sometimes called "surname" or "last name").
    pub family_name: Option<String>,

    /// Date of birth. Compared by exact equality.
    pub date_of_birth: Option<NaiveDate>,

    /// Date of death (FHIR `Patient.deceasedDateTime`). Compared using
    /// the same DOB transposition heuristic as
    /// [`Person::date_of_birth`] — DD/MM ↔ MM/DD data-entry bugs are
    /// just as common in death records as in birth records.
    #[serde(default)]
    pub death_date: Option<NaiveDate>,

    /// Administrative gender. See [`Gender`].
    pub gender: Option<Gender>,

    /// ABO+RhD blood type. Stable for life, so disagreement is strong
    /// evidence against a match; agreement is a weak positive signal
    /// (many people share a blood type). See [`BloodType`] for the
    /// scoring contract.
    #[serde(default)]
    pub blood_type: Option<BloodType>,

    /// FHIR `Patient.multipleBirth` — birth order in a multiple-birth
    /// set (twin / triplet / etc.). Convention:
    ///
    /// - `None` — unknown, not recorded, or singleton (the matcher
    ///   treats `None` as "no signal" and skips the field).
    /// - `Some(n)` with `n >= 1` — the `n`-th birth in a multiple-birth
    ///   set (1-indexed).
    ///
    /// Used to distinguish identical twins who otherwise share name,
    /// DOB, address, and demographic data. Disagreement (e.g. `Some(1)`
    /// vs `Some(2)`) is reliable evidence the records refer to
    /// different people in the same multiple-birth set.
    #[serde(default)]
    pub multiple_birth: Option<u8>,

    /// Current residential address.
    pub address: Option<Address>,

    /// Place of birth. Modelled as an [`Address`] for FHIR
    /// (`Patient.birthPlace`) parity — typically only `city` and
    /// `country` are populated in practice. Stable over a lifetime
    /// (modulo refugee / adoption edge cases), so disagreement on a
    /// populated value is informative. Scored independently from the
    /// current `address` field.
    #[serde(default)]
    pub birth_place: Option<Address>,

    /// Place of death. Modelled as an [`Address`] (parallel to
    /// [`Person::birth_place`]) — typically only `city` and `country`
    /// are populated. Useful for disambiguating records of deceased
    /// persons (e.g. distinguishing two people with the same name and
    /// DOB by where they died). Scored independently from `address`
    /// and `birth_place`.
    #[serde(default)]
    pub death_place: Option<Address>,

    /// Previous residential addresses. Used by the address sub-score
    /// (best-of cartesian product across `address ∪ previous_addresses`
    /// on both sides; see spec §12.4.2).
    pub previous_addresses: Vec<Address>,

    /// Passport books held by the person. A single person may hold
    /// passports from multiple countries simultaneously and may
    /// accumulate historical book numbers as old passports are
    /// renewed; this `Vec` carries every book ever recorded on the
    /// person (current and historical) without privileging any
    /// particular jurisdiction. Matching treats any shared
    /// `(country, number)` pair across the two persons' lists as
    /// evidence that the records refer to the same person, regardless
    /// of issue date. See [`PassportBook`].
    #[serde(default)]
    pub passport_books: Vec<PassportBook>,

    /// Primary phone number. Falls back to [`Self::mobile`] in scoring if absent.
    pub phone: Option<String>,

    /// Mobile phone number. Used as the fallback for [`Self::phone`].
    pub mobile: Option<String>,

    /// Email address. Not currently used in scoring (see spec task T-11).
    pub email: Option<String>,

    /// Local hospital or practice identifier. Not normalised — different
    /// organisations may issue colliding values.
    pub local_id: Option<String>,
}

impl Person {
    /// Begin constructing a [`Person`] with the [`PersonBuilder`].
    ///
    /// All fields default to `None` / empty until a setter is called.
    ///
    /// # Example
    ///
    /// ```
    /// use person_matcher::Person;
    ///
    /// let p = Person::builder()
    ///     .given_name("John")
    ///     .family_name("Smith")
    ///     .build();
    ///
    /// assert_eq!(p.family_name.as_deref(), Some("Smith"));
    /// ```
    #[must_use]
    pub fn builder() -> PersonBuilder {
        PersonBuilder::default()
    }

    /// Validate that the person carries at least one identifying field.
    ///
    /// Returns `Ok(())` if any of the following is set: a name (`given_name`
    /// or `family_name`), or any national identifier (`united_kingdom_national_health_service_number`,
    /// `fr_nir`, `es_tsi`, `ie_ihi`, `uk_hc_number`). Otherwise returns
    /// [`crate::MatchingError::MissingField`].
    ///
    /// This is **not** invoked automatically by the matcher — call it at the
    /// system boundary when you ingest data, not on every comparison.
    ///
    /// # Errors
    ///
    /// Returns [`crate::MatchingError::MissingField`] when the person carries
    /// neither a name nor any national identifier.
    ///
    /// # Example
    ///
    /// ```
    /// use person_matcher::Person;
    ///
    /// assert!(Person::builder().given_name("Ada").build().validate().is_ok());
    /// assert!(Person::builder().united_kingdom_national_health_service_number("9434765919").build().validate().is_ok());
    /// assert!(Person::builder().ie_ihi("1234567").build().validate().is_ok());
    /// assert!(Person::builder().us_ssn("123-45-6789").build().validate().is_ok());
    /// assert!(Person::builder().de_kvnr("A123456780").build().validate().is_ok());
    /// assert!(
    ///     Person::builder()
    ///         .add_passport_book(person_matcher::PassportBook::new("GB", "123456789").unwrap())
    ///         .build()
    ///         .validate()
    ///         .is_ok()
    /// );
    /// assert!(Person::builder().build().validate().is_err());
    /// ```
    pub fn validate(&self) -> crate::Result<()> {
        // A name on either side counts; a middle name alone is intentionally
        // NOT sufficient, since it is not scored (see Person::middle_name).
        let has_name = self.given_name.is_some() || self.family_name.is_some();
        // Any single populated national-identifier scheme OR a passport book
        // satisfies the "has identifier" requirement. The schemes are
        // OR-ed (not weighted) here because validate() only checks for the
        // *presence* of an identifying anchor, not its quality — scheme-local
        // matching strength is decided later by the matcher, not here.
        let has_identifier = self.united_kingdom_national_health_service_number.is_some()
            || self.fr_nir.is_some()
            || self.es_tsi.is_some()
            || self.ie_ihi.is_some()
            || self.uk_hc_number.is_some()
            || self.us_ssn.is_some()
            || self.au_ihi.is_some()
            || self.de_kvnr.is_some()
            || self.it_cf.is_some()
            || self.nl_bsn.is_some()
            || self.se_personnummer.is_some()
            || self.uk_chi_number.is_some()
            || self.be_nn.is_some()
            || self.bg_egn.is_some()
            || self.cz_rc.is_some()
            || self.dk_cpr.is_some()
            || self.ee_ik.is_some()
            || self.es_dni.is_some()
            || self.fi_hetu.is_some()
            || self.hr_oib.is_some()
            || self.is_kt.is_some()
            || self.lt_ak.is_some()
            || self.lv_pk.is_some()
            || self.mt_id.is_some()
            || self.no_fnr.is_some()
            || self.pl_pesel.is_some()
            || self.ro_cnp.is_some()
            || self.si_emso.is_some()
            || self.sk_rc.is_some()
            || self.uk_nino.is_some()
            || self.gr_dss.is_some()
            || self.li_id.is_some()
            || self.nl_id.is_some()
            || self.pl_nip.is_some()
            || self.pt_nif.is_some()
            || self.br_cpf.is_some()
            || self.cn_rrn.is_some()
            || self.in_aadhaar.is_some()
            || self.jp_my_number.is_some()
            || self.mx_curp.is_some()
            || self.nz_nhi.is_some()
            || self.za_id.is_some()
            || !self.passport_books.is_empty();
        // Reject only the truly anonymous record: no name AND no identifier.
        // Note blood_type, gender, dates, and address are deliberately
        // excluded — none of them is an identifying anchor on its own.
        if !has_name && !has_identifier {
            return Err(crate::MatchingError::MissingField(
                "At least one of: a name, a national identifier (any of 30 supported schemes), or at least one passport book is required"
                    .to_string(),
            ));
        }
        Ok(())
    }
}

/// Fluent builder for [`Person`].
///
/// All setters accept `impl Into<String>` so call-sites may pass `&str`,
/// `String`, or `&String` interchangeably without explicit conversion.
///
/// # Example
///
/// ```
/// use person_matcher::{Gender, Person, PersonBuilder};
/// use chrono::NaiveDate;
///
/// let p: Person = PersonBuilder::default()
///     .united_kingdom_national_health_service_number("9434765919")
///     .given_name(String::from("Owen"))   // owned String
///     .family_name("Williams")            // &str
///     .date_of_birth(NaiveDate::from_ymd_opt(1972, 11, 4).unwrap())
///     .gender(Gender::Male)
///     .build();
///
/// assert_eq!(p.united_kingdom_national_health_service_number.as_deref(), Some("9434765919"));
/// ```
#[derive(Default)]
pub struct PersonBuilder {
    /// Staged [`Person::united_kingdom_national_health_service_number`].
    united_kingdom_national_health_service_number: Option<String>,
    /// Staged [`Person::fr_nir`].
    fr_nir: Option<String>,
    /// Staged [`Person::es_tsi`].
    es_tsi: Option<String>,
    /// Staged [`Person::ie_ihi`].
    ie_ihi: Option<String>,
    /// Staged [`Person::uk_hc_number`].
    uk_hc_number: Option<String>,
    /// Staged [`Person::us_ssn`].
    us_ssn: Option<String>,
    /// Staged [`Person::au_ihi`].
    au_ihi: Option<String>,
    /// Staged [`Person::de_kvnr`].
    de_kvnr: Option<String>,
    /// Staged [`Person::it_cf`].
    it_cf: Option<String>,
    /// Staged [`Person::nl_bsn`].
    nl_bsn: Option<String>,
    /// Staged [`Person::se_personnummer`].
    se_personnummer: Option<String>,
    /// Staged [`Person::uk_chi_number`].
    uk_chi_number: Option<String>,
    /// Staged [`Person::be_nn`].
    be_nn: Option<String>,
    /// Staged [`Person::bg_egn`].
    bg_egn: Option<String>,
    /// Staged [`Person::cz_rc`].
    cz_rc: Option<String>,
    /// Staged [`Person::dk_cpr`].
    dk_cpr: Option<String>,
    /// Staged [`Person::ee_ik`].
    ee_ik: Option<String>,
    /// Staged [`Person::es_dni`].
    es_dni: Option<String>,
    /// Staged [`Person::fi_hetu`].
    fi_hetu: Option<String>,
    /// Staged [`Person::hr_oib`].
    hr_oib: Option<String>,
    /// Staged [`Person::is_kt`].
    is_kt: Option<String>,
    /// Staged [`Person::lt_ak`].
    lt_ak: Option<String>,
    /// Staged [`Person::lv_pk`].
    lv_pk: Option<String>,
    /// Staged [`Person::mt_id`].
    mt_id: Option<String>,
    /// Staged [`Person::no_fnr`].
    no_fnr: Option<String>,
    /// Staged [`Person::pl_pesel`].
    pl_pesel: Option<String>,
    /// Staged [`Person::ro_cnp`].
    ro_cnp: Option<String>,
    /// Staged [`Person::si_emso`].
    si_emso: Option<String>,
    /// Staged [`Person::sk_rc`].
    sk_rc: Option<String>,
    /// Staged [`Person::uk_nino`].
    uk_nino: Option<String>,
    /// Staged [`Person::gr_dss`].
    gr_dss: Option<String>,
    /// Staged [`Person::li_id`].
    li_id: Option<String>,
    /// Staged [`Person::nl_id`].
    nl_id: Option<String>,
    /// Staged [`Person::pl_nip`].
    pl_nip: Option<String>,
    /// Staged [`Person::pt_nif`].
    pt_nif: Option<String>,
    /// Staged [`Person::br_cpf`].
    br_cpf: Option<String>,
    /// Staged [`Person::cn_rrn`].
    cn_rrn: Option<String>,
    /// Staged [`Person::in_aadhaar`].
    in_aadhaar: Option<String>,
    /// Staged [`Person::jp_my_number`].
    jp_my_number: Option<String>,
    /// Staged [`Person::mx_curp`].
    mx_curp: Option<String>,
    /// Staged [`Person::nz_nhi`].
    nz_nhi: Option<String>,
    /// Staged [`Person::za_id`].
    za_id: Option<String>,
    /// Staged [`Person::given_name`].
    given_name: Option<String>,
    /// Staged [`Person::middle_name`].
    middle_name: Option<String>,
    /// Staged [`Person::family_name`].
    family_name: Option<String>,
    /// Staged [`Person::date_of_birth`].
    date_of_birth: Option<NaiveDate>,
    /// Staged [`Person::death_date`].
    death_date: Option<NaiveDate>,
    /// Staged [`Person::gender`].
    gender: Option<Gender>,
    /// Staged [`Person::blood_type`].
    blood_type: Option<BloodType>,
    /// Staged [`Person::multiple_birth`].
    multiple_birth: Option<u8>,
    /// Staged [`Person::address`].
    address: Option<Address>,
    /// Staged [`Person::birth_place`].
    birth_place: Option<Address>,
    /// Staged [`Person::death_place`].
    death_place: Option<Address>,
    /// Staged [`Person::previous_addresses`].
    previous_addresses: Vec<Address>,
    /// Staged [`Person::passport_books`].
    passport_books: Vec<PassportBook>,
    /// Staged [`Person::phone`].
    phone: Option<String>,
    /// Staged [`Person::mobile`].
    mobile: Option<String>,
    /// Staged [`Person::email`].
    email: Option<String>,
    /// Staged [`Person::local_id`].
    local_id: Option<String>,
}

impl PersonBuilder {
    /// Set the United Kingdom National Health Service Number (England, Wales,
    /// Isle of Man).
    ///
    /// The string is stored verbatim; normalisation and validation happen at
    /// match time via
    /// [`crate::identifiers::parse_united_kingdom_national_health_service_number`].
    /// Whitespace in the canonical `"XXX XXX XXXX"` layout is permitted.
    ///
    /// ```
    /// # use person_matcher::Person;
    /// let p = Person::builder().united_kingdom_national_health_service_number("943 476 5919").build();
    /// assert_eq!(p.united_kingdom_national_health_service_number.as_deref(), Some("943 476 5919"));
    /// ```
    pub fn united_kingdom_national_health_service_number<S: Into<String>>(
        mut self,
        value: S,
    ) -> Self {
        self.united_kingdom_national_health_service_number = Some(value.into());
        self
    }

    /// Set the France NIR (*Numéro d'Inscription au Répertoire*).
    ///
    /// The 15-character national identifier. Stored verbatim; parsing happens
    /// at match time via [`crate::identifiers::parse_fr_nir`].
    ///
    /// ```
    /// # use person_matcher::Person;
    /// let p = Person::builder().fr_nir("180127512345642").build();
    /// assert_eq!(p.fr_nir.as_deref(), Some("180127512345642"));
    /// ```
    pub fn fr_nir<S: Into<String>>(mut self, value: S) -> Self {
        self.fr_nir = Some(value.into());
        self
    }

    /// Set the España (Spain) TSI (*Tarjeta Sanitaria Individual*) / CIP-SNS
    /// identifier.
    ///
    /// Stored verbatim; parsing happens at match time via
    /// [`crate::identifiers::parse_es_tsi`].
    ///
    /// ```
    /// # use person_matcher::Person;
    /// let p = Person::builder().es_tsi("ABCD123456XY1234").build();
    /// assert_eq!(p.es_tsi.as_deref(), Some("ABCD123456XY1234"));
    /// ```
    pub fn es_tsi<S: Into<String>>(mut self, value: S) -> Self {
        self.es_tsi = Some(value.into());
        self
    }

    /// Set the Éire (Ireland) IHI (Individual Health Identifier).
    ///
    /// The 7-digit identifier. Stored verbatim; parsing happens at match time
    /// via [`crate::identifiers::parse_ie_ihi`].
    ///
    /// ```
    /// # use person_matcher::Person;
    /// let p = Person::builder().ie_ihi("1234567").build();
    /// assert_eq!(p.ie_ihi.as_deref(), Some("1234567"));
    /// ```
    pub fn ie_ihi<S: Into<String>>(mut self, value: S) -> Self {
        self.ie_ihi = Some(value.into());
        self
    }

    /// Set the United Kingdom Northern Ireland H&C (Health and Care) Number.
    ///
    /// A 10-digit Modulus-11 identifier sharing the United Kingdom National
    /// Health Service Number algorithm.
    /// Stored verbatim; parsing happens at match time via
    /// [`crate::identifiers::parse_uk_hc_number`].
    ///
    /// ```
    /// # use person_matcher::Person;
    /// let p = Person::builder().uk_hc_number("9434765919").build();
    /// assert_eq!(p.uk_hc_number.as_deref(), Some("9434765919"));
    /// ```
    pub fn uk_hc_number<S: Into<String>>(mut self, value: S) -> Self {
        self.uk_hc_number = Some(value.into());
        self
    }

    /// Set the United States Social Security Number (SSN).
    ///
    /// A 9-digit identifier issued by the Social Security Administration.
    /// Stored verbatim; parsing happens at match time via
    /// [`crate::identifiers::parse_us_ssn`]. The canonical
    /// `"AAA-GG-SSSS"` layout and the compact `"AAAGGSSSS"` layout are
    /// equivalent under parsing.
    ///
    /// ```
    /// # use person_matcher::Person;
    /// let p = Person::builder().us_ssn("123-45-6789").build();
    /// assert_eq!(p.us_ssn.as_deref(), Some("123-45-6789"));
    /// ```
    pub fn us_ssn<S: Into<String>>(mut self, value: S) -> Self {
        self.us_ssn = Some(value.into());
        self
    }

    /// Set the Australia IHI (Individual Healthcare Identifier).
    ///
    /// 16-digit identifier with a Luhn check, conforming to ISO/IEC
    /// 7812-1. Stored verbatim; parsing happens at match time via
    /// [`crate::identifiers::parse_au_ihi`].
    ///
    /// ```
    /// # use person_matcher::Person;
    /// let p = Person::builder().au_ihi("8003601234567894").build();
    /// assert_eq!(p.au_ihi.as_deref(), Some("8003601234567894"));
    /// ```
    pub fn au_ihi<S: Into<String>>(mut self, value: S) -> Self {
        self.au_ihi = Some(value.into());
        self
    }

    /// Set the Germany KVNR (*Krankenversichertennummer*).
    ///
    /// 10-character (1 letter + 9 digits) lifelong health-insurance
    /// number with a Mod-10 check. Stored verbatim; parsing happens at
    /// match time via [`crate::identifiers::parse_de_kvnr`].
    ///
    /// ```
    /// # use person_matcher::Person;
    /// let p = Person::builder().de_kvnr("A123456780").build();
    /// assert_eq!(p.de_kvnr.as_deref(), Some("A123456780"));
    /// ```
    pub fn de_kvnr<S: Into<String>>(mut self, value: S) -> Self {
        self.de_kvnr = Some(value.into());
        self
    }

    /// Set the Italy *Codice Fiscale* (CF).
    ///
    /// 16-character alphanumeric tax identifier with a Mod-26 check
    /// character. Stored verbatim; parsing happens at match time via
    /// [`crate::identifiers::parse_it_cf`].
    ///
    /// ```
    /// # use person_matcher::Person;
    /// let p = Person::builder().it_cf("RSSMRA85T10A562S").build();
    /// assert_eq!(p.it_cf.as_deref(), Some("RSSMRA85T10A562S"));
    /// ```
    pub fn it_cf<S: Into<String>>(mut self, value: S) -> Self {
        self.it_cf = Some(value.into());
        self
    }

    /// Set the Netherlands BSN (*Burgerservicenummer*).
    ///
    /// 9-digit citizen-service number with the "11-test" check rule.
    /// Stored verbatim; parsing happens at match time via
    /// [`crate::identifiers::parse_nl_bsn`].
    ///
    /// ```
    /// # use person_matcher::Person;
    /// let p = Person::builder().nl_bsn("111222333").build();
    /// assert_eq!(p.nl_bsn.as_deref(), Some("111222333"));
    /// ```
    pub fn nl_bsn<S: Into<String>>(mut self, value: S) -> Self {
        self.nl_bsn = Some(value.into());
        self
    }

    /// Set the Sweden *Personnummer*.
    ///
    /// 10- or 12-digit personal identity number with a Luhn check
    /// computed over the 10-digit form. Stored verbatim; parsing happens
    /// at match time via [`crate::identifiers::parse_se_personnummer`].
    ///
    /// ```
    /// # use person_matcher::Person;
    /// let p = Person::builder().se_personnummer("19460324-3850").build();
    /// assert_eq!(p.se_personnummer.as_deref(), Some("19460324-3850"));
    /// ```
    pub fn se_personnummer<S: Into<String>>(mut self, value: S) -> Self {
        self.se_personnummer = Some(value.into());
        self
    }

    /// Set the United Kingdom (Scotland) CHI Number (Community Health Index).
    ///
    /// 10-digit identifier issued by United Kingdom National Health Service Scotland, sharing the Mod-11
    /// algorithm of the United Kingdom National Health Service Number but
    /// scheme-local. Stored verbatim;
    /// parsing happens at match time via
    /// [`crate::identifiers::parse_uk_chi_number`].
    ///
    /// ```
    /// # use person_matcher::Person;
    /// let p = Person::builder().uk_chi_number("0101701233").build();
    /// assert_eq!(p.uk_chi_number.as_deref(), Some("0101701233"));
    /// ```
    pub fn uk_chi_number<S: Into<String>>(mut self, value: S) -> Self {
        self.uk_chi_number = Some(value.into());
        self
    }

    /// Set the Belgium National Number (*Rijksregisternummer*). 11 digits, Mod-97.
    pub fn be_nn<S: Into<String>>(mut self, value: S) -> Self {
        self.be_nn = Some(value.into());
        self
    }

    /// Set the Bulgaria EGN (*Edinen grazhdanski nomer*). 10 digits, weighted Mod-11.
    pub fn bg_egn<S: Into<String>>(mut self, value: S) -> Self {
        self.bg_egn = Some(value.into());
        self
    }

    /// Set the Czech Republic *Rodné číslo*. 9 or 10 digits.
    pub fn cz_rc<S: Into<String>>(mut self, value: S) -> Self {
        self.cz_rc = Some(value.into());
        self
    }

    /// Set the Denmark CPR (*Centrale Personregister*). 10 digits.
    pub fn dk_cpr<S: Into<String>>(mut self, value: S) -> Self {
        self.dk_cpr = Some(value.into());
        self
    }

    /// Set the Estonia *Isikukood* (Personal Identification Code). 11 digits.
    pub fn ee_ik<S: Into<String>>(mut self, value: S) -> Self {
        self.ee_ik = Some(value.into());
        self
    }

    /// Set the Spain DNI / NIE. 8 digits + Mod-23 letter.
    pub fn es_dni<S: Into<String>>(mut self, value: S) -> Self {
        self.es_dni = Some(value.into());
        self
    }

    /// Set the Finland HETU (*Henkilötunnus*). 11 chars with century sign.
    pub fn fi_hetu<S: Into<String>>(mut self, value: S) -> Self {
        self.fi_hetu = Some(value.into());
        self
    }

    /// Set the Croatia OIB (*Osobni identifikacijski broj*). 11 digits.
    pub fn hr_oib<S: Into<String>>(mut self, value: S) -> Self {
        self.hr_oib = Some(value.into());
        self
    }

    /// Set the Iceland *Kennitala*. 10 digits.
    pub fn is_kt<S: Into<String>>(mut self, value: S) -> Self {
        self.is_kt = Some(value.into());
        self
    }

    /// Set the Lithuania *Asmens kodas*. 11 digits.
    pub fn lt_ak<S: Into<String>>(mut self, value: S) -> Self {
        self.lt_ak = Some(value.into());
        self
    }

    /// Set the Latvia *Personas kods*. 11 digits.
    pub fn lv_pk<S: Into<String>>(mut self, value: S) -> Self {
        self.lv_pk = Some(value.into());
        self
    }

    /// Set the Malta National ID. 7 digits + letter.
    pub fn mt_id<S: Into<String>>(mut self, value: S) -> Self {
        self.mt_id = Some(value.into());
        self
    }

    /// Set the Norway *Fødselsnummer*. 11 digits, dual Mod-11.
    pub fn no_fnr<S: Into<String>>(mut self, value: S) -> Self {
        self.no_fnr = Some(value.into());
        self
    }

    /// Set the Poland PESEL. 11 digits, weighted Mod-10.
    pub fn pl_pesel<S: Into<String>>(mut self, value: S) -> Self {
        self.pl_pesel = Some(value.into());
        self
    }

    /// Set the Romania CNP (*Cod Numeric Personal*). 13 digits.
    pub fn ro_cnp<S: Into<String>>(mut self, value: S) -> Self {
        self.ro_cnp = Some(value.into());
        self
    }

    /// Set the Slovenia EMŠO (*Enotna Matična Številka Občana*). 13 digits.
    pub fn si_emso<S: Into<String>>(mut self, value: S) -> Self {
        self.si_emso = Some(value.into());
        self
    }

    /// Set the Slovakia *Rodné číslo*. 9 or 10 digits.
    pub fn sk_rc<S: Into<String>>(mut self, value: S) -> Self {
        self.sk_rc = Some(value.into());
        self
    }

    /// Set the United Kingdom National Insurance Number (NINO).
    pub fn uk_nino<S: Into<String>>(mut self, value: S) -> Self {
        self.uk_nino = Some(value.into());
        self
    }

    /// Set the Greece DSS investor share code. 10 digits.
    pub fn gr_dss<S: Into<String>>(mut self, value: S) -> Self {
        self.gr_dss = Some(value.into());
        self
    }

    /// Set the Liechtenstein National Identity Card Number. 2 letters + 8 digits.
    pub fn li_id<S: Into<String>>(mut self, value: S) -> Self {
        self.li_id = Some(value.into());
        self
    }

    /// Set the Netherlands National Identity Card Number. 9 chars per spec.
    pub fn nl_id<S: Into<String>>(mut self, value: S) -> Self {
        self.nl_id = Some(value.into());
        self
    }

    /// Set the Poland NIP (*Numer Identyfikacji Podatkowej*). 10 digits, weighted Mod-11.
    pub fn pl_nip<S: Into<String>>(mut self, value: S) -> Self {
        self.pl_nip = Some(value.into());
        self
    }

    /// Set the Portugal NIF (*Número de Identificação Fiscal*). 9 digits, weighted Mod-11.
    pub fn pt_nif<S: Into<String>>(mut self, value: S) -> Self {
        self.pt_nif = Some(value.into());
        self
    }

    /// Set the Brazil CPF (*Cadastro de Pessoas Físicas*). 11 digits, two Mod-11 check digits.
    pub fn br_cpf<S: Into<String>>(mut self, value: S) -> Self {
        self.br_cpf = Some(value.into());
        self
    }

    /// Set the China Resident Identity Card number (*居民身份证*). 18 chars, weighted Mod-11 + date substring.
    pub fn cn_rrn<S: Into<String>>(mut self, value: S) -> Self {
        self.cn_rrn = Some(value.into());
        self
    }

    /// Set the India Aadhaar number. 12 digits, Verhoeff check digit.
    pub fn in_aadhaar<S: Into<String>>(mut self, value: S) -> Self {
        self.in_aadhaar = Some(value.into());
        self
    }

    /// Set the Japan My Number (*個人番号*). 12 digits, weighted Mod-11 check digit.
    pub fn jp_my_number<S: Into<String>>(mut self, value: S) -> Self {
        self.jp_my_number = Some(value.into());
        self
    }

    /// Set the Mexico CURP. 18 alphanumeric chars, structural + Mod-10 check digit.
    pub fn mx_curp<S: Into<String>>(mut self, value: S) -> Self {
        self.mx_curp = Some(value.into());
        self
    }

    /// Set the New Zealand NHI Number. Original 7-char format (3 letters + 4 digits).
    pub fn nz_nhi<S: Into<String>>(mut self, value: S) -> Self {
        self.nz_nhi = Some(value.into());
        self
    }

    /// Set the South Africa ID Number. 13 digits, Luhn + date substring.
    pub fn za_id<S: Into<String>>(mut self, value: S) -> Self {
        self.za_id = Some(value.into());
        self
    }

    /// Set the given name (forename).
    ///
    /// ```
    /// # use person_matcher::Person;
    /// let p = Person::builder().given_name("Carys").build();
    /// assert_eq!(p.given_name.as_deref(), Some("Carys"));
    /// ```
    pub fn given_name<S: Into<String>>(mut self, value: S) -> Self {
        self.given_name = Some(value.into());
        self
    }

    /// Set the middle name(s).
    ///
    /// Stored on the person but not currently used in matching scoring
    /// (see spec OQ-1).
    ///
    /// ```
    /// # use person_matcher::Person;
    /// let p = Person::builder().middle_name("Eleri").build();
    /// assert_eq!(p.middle_name.as_deref(), Some("Eleri"));
    /// ```
    pub fn middle_name<S: Into<String>>(mut self, value: S) -> Self {
        self.middle_name = Some(value.into());
        self
    }

    /// Set the family name (surname).
    ///
    /// ```
    /// # use person_matcher::Person;
    /// let p = Person::builder().family_name("Pritchard").build();
    /// assert_eq!(p.family_name.as_deref(), Some("Pritchard"));
    /// ```
    pub fn family_name<S: Into<String>>(mut self, value: S) -> Self {
        self.family_name = Some(value.into());
        self
    }

    /// Set the date of birth.
    ///
    /// ```
    /// # use person_matcher::Person;
    /// use chrono::NaiveDate;
    /// let dob = NaiveDate::from_ymd_opt(1990, 1, 1).unwrap();
    /// let p = Person::builder().date_of_birth(dob).build();
    /// assert_eq!(p.date_of_birth, Some(dob));
    /// ```
    #[must_use]
    pub fn date_of_birth(mut self, value: NaiveDate) -> Self {
        self.date_of_birth = Some(value);
        self
    }

    /// Set the date of death (FHIR `Patient.deceasedDateTime`).
    ///
    /// ```
    /// # use person_matcher::Person;
    /// use chrono::NaiveDate;
    /// let dod = NaiveDate::from_ymd_opt(2024, 6, 30).unwrap();
    /// let p = Person::builder().death_date(dod).build();
    /// assert_eq!(p.death_date, Some(dod));
    /// ```
    #[must_use]
    pub fn death_date(mut self, value: NaiveDate) -> Self {
        self.death_date = Some(value);
        self
    }

    /// Set the recorded gender.
    ///
    /// ```
    /// # use person_matcher::{Gender, Person};
    /// let p = Person::builder().gender(Gender::Female).build();
    /// assert_eq!(p.gender, Some(Gender::Female));
    /// ```
    #[must_use]
    pub fn gender(mut self, value: Gender) -> Self {
        self.gender = Some(value);
        self
    }

    /// Set the recorded ABO+RhD blood type.
    ///
    /// ```
    /// # use person_matcher::{BloodType, Person};
    /// let p = Person::builder().blood_type(BloodType::OPositive).build();
    /// assert_eq!(p.blood_type, Some(BloodType::OPositive));
    /// ```
    #[must_use]
    pub fn blood_type(mut self, value: BloodType) -> Self {
        self.blood_type = Some(value);
        self
    }

    /// Set the multiple-birth indicator (FHIR `Patient.multipleBirth`).
    ///
    /// The value is the 1-indexed birth order within a multiple-birth
    /// set: `1` for the first born, `2` for the second, and so on.
    /// `0` is conventionally not used; consumers should pass `None`
    /// (do not call this setter) for singletons or unknown values.
    ///
    /// ```
    /// # use person_matcher::Person;
    /// // First of identical twins.
    /// let p = Person::builder().multiple_birth(1).build();
    /// assert_eq!(p.multiple_birth, Some(1));
    /// ```
    #[must_use]
    pub fn multiple_birth(mut self, value: u8) -> Self {
        self.multiple_birth = Some(value);
        self
    }

    /// Set the current residential address.
    ///
    /// ```
    /// # use person_matcher::{Address, Person};
    /// let mut a = Address::new();
    /// a.postcode = Some("CF10 1AA".into());
    /// let p = Person::builder().address(a).build();
    /// assert_eq!(p.address.unwrap().postcode.as_deref(), Some("CF10 1AA"));
    /// ```
    #[must_use]
    pub fn address(mut self, value: Address) -> Self {
        self.address = Some(value);
        self
    }

    /// Set the place of birth (FHIR `Patient.birthPlace`).
    ///
    /// Typically only [`Address::city`] and [`Address::country`] are
    /// populated for a birth place.
    ///
    /// ```
    /// # use person_matcher::{Address, Person};
    /// let p = Person::builder()
    ///     .birth_place(Address::new().with_city("Cardiff").with_country("Wales"))
    ///     .build();
    /// assert_eq!(p.birth_place.as_ref().unwrap().city.as_deref(), Some("Cardiff"));
    /// ```
    #[must_use]
    pub fn birth_place(mut self, value: Address) -> Self {
        self.birth_place = Some(value);
        self
    }

    /// Set the place of death.
    ///
    /// Modelled as an [`Address`] for parity with [`Self::birth_place`]
    /// — typically only [`Address::city`] and [`Address::country`] are
    /// populated.
    ///
    /// ```
    /// # use person_matcher::{Address, Person};
    /// let p = Person::builder()
    ///     .death_place(Address::new().with_city("Glasgow").with_country("Scotland"))
    ///     .build();
    /// assert_eq!(p.death_place.as_ref().unwrap().city.as_deref(), Some("Glasgow"));
    /// ```
    #[must_use]
    pub fn death_place(mut self, value: Address) -> Self {
        self.death_place = Some(value);
        self
    }

    /// Set the list of previous addresses. Used by the address
    /// sub-score (best-of cartesian product, see spec §12.4.2).
    ///
    /// ```
    /// # use person_matcher::{Address, Person};
    /// let p = Person::builder()
    ///     .previous_addresses(vec![Address::new(), Address::new()])
    ///     .build();
    /// assert_eq!(p.previous_addresses.len(), 2);
    /// ```
    #[must_use]
    pub fn previous_addresses(mut self, value: Vec<Address>) -> Self {
        self.previous_addresses = value;
        self
    }

    /// Append a single passport book to the person's list. Chainable;
    /// call multiple times to record multi-country or historical
    /// books.
    ///
    /// ```
    /// # use person_matcher::{PassportBook, Person};
    /// let p = Person::builder()
    ///     .add_passport_book(PassportBook::new("GB", "123456789").unwrap())
    ///     .add_passport_book(PassportBook::new("US", "AB1234567").unwrap())
    ///     .build();
    /// assert_eq!(p.passport_books.len(), 2);
    /// ```
    #[must_use]
    pub fn add_passport_book(mut self, book: PassportBook) -> Self {
        self.passport_books.push(book);
        self
    }

    /// Replace the entire passport-book list.
    ///
    /// ```
    /// # use person_matcher::{PassportBook, Person};
    /// let books = vec![PassportBook::new("GB", "123456789").unwrap()];
    /// let p = Person::builder().passport_books(books).build();
    /// assert_eq!(p.passport_books.len(), 1);
    /// ```
    #[must_use]
    pub fn passport_books(mut self, value: Vec<PassportBook>) -> Self {
        self.passport_books = value;
        self
    }

    /// Set the primary phone number.
    ///
    /// ```
    /// # use person_matcher::Person;
    /// let p = Person::builder().phone("029 2034 5678").build();
    /// assert_eq!(p.phone.as_deref(), Some("029 2034 5678"));
    /// ```
    pub fn phone<S: Into<String>>(mut self, value: S) -> Self {
        self.phone = Some(value.into());
        self
    }

    /// Set the mobile phone number. Used as a fallback when `phone` is absent.
    ///
    /// ```
    /// # use person_matcher::Person;
    /// let p = Person::builder().mobile("07700 900123").build();
    /// assert_eq!(p.mobile.as_deref(), Some("07700 900123"));
    /// ```
    pub fn mobile<S: Into<String>>(mut self, value: S) -> Self {
        self.mobile = Some(value.into());
        self
    }

    /// Set the email address. Not currently used in scoring.
    ///
    /// ```
    /// # use person_matcher::Person;
    /// let p = Person::builder().email("alice@example.org").build();
    /// assert_eq!(p.email.as_deref(), Some("alice@example.org"));
    /// ```
    pub fn email<S: Into<String>>(mut self, value: S) -> Self {
        self.email = Some(value.into());
        self
    }

    /// Set the local hospital or practice identifier.
    ///
    /// ```
    /// # use person_matcher::Person;
    /// let p = Person::builder().local_id("MRN-12345").build();
    /// assert_eq!(p.local_id.as_deref(), Some("MRN-12345"));
    /// ```
    pub fn local_id<S: Into<String>>(mut self, value: S) -> Self {
        self.local_id = Some(value.into());
        self
    }

    /// Consume the builder and produce the [`Person`].
    ///
    /// ```
    /// # use person_matcher::Person;
    /// let p = Person::builder().given_name("Eira").build();
    /// assert!(p.family_name.is_none());
    /// ```
    #[must_use]
    pub fn build(self) -> Person {
        Person {
            united_kingdom_national_health_service_number: self
                .united_kingdom_national_health_service_number,
            fr_nir: self.fr_nir,
            es_tsi: self.es_tsi,
            ie_ihi: self.ie_ihi,
            uk_hc_number: self.uk_hc_number,
            us_ssn: self.us_ssn,
            au_ihi: self.au_ihi,
            de_kvnr: self.de_kvnr,
            it_cf: self.it_cf,
            nl_bsn: self.nl_bsn,
            se_personnummer: self.se_personnummer,
            uk_chi_number: self.uk_chi_number,
            be_nn: self.be_nn,
            bg_egn: self.bg_egn,
            cz_rc: self.cz_rc,
            dk_cpr: self.dk_cpr,
            ee_ik: self.ee_ik,
            es_dni: self.es_dni,
            fi_hetu: self.fi_hetu,
            hr_oib: self.hr_oib,
            is_kt: self.is_kt,
            lt_ak: self.lt_ak,
            lv_pk: self.lv_pk,
            mt_id: self.mt_id,
            no_fnr: self.no_fnr,
            pl_pesel: self.pl_pesel,
            ro_cnp: self.ro_cnp,
            si_emso: self.si_emso,
            sk_rc: self.sk_rc,
            uk_nino: self.uk_nino,
            gr_dss: self.gr_dss,
            li_id: self.li_id,
            nl_id: self.nl_id,
            pl_nip: self.pl_nip,
            pt_nif: self.pt_nif,
            br_cpf: self.br_cpf,
            cn_rrn: self.cn_rrn,
            in_aadhaar: self.in_aadhaar,
            jp_my_number: self.jp_my_number,
            mx_curp: self.mx_curp,
            nz_nhi: self.nz_nhi,
            za_id: self.za_id,
            given_name: self.given_name,
            middle_name: self.middle_name,
            family_name: self.family_name,
            date_of_birth: self.date_of_birth,
            death_date: self.death_date,
            gender: self.gender,
            blood_type: self.blood_type,
            multiple_birth: self.multiple_birth,
            address: self.address,
            birth_place: self.birth_place,
            death_place: self.death_place,
            previous_addresses: self.previous_addresses,
            passport_books: self.passport_books,
            phone: self.phone,
            mobile: self.mobile,
            email: self.email,
            local_id: self.local_id,
        }
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for the logic-free model layer: they pin the
    //! construction, canonicalisation, validation, and serde round-trip
    //! contracts that the matcher relies on. Coverage spans `Address`
    //! (constructors + fluent setters + serde), `Person` / `PersonBuilder`
    //! (empty defaults, identifier carry-through, `Into<String>`
    //! ergonomics, the "at least one identifying anchor" validate rule,
    //! serde), `BloodType` (canonical / word / separator / zero-as-O / VE
    //! parsing, rejection of bad input, `as_str` ↔ `Display` ↔ serde
    //! agreement), `Gender` (`Copy` + equality), and `PassportBook`
    //! (country/number canonicalisation, separator stripping, rejection
    //! rules, optional date metadata, serde with defaulted dates).

    use super::*;

    /// A freshly constructed `Address` must leave every field `None`, so
    /// partial addresses start from a clean slate and never carry stray
    /// defaults into matching.
    #[test]
    fn address_new_is_all_none() {
        let a = Address::new();
        assert!(a.line1.is_none());
        assert!(a.line2.is_none());
        assert!(a.city.is_none());
        assert!(a.county.is_none());
        assert!(a.postcode.is_none());
        assert!(a.country.is_none());
    }

    /// `Default` must be byte-for-byte identical to `new`, pinning the
    /// documented promise that the two constructors are interchangeable.
    #[test]
    fn address_default_matches_new() {
        assert_eq!(Address::default(), Address::new());
    }

    /// The fluent `with_*` setters must chain and set exactly the fields
    /// named — and leave the un-set fields (`line2`, `county`) `None`.
    /// This guards against a copy-paste setter writing the wrong field.
    #[test]
    fn address_fluent_builders_chain() {
        let a = Address::new()
            .with_line1("10 Downing Street")
            .with_city("London")
            .with_postcode("SW1A 2AA")
            .with_country("United Kingdom");
        assert_eq!(a.line1.as_deref(), Some("10 Downing Street"));
        assert_eq!(a.city.as_deref(), Some("London"));
        assert_eq!(a.postcode.as_deref(), Some("SW1A 2AA"));
        assert_eq!(a.country.as_deref(), Some("United Kingdom"));
        assert!(a.line2.is_none());
        assert!(a.county.is_none());
    }

    /// `Address` must survive a JSON encode/decode unchanged; the model
    /// is shared across services over the wire, so serde fidelity is a
    /// hard contract.
    #[test]
    fn address_round_trips_through_serde() {
        let mut a = Address::new();
        a.line1 = Some("123 High Street".into());
        a.postcode = Some("CF10 1AA".into());
        let json = serde_json::to_string(&a).expect("serialise");
        let back: Address = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(a, back);
    }

    /// A builder with no setters called must produce an all-absent
    /// `Person`: every identifier `None`, every collection empty. This is
    /// the baseline the matcher assumes — a `None` field must never
    /// penalise, so it must never silently default to a populated value.
    #[test]
    fn person_builder_starts_empty() {
        let p = Person::builder().build();
        assert!(p.united_kingdom_national_health_service_number.is_none());
        assert!(p.fr_nir.is_none());
        assert!(p.es_tsi.is_none());
        assert!(p.ie_ihi.is_none());
        assert!(p.uk_hc_number.is_none());
        assert!(p.us_ssn.is_none());
        assert!(p.au_ihi.is_none());
        assert!(p.de_kvnr.is_none());
        assert!(p.it_cf.is_none());
        assert!(p.nl_bsn.is_none());
        assert!(p.se_personnummer.is_none());
        assert!(p.uk_chi_number.is_none());
        assert!(p.given_name.is_none());
        assert!(p.family_name.is_none());
        assert!(p.date_of_birth.is_none());
        assert!(p.gender.is_none());
        assert!(p.address.is_none());
        assert!(p.previous_addresses.is_empty());
        assert!(p.passport_books.is_empty());
        assert!(p.phone.is_none());
        assert!(p.mobile.is_none());
        assert!(p.email.is_none());
        assert!(p.local_id.is_none());
    }

    /// Each national-identifier setter must land in its own field with no
    /// cross-wiring. Because the schemes are scheme-local and must never
    /// cross-match, a setter writing the wrong field would be a silent
    /// correctness bug; this pins all twelve to their distinct slots.
    #[test]
    fn person_builder_carries_all_national_identifiers() {
        let p = Person::builder()
            .united_kingdom_national_health_service_number("9434765919")
            .fr_nir("180127512345642")
            .es_tsi("ABCD123456XY1234")
            .ie_ihi("1234567")
            .uk_hc_number("9434765919")
            .us_ssn("123-45-6789")
            .au_ihi("8003601234567894")
            .de_kvnr("A123456780")
            .it_cf("RSSMRA85T10A562S")
            .nl_bsn("111222333")
            .se_personnummer("4603243850")
            .uk_chi_number("0101701233")
            .build();
        assert_eq!(
            p.united_kingdom_national_health_service_number.as_deref(),
            Some("9434765919")
        );
        assert_eq!(p.fr_nir.as_deref(), Some("180127512345642"));
        assert_eq!(p.es_tsi.as_deref(), Some("ABCD123456XY1234"));
        assert_eq!(p.ie_ihi.as_deref(), Some("1234567"));
        assert_eq!(p.uk_hc_number.as_deref(), Some("9434765919"));
        assert_eq!(p.us_ssn.as_deref(), Some("123-45-6789"));
        assert_eq!(p.au_ihi.as_deref(), Some("8003601234567894"));
        assert_eq!(p.de_kvnr.as_deref(), Some("A123456780"));
        assert_eq!(p.it_cf.as_deref(), Some("RSSMRA85T10A562S"));
        assert_eq!(p.nl_bsn.as_deref(), Some("111222333"));
        assert_eq!(p.se_personnummer.as_deref(), Some("4603243850"));
        assert_eq!(p.uk_chi_number.as_deref(), Some("0101701233"));
    }

    /// The `impl Into<String>` setters must accept both `&str` and owned
    /// `String` at the same call site, confirming the documented
    /// ergonomic contract that callers need no explicit conversion.
    #[test]
    fn person_builder_accepts_str_and_string() {
        let p = Person::builder()
            .given_name("Owen") // borrowed &str
            .family_name(String::from("Jones")) // owned String
            .build();
        assert_eq!(p.given_name.as_deref(), Some("Owen"));
        assert_eq!(p.family_name.as_deref(), Some("Jones"));
    }

    /// `validate` must accept a record anchored by a given name, a family
    /// name, OR a national identifier, and must reject a wholly empty
    /// record with `MissingField`. This pins the boundary-ingest rule that
    /// keeps anonymous (un-matchable) records out of the index.
    #[test]
    fn person_validate_requires_one_of_three_fields() {
        assert!(Person::builder().given_name("a").build().validate().is_ok());
        assert!(
            Person::builder()
                .family_name("a")
                .build()
                .validate()
                .is_ok()
        );
        assert!(
            Person::builder()
                .united_kingdom_national_health_service_number("9434765919")
                .build()
                .validate()
                .is_ok()
        );
        let err = Person::builder()
            .build()
            .validate()
            .expect_err("should be missing");
        assert!(matches!(err, crate::MatchingError::MissingField(_)));
    }

    /// A populated `Person` must round-trip through JSON unchanged,
    /// including its `Date` and `Gender` fields. Person records cross
    /// service boundaries as JSON, so any serde drift would silently
    /// corrupt identity data.
    #[test]
    fn person_round_trips_through_serde() {
        let p = Person::builder()
            .united_kingdom_national_health_service_number("9434765919")
            .given_name("Carys")
            .family_name("Pritchard")
            .date_of_birth(NaiveDate::from_ymd_opt(1990, 6, 1).unwrap())
            .gender(Gender::Female)
            .build();
        let json = serde_json::to_string(&p).expect("serialise");
        let back: Person = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(p, back);
    }

    // ---------- BloodType ----------

    /// Every one of the eight canonical short forms must parse to its
    /// matching variant. This is the baseline the looser textual-form
    /// parsing builds on.
    #[test]
    fn blood_type_parses_canonical_short_forms() {
        for (s, want) in [
            ("A+", BloodType::APositive),
            ("A-", BloodType::ANegative),
            ("B+", BloodType::BPositive),
            ("B-", BloodType::BNegative),
            ("AB+", BloodType::ABPositive),
            ("AB-", BloodType::ABNegative),
            ("O+", BloodType::OPositive),
            ("O-", BloodType::ONegative),
        ] {
            assert_eq!(BloodType::parse(s), Some(want), "parse {s:?}");
        }
    }

    /// Parsing must be case-insensitive and tolerate surrounding
    /// whitespace, since real EMR exports are inconsistently cased and
    /// padded.
    #[test]
    fn blood_type_parses_lowercase_and_whitespace() {
        assert_eq!(BloodType::parse("  a+ "), Some(BloodType::APositive));
        assert_eq!(BloodType::parse("ab-"), Some(BloodType::ABNegative));
    }

    /// Word-form Rhesus signs (`positive`/`pos`, `negative`/`neg`, in any
    /// case) must parse, covering the common free-text layouts found in
    /// clinical data alongside the symbolic `+`/`-` forms.
    #[test]
    fn blood_type_parses_word_forms() {
        assert_eq!(BloodType::parse("A positive"), Some(BloodType::APositive));
        assert_eq!(BloodType::parse("A pos"), Some(BloodType::APositive));
        assert_eq!(BloodType::parse("A POS"), Some(BloodType::APositive));
        assert_eq!(BloodType::parse("A negative"), Some(BloodType::ANegative));
        assert_eq!(BloodType::parse("ab neg"), Some(BloodType::ABNegative));
        assert_eq!(BloodType::parse("o NEG"), Some(BloodType::ONegative));
    }

    /// The digit `0` must be read as the letter `O`, recovering the
    /// extremely common zero/O transcription confusion for the O blood
    /// group rather than rejecting it as unparseable.
    #[test]
    fn blood_type_parses_zero_as_o() {
        assert_eq!(BloodType::parse("0+"), Some(BloodType::OPositive)); // '0' -> 'O'
        assert_eq!(BloodType::parse("0-"), Some(BloodType::ONegative));
    }

    /// A single separator (`_`, `-`, or space) between the ABO group and
    /// the sign must be tolerated. Note `A-neg` parses as A negative: the
    /// leading `-` is consumed as a separator because the word `neg`
    /// disambiguates the sign.
    #[test]
    fn blood_type_parses_with_separator() {
        assert_eq!(BloodType::parse("A_pos"), Some(BloodType::APositive));
        assert_eq!(BloodType::parse("A-neg"), Some(BloodType::ANegative)); // '-' is separator, 'neg' is sign
        assert_eq!(BloodType::parse("AB +"), Some(BloodType::ABPositive));
    }

    /// The British `+VE` / `-VE` shorthand (e.g. `A+VE`) must parse. The
    /// trailing `VE` is stripped after the sign so it does not look like
    /// unparseable trailing junk.
    #[test]
    fn blood_type_parses_ve_suffix() {
        assert_eq!(BloodType::parse("A+VE"), Some(BloodType::APositive));
        assert_eq!(BloodType::parse("a-ve"), Some(BloodType::ANegative));
    }

    /// Unparseable input must yield `None`, never a wrong guess. Covers
    /// empty / whitespace-only, an unknown group letter, a group with no
    /// sign, a rare phenotype name, and a doubled sign — returning `None`
    /// lets callers preserve the raw value rather than mis-record it.
    #[test]
    fn blood_type_rejects_unparseable() {
        assert_eq!(BloodType::parse(""), None);
        assert_eq!(BloodType::parse("   "), None);
        assert_eq!(BloodType::parse("Z+"), None); // 'Z' is not an ABO group
        assert_eq!(BloodType::parse("A"), None); // group present but no sign
        assert_eq!(BloodType::parse("Bombay"), None); // rare phenotype, unsupported
        assert_eq!(BloodType::parse("A++"), None); // doubled sign is invalid
    }

    /// For every variant, `as_str`, `Display`, and `parse` must agree:
    /// the string `as_str` emits must format identically via `Display`
    /// and parse back to the same variant. This pins the three surfaces
    /// as one canonical form and catches any future divergence.
    #[test]
    fn blood_type_as_str_and_display_round_trip() {
        for bt in [
            BloodType::APositive,
            BloodType::ANegative,
            BloodType::BPositive,
            BloodType::BNegative,
            BloodType::ABPositive,
            BloodType::ABNegative,
            BloodType::OPositive,
            BloodType::ONegative,
        ] {
            let s = bt.as_str();
            assert_eq!(format!("{bt}"), s);
            assert_eq!(BloodType::parse(s), Some(bt));
        }
    }

    /// The serde representation must be the canonical short form
    /// (`"A+"`, not the Rust variant name), and must deserialise back to
    /// the same variant. This is the on-the-wire contract other services
    /// depend on.
    #[test]
    fn blood_type_serde_uses_short_form() {
        for (bt, json) in [
            (BloodType::APositive, "\"A+\""),
            (BloodType::ABNegative, "\"AB-\""),
            (BloodType::ONegative, "\"O-\""),
        ] {
            assert_eq!(serde_json::to_string(&bt).unwrap(), json);
            let back: BloodType = serde_json::from_str(json).unwrap();
            assert_eq!(back, bt);
        }
    }

    /// The `blood_type` setter must store the value on the built person,
    /// confirming the optional supporting-evidence field is wired through
    /// the builder.
    #[test]
    fn person_builder_sets_blood_type() {
        let p = Person::builder().blood_type(BloodType::OPositive).build();
        assert_eq!(p.blood_type, Some(BloodType::OPositive));
    }

    /// Blood type must default to `None`, never a populated value —
    /// since blood-type *disagreement* is a strong negative signal, an
    /// accidental default would manufacture spurious mismatches.
    #[test]
    fn person_default_has_no_blood_type() {
        let p = Person::builder().build();
        assert!(p.blood_type.is_none());
    }

    /// `Gender` must be `Copy` (assigning `h = g` leaves `g` usable) and
    /// compare by value. Cheap by-value passing is the documented
    /// ergonomic the matcher relies on.
    #[test]
    fn gender_is_copy_and_eq() {
        let g = Gender::Female;
        let h = g; // Copy: g is still usable below
        assert_eq!(g, h);
        assert_ne!(g, Gender::Male);
    }

    // ---------- PassportBook ----------

    /// Construction must canonicalise the country (trim + uppercase) and
    /// the number (strip whitespace + uppercase) so two records carrying
    /// different textual layouts of the same book collapse to one
    /// `(country, number)` matching key.
    #[test]
    fn passport_book_new_canonicalises_country_and_number() {
        let b = PassportBook::new("  gb  ", " 123 ABC 789 ").unwrap();
        assert_eq!(b.country, "GB"); // trimmed + uppercased
        assert_eq!(b.number, "123ABC789"); // inner whitespace stripped
    }

    /// The common data-entry separators (hyphen, period, slash, plus
    /// whitespace) must all be stripped from the number, matching the
    /// canonicalisation used by the national-identifier parsers so the
    /// same book entered in different layouts keys identically.
    #[test]
    fn passport_book_new_strips_common_separators() {
        // Hyphens, periods, slashes and whitespace all stripped.
        let b = PassportBook::new("GB", "ABC-123/456.789").unwrap();
        assert_eq!(b.number, "ABC123456789");
        let c = PassportBook::new("US", "AB-12-34-567").unwrap();
        assert_eq!(c.number, "AB1234567");
    }

    /// The country code must be exactly two ASCII letters (ISO 3166-1
    /// alpha-2). Anything longer, shorter, non-alphabetic, or empty is
    /// rejected, since provenance correctness is what stops cross-country
    /// books with the same number from cross-matching.
    #[test]
    fn passport_book_new_rejects_bad_country() {
        assert!(PassportBook::new("GBR", "123").is_none()); // 3 letters
        assert!(PassportBook::new("G", "123").is_none()); // 1 letter
        assert!(PassportBook::new("1A", "123").is_none()); // not alphabetic
        assert!(PassportBook::new("", "123").is_none()); // empty
    }

    /// A number that canonicalises to an empty string (truly empty, or
    /// only whitespace) must be rejected — an empty key would match
    /// indiscriminately, so it must never be constructible.
    #[test]
    fn passport_book_new_rejects_empty_number() {
        assert!(PassportBook::new("GB", "").is_none());
        assert!(PassportBook::new("GB", "   ").is_none());
        assert!(PassportBook::new("GB", "\t\n").is_none()); // whitespace-only
    }

    /// The `with_issued` / `with_expires` setters must attach the
    /// optional dates. These are display/audit metadata only (not used in
    /// matching), but the builders must still store them.
    #[test]
    fn passport_book_with_dates_sets_metadata() {
        let b = PassportBook::new("GB", "123")
            .unwrap()
            .with_issued(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap())
            .with_expires(NaiveDate::from_ymd_opt(2030, 1, 1).unwrap());
        assert!(b.issued.is_some());
        assert!(b.expires.is_some());
    }

    /// A `PassportBook` with a populated date must round-trip through
    /// JSON unchanged, confirming serde fidelity for the date-carrying
    /// shape.
    #[test]
    fn passport_book_round_trips_through_serde() {
        let b = PassportBook::new("US", "AB1234567")
            .unwrap()
            .with_issued(NaiveDate::from_ymd_opt(2024, 6, 1).unwrap());
        let json = serde_json::to_string(&b).unwrap();
        let back: PassportBook = serde_json::from_str(&json).unwrap();
        assert_eq!(b, back);
    }

    /// Payloads written before the optional `issued` / `expires` fields
    /// existed must still deserialise, with the missing dates defaulting
    /// to `None`. This pins backward compatibility of the wire format.
    #[test]
    fn passport_book_serde_default_dates() {
        // Legacy payloads lacking the optional date fields must
        // deserialise cleanly.
        let legacy = r#"{"country": "GB", "number": "123"}"#;
        let b: PassportBook = serde_json::from_str(legacy).unwrap();
        assert_eq!(b.country, "GB");
        assert_eq!(b.number, "123");
        assert!(b.issued.is_none());
        assert!(b.expires.is_none());
    }

    /// Repeated `add_passport_book` calls must accumulate in order,
    /// supporting the multi-country / multi-book model (a person may hold
    /// several passports at once and across renewals).
    #[test]
    fn person_builder_carries_passport_books() {
        let p = Person::builder()
            .add_passport_book(PassportBook::new("GB", "111").unwrap())
            .add_passport_book(PassportBook::new("US", "222").unwrap())
            .build();
        assert_eq!(p.passport_books.len(), 2);
        assert_eq!(p.passport_books[0].country, "GB");
        assert_eq!(p.passport_books[1].country, "US");
    }

    /// A single passport book, with no name and no national identifier,
    /// must satisfy `validate` — a passport is an identifying anchor in
    /// its own right.
    #[test]
    fn person_validate_accepts_solo_passport_book() {
        let p = Person::builder()
            .add_passport_book(PassportBook::new("GB", "123456789").unwrap())
            .build();
        assert!(p.validate().is_ok());
    }

    /// The `previous_addresses` setter must *replace* (not append to) the
    /// list, matching its documented contract and distinguishing it from
    /// the additive `add_passport_book`.
    #[test]
    fn previous_addresses_setter_replaces_vec() {
        let mut a = Address::new();
        a.postcode = Some("CF10 1AA".into());
        let p = Person::builder()
            .previous_addresses(vec![a.clone()])
            .build();
        assert_eq!(p.previous_addresses, vec![a]);
    }
}
