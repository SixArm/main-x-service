//! Text normalisation for person demographic data.
//!
//! Research on person identification (see `spec.md` §5) is unanimous: most
//! accuracy gains come from **standardising the input** before scoring, not
//! from cleverer similarity algorithms. This module exposes the canonical
//! transformations the matching engine applies to names, postcodes, phone
//! numbers, and phonetic codes.
//!
//! All transformations are **idempotent**: `f(f(x)) == f(x)`. They are also
//! **deterministic** and allocate at most a single new `String`.
//!
//! ## Quick examples
//!
//! ```
//! use person_matcher::Normalizer;
//!
//! // Names: lowercase, drop diacritics, drop ASCII punctuation, collapse spaces.
//! assert_eq!(Normalizer::normalize_name("  O'Brien  "), "obrien");
//! assert_eq!(Normalizer::normalize_name("Siân"),         "sian");
//!
//! // Postcodes: strip whitespace, uppercase.
//! assert_eq!(Normalizer::normalize_postcode("cf10 1aa"), "CF101AA");
//!
//! // Phone numbers: keep digits, strip international and trunk prefixes.
//! assert_eq!(Normalizer::normalize_phone("+44 7700 900123"), "7700900123");
//! ```
//!
//! ## What this module deliberately does *not* do
//!
//! - It does not validate United Kingdom National Health Service Numbers —
//!   that is delegated to the
//!   `united-kingdom-national-health-service-number` crate at the call-site
//!   (see [`crate::matcher`]).
//! - It does not normalise email addresses or middle names (see spec
//!   tasks T-11 and OQ-1 respectively).
//! - It does not handle non-ASCII punctuation such as the curly apostrophe
//!   `’` (U+2019). Upstream code should convert those to ASCII first.
//!
//! ## International phone numbers
//!
//! Two phone normalisers are provided:
//!
//! - [`Normalizer::normalize_phone`] — UK-centric national-significant form,
//!   suitable for legacy or single-jurisdiction call-sites. Idempotent and
//!   infallible.
//! - [`Normalizer::normalize_phone_e164`] — international-aware E.164 form
//!   (`+CCNNNN…`) for jurisdictions in the supported country table. Returns
//!   `None` if the input cannot be confidently parsed.
//!
//! The matching engine tries E.164 first and falls back to the legacy form
//! when either input is unparseable, so existing single-country deployments
//! observe the same behaviour while multinational deployments gain
//! cross-country disambiguation (a French number and a UK number that share
//! the same trunk digits no longer collide).

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

/// Stateless namespace for text normalisation routines.
///
/// `Normalizer` is a unit type with no fields; every method is associated.
/// It is held as a struct rather than a free function module purely so the
/// public API has a single, discoverable entry point.
///
/// ```
/// use person_matcher::Normalizer;
///
/// let canonical = Normalizer::normalize_name("José-María");
/// assert_eq!(canonical, "josemaria");
/// ```
pub struct Normalizer;

impl Normalizer {
    /// Normalise a human name for comparison.
    ///
    /// Steps, in order:
    ///
    /// 1. Decompose to Unicode NFKD form (`é` → `e` + combining acute).
    /// 2. Drop combining marks (diacritics).
    /// 3. Drop ASCII punctuation (apostrophes, hyphens, full stops, …).
    /// 4. Lowercase.
    /// 5. Collapse consecutive whitespace to single ASCII spaces; trim ends.
    ///
    /// The result is suitable for direct equality comparison or for feeding
    /// into a string-similarity scorer.
    ///
    /// # Examples
    ///
    /// Whitespace is collapsed and trimmed:
    ///
    /// ```
    /// use person_matcher::Normalizer;
    /// assert_eq!(Normalizer::normalize_name("  John  Smith  "), "john smith");
    /// ```
    ///
    /// Apostrophes and hyphens are stripped:
    ///
    /// ```
    /// # use person_matcher::Normalizer;
    /// assert_eq!(Normalizer::normalize_name("O'Brien"),    "obrien");
    /// assert_eq!(Normalizer::normalize_name("MARY-JANE"),  "maryjane");
    /// ```
    ///
    /// Diacritics are removed:
    ///
    /// ```
    /// # use person_matcher::Normalizer;
    /// assert_eq!(Normalizer::normalize_name("José"),  "jose");
    /// assert_eq!(Normalizer::normalize_name("Siân"),  "sian");
    /// assert_eq!(Normalizer::normalize_name("Łukasz"), "łukasz");  // ł has no decomposition
    /// ```
    ///
    /// Empty and whitespace-only input round-trip cleanly:
    ///
    /// ```
    /// # use person_matcher::Normalizer;
    /// assert_eq!(Normalizer::normalize_name(""),       "");
    /// assert_eq!(Normalizer::normalize_name("    "),   "");
    /// ```
    ///
    /// The function is **idempotent**:
    ///
    /// ```
    /// # use person_matcher::Normalizer;
    /// let once = Normalizer::normalize_name("  José-María  ");
    /// let twice = Normalizer::normalize_name(&once);
    /// assert_eq!(once, twice);
    /// ```
    #[must_use]
    pub fn normalize_name(name: &str) -> String {
        // NFKD decomposes a precomposed glyph (e.g. `é`) into a base letter
        // plus a separate combining mark, so the diacritic can be dropped
        // independently of the letter it sits on.
        name.nfkd()
            // Drop the combining marks left behind by decomposition — this is
            // the diacritic-stripping step (`é` + acute → `e`).
            .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
            // Remove ASCII punctuation so `O'Brien`/`Mary-Jane` lose their
            // apostrophes/hyphens and compare equal to the bare letters.
            .filter(|c| !c.is_ascii_punctuation())
            .collect::<String>()
            // Case-fold after stripping so `MARY` and `mary` collapse together.
            .to_lowercase()
            // `split_whitespace` + `join(" ")` collapses any run of internal
            // whitespace to a single ASCII space and trims both ends.
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Normalise a postcode for comparison.
    ///
    /// Steps: drop all whitespace, then uppercase. No locale-specific
    /// validation — that is intentionally out of scope.
    ///
    /// # Examples
    ///
    /// UK postcodes with and without the conventional space are equivalent:
    ///
    /// ```
    /// use person_matcher::Normalizer;
    /// assert_eq!(Normalizer::normalize_postcode("CF10 1AA"),    "CF101AA");
    /// assert_eq!(Normalizer::normalize_postcode("cf101aa"),     "CF101AA");
    /// assert_eq!(Normalizer::normalize_postcode("  cf10 1aa "), "CF101AA");
    /// ```
    ///
    /// Empty input is preserved:
    ///
    /// ```
    /// # use person_matcher::Normalizer;
    /// assert_eq!(Normalizer::normalize_postcode(""), "");
    /// ```
    ///
    /// Idempotent:
    ///
    /// ```
    /// # use person_matcher::Normalizer;
    /// let once = Normalizer::normalize_postcode("sw1a 2aa");
    /// let twice = Normalizer::normalize_postcode(&once);
    /// assert_eq!(once, twice);
    /// ```
    #[must_use]
    pub fn normalize_postcode(postcode: &str) -> String {
        postcode
            .chars()
            // Drop every whitespace character so the conventional space in a
            // UK postcode (`CF10 1AA`) does not split otherwise-equal values.
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            // Postcodes are conventionally uppercase; fold so `cf101aa` matches.
            .to_uppercase()
    }

    /// Normalise a phone number for comparison.
    ///
    /// Steps:
    ///
    /// 1. Keep only ASCII digits (drop spaces, brackets, hyphens, `+`, …).
    /// 2. If the result starts with `0044`, drop those four characters.
    /// 3. Else, if the result starts with `44` and is at least 12 digits long,
    ///    drop the leading `44`.
    /// 4. Else, if the result starts with `0` and is longer than one digit,
    ///    drop the leading `0`.
    ///
    /// This canonicalises the common UK formats into a single subscriber
    /// number with no leading prefix. International numbers from other
    /// countries pass through unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// use person_matcher::Normalizer;
    ///
    /// // UK mobile, in three formats:
    /// assert_eq!(Normalizer::normalize_phone("07700 900123"),    "7700900123");
    /// assert_eq!(Normalizer::normalize_phone("+44 7700 900123"), "7700900123");
    /// assert_eq!(Normalizer::normalize_phone("0044 7700 900123"), "7700900123");
    ///
    /// // UK landline with brackets and spaces:
    /// assert_eq!(Normalizer::normalize_phone("(029) 2034 5678"), "2920345678");
    ///
    /// // Empty input is preserved (no digits to keep):
    /// assert_eq!(Normalizer::normalize_phone(""), "");
    /// ```
    ///
    /// Idempotent on canonical inputs:
    ///
    /// ```
    /// # use person_matcher::Normalizer;
    /// let once = Normalizer::normalize_phone("07700 900123");
    /// let twice = Normalizer::normalize_phone(&once);
    /// assert_eq!(once, twice);
    /// ```
    #[must_use]
    pub fn normalize_phone(phone: &str) -> String {
        // Strip every non-digit (spaces, brackets, hyphens, leading `+`, …)
        // so only the dialable digits remain.
        let digits: String = phone.chars().filter(char::is_ascii_digit).collect();

        // `0044` is the UK international access form (`00` access code + `44`
        // dial code); drop all four. The `len > 4` guard keeps a bare `0044`
        // from yielding an empty string.
        if digits.starts_with("0044") && digits.len() > 4 {
            return digits[4..].to_string();
        }

        // `44…` is the bare UK dial code. Require at least 12 digits (44 + a
        // plausible 10-digit national number) so we don't mistake a domestic
        // number that merely happens to start `44` for an international one.
        if digits.starts_with("44") && digits.len() >= 12 {
            return digits[2..].to_string();
        }

        // A single leading `0` is the UK national trunk prefix; drop it. The
        // `len > 1` guard preserves a lone `0` (see `normalize_phone_keeps_lone_zero`).
        if digits.starts_with('0') && digits.len() > 1 {
            return digits[1..].to_string();
        }

        // No recognised prefix: foreign / already-bare numbers pass through.
        digits
    }

    /// Normalise a phone number to its E.164-style canonical form.
    ///
    /// E.164 is the ITU-T standard for international telephone numbers and
    /// has the shape `+CCNNN…`, where `CC` is the country dialling code
    /// (1–3 digits) and the remainder is the national-significant number
    /// (NSN) with no trunk prefix.
    ///
    /// The function accepts a wide range of textual layouts:
    ///
    /// - `+CC…` (explicit international, the canonical input form).
    /// - `00CC…` (international access code, common across Europe).
    /// - `0…` (national format, trunk-prefix) — interpreted relative to
    ///   `default_country` when the country uses a national trunk `0`.
    /// - `NSN…` (bare national-significant number) — interpreted relative
    ///   to `default_country`.
    ///
    /// Returns `Some(canonical)` if the input parses against a country in
    /// the supported table; otherwise `None`. The supported countries are
    /// the five jurisdictions for which the crate exposes a national
    /// healthcare identifier (United Kingdom, France, Spain, Ireland, and
    /// — sharing the GB dial code — UK Northern Ireland), plus the most
    /// common person-mobility partners (US, CA, DE, IT, NL, BE, PT, CH,
    /// AT, SE, NO, DK, FI, PL, AU, NZ, JP, CN, IN, BR, MX, ZA). `default_country` is the
    /// **ISO 3166-1 alpha-2 code** (e.g. `"GB"`, `"FR"`, `"US"`) of the
    /// jurisdiction whose national format applies when the input lacks an
    /// explicit international marker. Pass `None` to refuse to assume a
    /// default — only explicit `+CC` / `00CC` inputs will parse.
    ///
    /// The function is **deterministic** and **idempotent**: feeding a
    /// canonical `+CCNNN…` string back in returns the same string.
    ///
    /// # Examples
    ///
    /// UK mobile, three textual layouts, all canonicalise to the same E.164 form:
    ///
    /// ```
    /// use person_matcher::Normalizer;
    /// assert_eq!(
    ///     Normalizer::normalize_phone_e164("+44 7700 900123", Some("GB")),
    ///     Some("+447700900123".to_string()),
    /// );
    /// assert_eq!(
    ///     Normalizer::normalize_phone_e164("0044 7700 900123", Some("GB")),
    ///     Some("+447700900123".to_string()),
    /// );
    /// assert_eq!(
    ///     Normalizer::normalize_phone_e164("07700 900123", Some("GB")),
    ///     Some("+447700900123".to_string()),
    /// );
    /// ```
    ///
    /// French national format vs international form:
    ///
    /// ```
    /// # use person_matcher::Normalizer;
    /// assert_eq!(
    ///     Normalizer::normalize_phone_e164("01 23 45 67 89", Some("FR")),
    ///     Some("+33123456789".to_string()),
    /// );
    /// assert_eq!(
    ///     Normalizer::normalize_phone_e164("+33 1 23 45 67 89", Some("GB")),
    ///     Some("+33123456789".to_string()),
    /// );
    /// ```
    ///
    /// North American (NANP) numbers have no trunk prefix:
    ///
    /// ```
    /// # use person_matcher::Normalizer;
    /// assert_eq!(
    ///     Normalizer::normalize_phone_e164("(415) 555-1234", Some("US")),
    ///     Some("+14155551234".to_string()),
    /// );
    /// assert_eq!(
    ///     Normalizer::normalize_phone_e164("+1 415 555 1234", None),
    ///     Some("+14155551234".to_string()),
    /// );
    /// ```
    ///
    /// Unparseable or ambiguous inputs return `None`:
    ///
    /// ```
    /// # use person_matcher::Normalizer;
    /// // No default country and no international marker: ambiguous.
    /// assert_eq!(Normalizer::normalize_phone_e164("07700 900123", None), None);
    /// // Unknown dial code.
    /// assert_eq!(Normalizer::normalize_phone_e164("+999 1234567", None), None);
    /// // Empty input.
    /// assert_eq!(Normalizer::normalize_phone_e164("", Some("GB")), None);
    /// ```
    ///
    /// Idempotent on canonical inputs:
    ///
    /// ```
    /// # use person_matcher::Normalizer;
    /// let once = Normalizer::normalize_phone_e164("+44 7700 900123", Some("GB")).unwrap();
    /// let twice = Normalizer::normalize_phone_e164(&once, Some("GB")).unwrap();
    /// assert_eq!(once, twice);
    /// ```
    #[must_use]
    pub fn normalize_phone_e164(phone: &str, default_country: Option<&str>) -> Option<String> {
        // A literal `+` anywhere is the unambiguous international marker; we
        // detect it before discarding non-digits below.
        let has_plus = phone.chars().any(|c| c == '+');
        // Reduce to bare digits; the textual layout (spaces, brackets) is noise.
        let digits: String = phone.chars().filter(char::is_ascii_digit).collect();
        if digits.is_empty() {
            return None;
        }

        // Dispatch on the three input shapes, each yielding the matched country
        // plus the national-significant number (NSN) with any trunk prefix gone.
        let (info, nsn): (&CountryPhoneInfo, String) = if has_plus {
            // Explicit `+CC…`: the country is encoded in the leading dial code.
            let info = lookup_by_dial_code_prefix(&digits)?;
            // Everything after the dial code is the national portion …
            let rest = &digits[info.dial_code.len()..];
            // … minus the national trunk prefix if the entry system left it in
            // (e.g. the stray `0` in `+44 0 7700 900123`).
            let rest = strip_trunk_prefix(info, rest);
            (info, rest.to_string())
        } else if let Some(stripped) = digits.strip_prefix("00") {
            // `00CC…`: the `00` is the international access code (common in
            // Europe); after stripping it the tail behaves like the `+` case.
            let info = lookup_by_dial_code_prefix(stripped)?;
            let rest = &stripped[info.dial_code.len()..];
            let rest = strip_trunk_prefix(info, rest);
            (info, rest.to_string())
        } else {
            // Bare national format: with no marker we must be told which
            // country to assume; refuse (`None`) if the caller passed none.
            let iso = default_country?;
            let info = lookup_by_iso(iso)?;
            // National input carries the trunk prefix; strip it to reach the NSN.
            let nsn = strip_trunk_prefix(info, &digits);
            (info, nsn.to_string())
        };

        // Length-bound the NSN against the country's plan: too short / too long
        // means the parse was not confident, so reject rather than emit junk.
        if nsn.len() < info.min_nsn || nsn.len() > info.max_nsn {
            return None;
        }

        // Reassemble canonical E.164: `+` then dial code then trunk-free NSN.
        Some(format!("+{}{}", info.dial_code, nsn))
    }

    /// Expand common postal address abbreviations as whole tokens.
    ///
    /// The input is tokenised on whitespace and each token is matched
    /// case-insensitively (after stripping a single trailing `.` or `,`)
    /// against a fixed table of street-type and directional abbreviations.
    /// Recognised tokens are replaced with their long form, lowercased;
    /// unrecognised tokens are passed through verbatim. Tokens are then
    /// re-joined by single spaces.
    ///
    /// This function is intentionally simple: it does **not** apply any
    /// position-aware heuristics. The well-known ambiguous case `"St"` —
    /// which can mean *Street* or *Saint* — is always expanded to
    /// *Street*. In practice this remains useful for fuzzy matching
    /// because the canonical form is consistent on both sides of a
    /// comparison; pre-process upstream if you need finer disambiguation.
    ///
    /// # Examples
    ///
    /// ```
    /// use person_matcher::Normalizer;
    /// assert_eq!(
    ///     Normalizer::expand_street_abbreviations("123 High St"),
    ///     "123 High street",
    /// );
    /// assert_eq!(
    ///     Normalizer::expand_street_abbreviations("45 N. Park Ave."),
    ///     "45 north Park avenue",
    /// );
    /// assert_eq!(
    ///     Normalizer::expand_street_abbreviations("12 Sunset Blvd"),
    ///     "12 Sunset boulevard",
    /// );
    /// ```
    ///
    /// Idempotent on already-expanded inputs (long forms are not
    /// re-expanded):
    ///
    /// ```
    /// # use person_matcher::Normalizer;
    /// let once = Normalizer::expand_street_abbreviations("10 Downing St");
    /// let twice = Normalizer::expand_street_abbreviations(&once);
    /// assert_eq!(once, twice);
    /// ```
    pub fn expand_street_abbreviations(line: &str) -> String {
        line.split_whitespace()
            .map(expand_one_token)
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Normalise an address line for comparison.
    ///
    /// Pipeline:
    ///
    /// 1. Expand street-type and directional abbreviations via
    ///    [`Normalizer::expand_street_abbreviations`] (so `"St" → "street"`,
    ///    `"Rd" → "road"`, `"N" → "north"`).
    /// 2. Apply the name-normalisation pipeline
    ///    ([`Normalizer::normalize_name`]): NFKD-decompose, drop combining
    ///    marks, drop ASCII punctuation, lowercase, collapse whitespace.
    ///
    /// The result is idempotent and suitable for direct equality or
    /// similarity comparison.
    ///
    /// # Examples
    ///
    /// Abbreviated and full forms canonicalise identically:
    ///
    /// ```
    /// use person_matcher::Normalizer;
    /// assert_eq!(
    ///     Normalizer::normalize_address_line("123 High St"),
    ///     Normalizer::normalize_address_line("123 High Street"),
    /// );
    /// assert_eq!(
    ///     Normalizer::normalize_address_line("45 N Park Ave"),
    ///     Normalizer::normalize_address_line("45 North Park Avenue"),
    /// );
    /// ```
    ///
    /// Punctuation and case are normalised:
    ///
    /// ```
    /// # use person_matcher::Normalizer;
    /// assert_eq!(
    ///     Normalizer::normalize_address_line("10, DOWNING Street."),
    ///     "10 downing street",
    /// );
    /// ```
    #[must_use]
    pub fn normalize_address_line(line: &str) -> String {
        Self::normalize_name(&Self::expand_street_abbreviations(line))
    }

    /// Parse an address line into its structured components.
    ///
    /// The function performs a best-effort structural decomposition of a
    /// single-line postal address into:
    ///
    /// - `house_number` — the leading run of digits (with an optional
    ///   single alphabetic suffix, e.g. `"10A"`), uppercased. `None` if
    ///   no leading number is present.
    /// - `unit` — a recognised sub-unit prefix (`Flat`, `Apt`,
    ///   `Apartment`, `Unit`, `Suite`, `Ste`) and its identifier,
    ///   lowercased and space-joined (e.g. `"flat 2a"`). `None` if no
    ///   recognised prefix is present.
    /// - `street` — the remaining text after `unit` and `house_number`
    ///   are removed, run through [`Normalizer::normalize_address_line`].
    ///
    /// Parsing is **deterministic** and **format-only** — no postal
    /// reference is consulted. Inputs that do not match the simple
    /// regular structure (e.g. a postcode-only string, a city name)
    /// degrade gracefully: `house_number` and `unit` are `None`, and
    /// `street` carries the normalised input.
    ///
    /// # Examples
    ///
    /// Typical UK / US single-line addresses:
    ///
    /// ```
    /// use person_matcher::Normalizer;
    ///
    /// let p = Normalizer::parse_address_line("123 High Street");
    /// assert_eq!(p.house_number.as_deref(), Some("123"));
    /// assert_eq!(p.unit, None);
    /// assert_eq!(p.street, "high street");
    ///
    /// let p = Normalizer::parse_address_line("10A Downing St");
    /// assert_eq!(p.house_number.as_deref(), Some("10A"));
    /// assert_eq!(p.street, "downing street");
    ///
    /// let p = Normalizer::parse_address_line("Flat 2A, 10 Downing Street");
    /// assert_eq!(p.unit.as_deref(), Some("flat 2a"));
    /// assert_eq!(p.house_number.as_deref(), Some("10"));
    /// assert_eq!(p.street, "downing street");
    ///
    /// let p = Normalizer::parse_address_line("Apt 5, 1600 Pennsylvania Ave");
    /// assert_eq!(p.unit.as_deref(), Some("apt 5"));
    /// assert_eq!(p.house_number.as_deref(), Some("1600"));
    /// assert_eq!(p.street, "pennsylvania avenue");
    /// ```
    ///
    /// Inputs without a leading number still parse:
    ///
    /// ```
    /// # use person_matcher::Normalizer;
    /// let p = Normalizer::parse_address_line("Buckingham Palace");
    /// assert_eq!(p.house_number, None);
    /// assert_eq!(p.unit, None);
    /// assert_eq!(p.street, "buckingham palace");
    /// ```
    #[must_use]
    pub fn parse_address_line(line: &str) -> ParsedAddressLine {
        let trimmed = line.trim();
        let (unit, after_unit) = extract_unit_prefix(trimmed);
        let after_unit = after_unit.trim_start_matches([',', ' ', '\t']).trim();
        let (house_number, after_number) = extract_house_number(after_unit);
        let after_number = after_number.trim_start_matches([',', ' ', '\t']).trim();
        ParsedAddressLine {
            house_number,
            unit,
            street: Self::normalize_address_line(after_number),
        }
    }

    /// Compute a phonetic (Soundex) code for a name.
    ///
    /// Internally, the input is first normalised via
    /// [`Normalizer::normalize_name`] and then encoded with the American
    /// Soundex algorithm. Names that sound alike map to the same code, which
    /// lets the matcher catch spelling variants such as "Smith" / "Smyth" or
    /// "Stephen" / "Steven".
    ///
    /// The implementation is suitable for English-language names. Non-English
    /// phonemes may be lost. T-9 (spec §21.4) decided to keep Soundex as the
    /// default and expose an opt-in `MatchConfig::phonetic_encoder` enum
    /// (Double Metaphone, Daitch-Mokotoff) gated behind a Cargo feature flag
    /// once an empirical multinational person corpus is available;
    /// implementation is tracked as T-9.1.
    ///
    /// # Examples
    ///
    /// Similar-sounding spellings share a code:
    ///
    /// ```
    /// use person_matcher::Normalizer;
    /// assert_eq!(Normalizer::phonetic_code("Smith"), Normalizer::phonetic_code("Smyth"));
    /// assert_eq!(Normalizer::phonetic_code("Stephen"), Normalizer::phonetic_code("Steven"));
    /// ```
    ///
    /// Different families produce different codes:
    ///
    /// ```
    /// # use person_matcher::Normalizer;
    /// assert_ne!(Normalizer::phonetic_code("Jones"), Normalizer::phonetic_code("Smith"));
    /// ```
    ///
    /// Empty input returns an empty string, not a default Soundex value:
    ///
    /// ```
    /// # use person_matcher::Normalizer;
    /// assert_eq!(Normalizer::phonetic_code(""),       "");
    /// assert_eq!(Normalizer::phonetic_code("   "),    "");
    /// ```
    #[must_use]
    pub fn phonetic_code(name: &str) -> String {
        let normalized = Self::normalize_name(name);
        if normalized.is_empty() {
            return String::new();
        }
        soundex::american_soundex(&normalized)
    }

    /// Normalise an email address for comparison.
    ///
    /// Steps:
    ///
    /// 1. Trim surrounding whitespace.
    /// 2. Lowercase the entire address (RFC 5321 makes the domain
    ///    case-insensitive and most real-world deployments treat the
    ///    localpart case-insensitively too; case-sensitive localparts
    ///    are technically legal but vanishingly rare in healthcare data).
    /// 3. Reject inputs that lack exactly one `@` or that have an empty
    ///    localpart or domain by returning `None`.
    /// 4. If `gmail_dot_folding` is `true` and the domain is `gmail.com`
    ///    or `googlemail.com`, strip every `.` from the localpart and
    ///    drop any `+tag` suffix. Both transformations are reversible
    ///    for Gmail addresses by Google's documented routing rules:
    ///    `j.smith@gmail.com`, `js.mith@gmail.com`, and
    ///    `jsmith+work@gmail.com` all deliver to the same mailbox as
    ///    `jsmith@gmail.com`.
    ///
    /// The function is **deterministic** and **idempotent** on
    /// successful outputs.
    ///
    /// # Examples
    ///
    /// Common case-and-whitespace normalisation:
    ///
    /// ```
    /// use person_matcher::Normalizer;
    /// assert_eq!(
    ///     Normalizer::normalize_email("  Alice@Example.ORG  ", false),
    ///     Some("alice@example.org".to_string()),
    /// );
    /// ```
    ///
    /// Malformed inputs return `None`:
    ///
    /// ```
    /// # use person_matcher::Normalizer;
    /// assert_eq!(Normalizer::normalize_email("no-at-sign", false), None);
    /// assert_eq!(Normalizer::normalize_email("@example.org", false), None);
    /// assert_eq!(Normalizer::normalize_email("alice@", false), None);
    /// assert_eq!(Normalizer::normalize_email("a@b@c", false), None);
    /// assert_eq!(Normalizer::normalize_email("", false), None);
    /// ```
    ///
    /// Optional Gmail dot-folding:
    ///
    /// ```
    /// # use person_matcher::Normalizer;
    /// assert_eq!(
    ///     Normalizer::normalize_email("j.smith@gmail.com", true),
    ///     Some("jsmith@gmail.com".to_string()),
    /// );
    /// assert_eq!(
    ///     Normalizer::normalize_email("jsmith+work@googlemail.com", true),
    ///     Some("jsmith@googlemail.com".to_string()),
    /// );
    /// // Dot-folding does not touch non-Gmail addresses.
    /// assert_eq!(
    ///     Normalizer::normalize_email("j.smith@example.org", true),
    ///     Some("j.smith@example.org".to_string()),
    /// );
    /// ```
    ///
    /// Idempotent on canonical inputs:
    ///
    /// ```
    /// # use person_matcher::Normalizer;
    /// let once = Normalizer::normalize_email("Alice@Example.ORG", false).unwrap();
    /// let twice = Normalizer::normalize_email(&once, false).unwrap();
    /// assert_eq!(once, twice);
    /// ```
    #[must_use]
    pub fn normalize_email(email: &str, gmail_dot_folding: bool) -> Option<String> {
        let trimmed = email.trim().to_lowercase();
        if trimmed.is_empty() {
            return None;
        }
        // Require exactly one '@'.
        let (local, domain) = trimmed.split_once('@')?;
        if local.is_empty() || domain.is_empty() {
            return None;
        }
        // Reject any further '@' in the domain side.
        if domain.contains('@') {
            return None;
        }
        if gmail_dot_folding && (domain == "gmail.com" || domain == "googlemail.com") {
            let local_no_plus = match local.find('+') {
                Some(i) => &local[..i],
                None => local,
            };
            let local_folded: String = local_no_plus.chars().filter(|c| *c != '.').collect();
            if local_folded.is_empty() {
                return None;
            }
            return Some(format!("{local_folded}@{domain}"));
        }
        Some(format!("{local}@{domain}"))
    }
}

/// Structured decomposition of a postal-address line.
///
/// Produced by [`Normalizer::parse_address_line`]. The struct is
/// `Serialize + Deserialize` so it round-trips through JSON and can be
/// embedded in downstream data models.
///
/// All three fields are best-effort: parsing is format-only and consults
/// no postal reference. Inputs that don't follow the
/// `(unit, house_number, street)` shape degrade gracefully, with the
/// missing pieces returned as `None`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedAddressLine {
    /// Leading house / building number, including an optional single
    /// alphabetic suffix (`"10A"`), uppercased. `None` when no leading
    /// digit is present.
    pub house_number: Option<String>,
    /// Sub-unit prefix and identifier, lowercased and space-joined
    /// (e.g. `"flat 2a"`, `"apt 5"`, `"suite 12"`). `None` when no
    /// recognised prefix is present.
    pub unit: Option<String>,
    /// Remaining street portion, normalised via
    /// [`Normalizer::normalize_address_line`].
    pub street: String,
}

/// Token-level expansion table used by [`Normalizer::expand_street_abbreviations`].
///
/// Entries are matched case-insensitively against a token with at most one
/// trailing `.` or `,` stripped. The replacement is always lowercase so the
/// downstream name-normalisation pipeline is a no-op for these tokens.
const STREET_ABBREVIATIONS: &[(&str, &str)] = &[
    ("st", "street"),
    ("str", "street"),
    ("rd", "road"),
    ("ave", "avenue"),
    ("av", "avenue"),
    ("blvd", "boulevard"),
    ("bvd", "boulevard"),
    ("ln", "lane"),
    ("dr", "drive"),
    ("ct", "court"),
    ("pl", "place"),
    ("sq", "square"),
    ("ter", "terrace"),
    ("terr", "terrace"),
    ("hwy", "highway"),
    ("pkwy", "parkway"),
    ("mt", "mount"),
    ("mtn", "mountain"),
    ("cres", "crescent"),
    ("gdns", "gardens"),
    ("gdn", "garden"),
    ("gr", "grove"),
    ("cl", "close"),
    ("pk", "park"),
    ("plz", "plaza"),
    ("expy", "expressway"),
    ("trl", "trail"),
    ("n", "north"),
    ("s", "south"),
    ("e", "east"),
    ("w", "west"),
    ("ne", "northeast"),
    ("nw", "northwest"),
    ("se", "southeast"),
    ("sw", "southwest"),
];

/// Recognised sub-unit prefix keywords for [`Normalizer::parse_address_line`].
const UNIT_PREFIXES: &[&str] = &[
    "flat",
    "apartment",
    "apt",
    "unit",
    "suite",
    "ste",
    "room",
    "rm",
];

/// Expand a single whitespace-separated token if it appears in
/// [`STREET_ABBREVIATIONS`].
///
/// The token is matched after stripping at most one trailing `.` or `,`;
/// the comparison is ASCII case-insensitive. Tokens that contain non-ASCII
/// characters short-circuit to the original input unchanged.
fn expand_one_token(tok: &str) -> String {
    // Drop a trailing `.`/`,` so `St.` and `St` both match the table key `st`.
    let stripped = tok.trim_end_matches(['.', ',']);
    // The abbreviation table is ASCII-only; a non-ASCII token cannot be a key,
    // so return it verbatim (and avoid touching multi-byte characters).
    if !stripped.is_ascii() {
        return tok.to_string();
    }
    let lower = stripped.to_ascii_lowercase();
    // Linear scan: the table is short and constant, so this is fine.
    for (abbrev, full) in STREET_ABBREVIATIONS {
        if lower == *abbrev {
            return (*full).to_string();
        }
    }
    // Not an abbreviation: preserve the original token, including its casing
    // and any trailing punctuation we only stripped for the comparison.
    tok.to_string()
}

/// Extract a recognised unit prefix and its identifier from the start of `s`.
///
/// Returns `(Some("flat 2a"), rest)` when the input begins with a
/// recognised keyword followed by an alphanumeric identifier; otherwise
/// `(None, s)` unchanged.
fn extract_unit_prefix(s: &str) -> (Option<String>, &str) {
    let trimmed = s.trim_start();
    // Find the first whitespace; everything before is the candidate keyword.
    let kw_end = trimmed
        .find(|c: char| c.is_whitespace())
        .unwrap_or(trimmed.len());
    // No leading token (string was empty/all-whitespace): nothing to match.
    if kw_end == 0 {
        return (None, s);
    }
    let kw_raw = &trimmed[..kw_end];
    // Tolerate a trailing `.`/`,` on the keyword (`Apt.`, `Flat,`).
    let kw_stripped = kw_raw.trim_end_matches(['.', ',']);
    // The keyword table is ASCII; bail on non-ASCII candidates.
    if !kw_stripped.is_ascii() {
        return (None, s);
    }
    let kw_lower = kw_stripped.to_ascii_lowercase();
    // Only proceed if the first token is a recognised unit keyword; otherwise
    // leave the input untouched so the house-number stage can run on it.
    if !UNIT_PREFIXES.iter().any(|p| *p == kw_lower) {
        return (None, s);
    }
    // Skip whitespace and `#` after the keyword (`Apt #5` → identifier `5`).
    let after_kw = trimmed[kw_end..].trim_start_matches([' ', '\t', '#']);
    // The identifier is the leading alphanumeric run (`2A`, `5`, `12`).
    let id_end = after_kw
        .find(|c: char| !c.is_ascii_alphanumeric())
        .unwrap_or(after_kw.len());
    // A keyword with no following identifier is not a real unit prefix.
    if id_end == 0 {
        return (None, s);
    }
    let id = &after_kw[..id_end];
    // The remainder (after the identifier) is handed back for further parsing.
    let rest = &after_kw[id_end..];
    // Canonical unit string: lowercase keyword + space + lowercase identifier.
    let unit = format!("{} {}", kw_lower, id.to_ascii_lowercase());
    (Some(unit), rest)
}

/// Extract a leading house number (digits + optional single alphabetic
/// suffix) from the start of `s`.
///
/// `"10 Downing Street"` → `(Some("10"), " Downing Street")`.
/// `"10A High St"` → `(Some("10A"), " High St")`.
/// `"Buckingham Palace"` → `(None, "Buckingham Palace")`.
fn extract_house_number(s: &str) -> (Option<String>, &str) {
    let trimmed = s.trim_start();
    // Walk the leading digit run; `digits_end` is the byte index just past it.
    let mut digits_end = 0;
    for (i, c) in trimmed.char_indices() {
        if c.is_ascii_digit() {
            digits_end = i + c.len_utf8();
        } else {
            // First non-digit ends the number.
            break;
        }
    }
    // No leading digits means there is no house number (e.g. "Buckingham Palace").
    if digits_end == 0 {
        return (None, s);
    }
    let mut end = digits_end;
    // Allow a single alphabetic suffix (e.g. "10A"), but only when not
    // followed by another alphabetic — otherwise we'd swallow the start
    // of a street name like "10 Apple Tree Lane".
    let after_digits = &trimmed[digits_end..];
    let mut chars = after_digits.chars();
    if let Some(c1) = chars.next()
        && c1.is_ascii_alphabetic()
    {
        let next = chars.next();
        if next.is_none_or(|c2| !c2.is_ascii_alphanumeric()) {
            end += c1.len_utf8();
        }
    }
    let number = trimmed[..end].to_ascii_uppercase();
    (Some(number), &trimmed[end..])
}

/// Per-country phone metadata for [`Normalizer::normalize_phone_e164`].
///
/// `min_nsn` / `max_nsn` bound the **national-significant number** length —
/// the digits after the dial code, with the national trunk prefix removed.
/// `trunk_prefix` is the digit string used for national dialling (`"0"` for
/// most of Europe and Asia, `"8"` for Lithuania, `None` for NANP / Spain /
/// Portugal and several others). When set, a single occurrence of the
/// string at the start of the national number is stripped before
/// canonicalisation.
struct CountryPhoneInfo {
    /// ISO 3166-1 alpha-2 country code, uppercase.
    iso_alpha2: &'static str,
    /// International dialling code, no leading `+`.
    dial_code: &'static str,
    /// National trunk prefix digit(s), if any.
    trunk_prefix: Option<&'static str>,
    /// Minimum national-significant-number length.
    min_nsn: usize,
    /// Maximum national-significant-number length.
    max_nsn: usize,
}

/// Phone-numbering metadata for countries supported by
/// [`Normalizer::normalize_phone_e164`].
///
/// Coverage: all five jurisdictions for which the crate exposes a national
/// healthcare identifier (GB England/Wales/IoM, FR, ES, IE, plus UK NI via
/// the GB dial code), plus the most common person-mobility partners. New
/// entries SHOULD follow the ISO 3166-1 alpha-2 convention and document the
/// trunk-prefix rule explicitly.
const COUNTRY_PHONE_TABLE: &[CountryPhoneInfo] = &[
    CountryPhoneInfo {
        iso_alpha2: "GB",
        dial_code: "44",
        trunk_prefix: Some("0"),
        min_nsn: 7,
        max_nsn: 11,
    },
    CountryPhoneInfo {
        iso_alpha2: "FR",
        dial_code: "33",
        trunk_prefix: Some("0"),
        min_nsn: 9,
        max_nsn: 9,
    },
    CountryPhoneInfo {
        iso_alpha2: "DE",
        dial_code: "49",
        trunk_prefix: Some("0"),
        min_nsn: 7,
        max_nsn: 13,
    },
    CountryPhoneInfo {
        iso_alpha2: "ES",
        dial_code: "34",
        trunk_prefix: None,
        min_nsn: 9,
        max_nsn: 9,
    },
    CountryPhoneInfo {
        iso_alpha2: "IE",
        dial_code: "353",
        trunk_prefix: Some("0"),
        min_nsn: 7,
        max_nsn: 11,
    },
    CountryPhoneInfo {
        iso_alpha2: "IT",
        dial_code: "39",
        trunk_prefix: None,
        min_nsn: 6,
        max_nsn: 12,
    },
    CountryPhoneInfo {
        iso_alpha2: "NL",
        dial_code: "31",
        trunk_prefix: Some("0"),
        min_nsn: 9,
        max_nsn: 9,
    },
    CountryPhoneInfo {
        iso_alpha2: "BE",
        dial_code: "32",
        trunk_prefix: Some("0"),
        min_nsn: 8,
        max_nsn: 9,
    },
    CountryPhoneInfo {
        iso_alpha2: "PT",
        dial_code: "351",
        trunk_prefix: None,
        min_nsn: 9,
        max_nsn: 9,
    },
    CountryPhoneInfo {
        iso_alpha2: "CH",
        dial_code: "41",
        trunk_prefix: Some("0"),
        min_nsn: 9,
        max_nsn: 9,
    },
    CountryPhoneInfo {
        iso_alpha2: "AT",
        dial_code: "43",
        trunk_prefix: Some("0"),
        min_nsn: 4,
        max_nsn: 13,
    },
    CountryPhoneInfo {
        iso_alpha2: "SE",
        dial_code: "46",
        trunk_prefix: Some("0"),
        min_nsn: 7,
        max_nsn: 13,
    },
    CountryPhoneInfo {
        iso_alpha2: "NO",
        dial_code: "47",
        trunk_prefix: None,
        min_nsn: 8,
        max_nsn: 8,
    },
    CountryPhoneInfo {
        iso_alpha2: "DK",
        dial_code: "45",
        trunk_prefix: None,
        min_nsn: 8,
        max_nsn: 8,
    },
    CountryPhoneInfo {
        iso_alpha2: "FI",
        dial_code: "358",
        trunk_prefix: Some("0"),
        min_nsn: 5,
        max_nsn: 12,
    },
    CountryPhoneInfo {
        iso_alpha2: "PL",
        dial_code: "48",
        trunk_prefix: None,
        min_nsn: 9,
        max_nsn: 9,
    },
    CountryPhoneInfo {
        iso_alpha2: "AU",
        dial_code: "61",
        trunk_prefix: Some("0"),
        min_nsn: 9,
        max_nsn: 9,
    },
    CountryPhoneInfo {
        iso_alpha2: "NZ",
        dial_code: "64",
        trunk_prefix: Some("0"),
        min_nsn: 8,
        max_nsn: 10,
    },
    CountryPhoneInfo {
        iso_alpha2: "US",
        dial_code: "1",
        trunk_prefix: None,
        min_nsn: 10,
        max_nsn: 10,
    },
    CountryPhoneInfo {
        iso_alpha2: "CA",
        dial_code: "1",
        trunk_prefix: None,
        min_nsn: 10,
        max_nsn: 10,
    },
    CountryPhoneInfo {
        iso_alpha2: "JP",
        dial_code: "81",
        trunk_prefix: Some("0"),
        min_nsn: 9,
        max_nsn: 10,
    },
    CountryPhoneInfo {
        iso_alpha2: "CN",
        dial_code: "86",
        trunk_prefix: Some("0"),
        min_nsn: 5,
        max_nsn: 12,
    },
    CountryPhoneInfo {
        iso_alpha2: "IN",
        dial_code: "91",
        trunk_prefix: Some("0"),
        min_nsn: 10,
        max_nsn: 10,
    },
    CountryPhoneInfo {
        iso_alpha2: "BR",
        dial_code: "55",
        trunk_prefix: Some("0"),
        min_nsn: 10,
        max_nsn: 11,
    },
    CountryPhoneInfo {
        iso_alpha2: "MX",
        dial_code: "52",
        trunk_prefix: None,
        min_nsn: 10,
        max_nsn: 10,
    },
    CountryPhoneInfo {
        iso_alpha2: "ZA",
        dial_code: "27",
        trunk_prefix: Some("0"),
        min_nsn: 9,
        max_nsn: 9,
    },
    // ---- T-19: coverage of remaining 35-scheme identifier jurisdictions ----
    CountryPhoneInfo {
        iso_alpha2: "BG",
        dial_code: "359",
        trunk_prefix: Some("0"),
        min_nsn: 8,
        max_nsn: 9,
    },
    CountryPhoneInfo {
        iso_alpha2: "CZ",
        dial_code: "420",
        trunk_prefix: None,
        min_nsn: 9,
        max_nsn: 9,
    },
    CountryPhoneInfo {
        iso_alpha2: "EE",
        dial_code: "372",
        trunk_prefix: None,
        min_nsn: 7,
        max_nsn: 8,
    },
    CountryPhoneInfo {
        iso_alpha2: "GR",
        dial_code: "30",
        trunk_prefix: None,
        min_nsn: 10,
        max_nsn: 10,
    },
    CountryPhoneInfo {
        iso_alpha2: "HR",
        dial_code: "385",
        trunk_prefix: Some("0"),
        min_nsn: 8,
        max_nsn: 9,
    },
    CountryPhoneInfo {
        iso_alpha2: "IS",
        dial_code: "354",
        trunk_prefix: None,
        min_nsn: 7,
        max_nsn: 9,
    },
    CountryPhoneInfo {
        iso_alpha2: "LI",
        dial_code: "423",
        trunk_prefix: None,
        min_nsn: 7,
        max_nsn: 9,
    },
    // Lithuania uses `8` (not `0`) as the national trunk prefix.
    CountryPhoneInfo {
        iso_alpha2: "LT",
        dial_code: "370",
        trunk_prefix: Some("8"),
        min_nsn: 8,
        max_nsn: 8,
    },
    CountryPhoneInfo {
        iso_alpha2: "LV",
        dial_code: "371",
        trunk_prefix: None,
        min_nsn: 8,
        max_nsn: 8,
    },
    CountryPhoneInfo {
        iso_alpha2: "MT",
        dial_code: "356",
        trunk_prefix: None,
        min_nsn: 8,
        max_nsn: 8,
    },
    CountryPhoneInfo {
        iso_alpha2: "RO",
        dial_code: "40",
        trunk_prefix: Some("0"),
        min_nsn: 9,
        max_nsn: 9,
    },
    CountryPhoneInfo {
        iso_alpha2: "SI",
        dial_code: "386",
        trunk_prefix: Some("0"),
        min_nsn: 8,
        max_nsn: 8,
    },
    CountryPhoneInfo {
        iso_alpha2: "SK",
        dial_code: "421",
        trunk_prefix: Some("0"),
        min_nsn: 9,
        max_nsn: 9,
    },
];

/// Look up a country by ISO 3166-1 alpha-2 code (case-insensitive).
///
/// Returns the first match in [`COUNTRY_PHONE_TABLE`]. For NANP countries
/// (US/CA) which share dial code `1`, this disambiguates by the caller's
/// chosen default; the canonical E.164 output is identical for both.
fn lookup_by_iso(iso: &str) -> Option<&'static CountryPhoneInfo> {
    // ISO 3166-1 alpha-2 codes are ASCII letters; anything else cannot match.
    if !iso.is_ascii() {
        return None;
    }
    // Fold to uppercase so callers may pass `"gb"` or `"GB"` interchangeably.
    let upper = iso.to_ascii_uppercase();
    COUNTRY_PHONE_TABLE.iter().find(|c| c.iso_alpha2 == upper)
}

/// Match the longest known dial-code prefix at the start of `digits`.
///
/// Tries 3-, 2-, then 1-digit prefixes to honour the country table's most
/// specific entry. For NANP (dial code `1`) the first matching entry — US —
/// is returned; the canonical E.164 form is the same whether the caller
/// later interprets the country as US or CA.
fn lookup_by_dial_code_prefix(digits: &str) -> Option<&'static CountryPhoneInfo> {
    // Try longest prefix first (3 → 2 → 1) so a 3-digit code like `353` (IE)
    // is not shadowed by a shorter code that happens to be a prefix of it.
    for len in [3usize, 2, 1] {
        if digits.len() >= len {
            let prefix = &digits[..len];
            if let Some(info) = COUNTRY_PHONE_TABLE.iter().find(|c| c.dial_code == prefix) {
                return Some(info);
            }
        }
    }
    // No table entry matched any prefix length: unknown dial code.
    None
}

/// Strip a single occurrence of the country's national trunk prefix
/// from `nsn` if one is configured and present.
fn strip_trunk_prefix<'a>(info: &CountryPhoneInfo, nsn: &'a str) -> &'a str {
    // Only strip when the country has a trunk prefix, the NSN actually starts
    // with it, and stripping leaves a non-empty remainder (don't reduce a bare
    // trunk digit to the empty string).
    if let Some(tp) = info.trunk_prefix
        && let Some(rest) = nsn.strip_prefix(tp)
        && !rest.is_empty()
    {
        rest
    } else {
        // No trunk prefix configured, or it isn't present: leave the NSN as-is
        // (NANP/Spain/Portugal have no trunk `0`, so nothing to remove).
        nsn
    }
}

#[cfg(test)]
mod tests {
    //! Unit coverage for the text normalisers in this module.
    //!
    //! The suite pins, per function: the canonical happy-path transform, the
    //! tricky edge cases the prose docs promise (diacritic round-trips,
    //! punctuation stripping, trunk/international phone prefixes, per-country
    //! E.164 dispatch, address-line tokenisation, Gmail dot-folding), the
    //! degenerate empty/whitespace inputs, and — crucially — **idempotence**
    //! (`f(f(x)) == f(x)`), since the matching engine may normalise an
    //! already-normalised value. Many assertions use deliberately well-known
    //! example numbers (UK Ofcom reserved `7700 900xxx`, US `555` exchange)
    //! so no real PII appears in tests.
    use super::*;

    // ---------- normalize_name ----------

    /// Pins that internal whitespace collapses to a single space and the ends
    /// are trimmed — the layout-insensitivity the matcher relies on.
    #[test]
    fn normalize_name_collapses_whitespace_and_trims() {
        assert_eq!(Normalizer::normalize_name("  John  Smith  "), "john smith");
    }

    /// Pins that apostrophes, hyphens and full stops are removed, so spelling
    /// variants like `O'Brien`/`OBrien` and `Mary-Jane`/`MaryJane` unify.
    #[test]
    fn normalize_name_strips_ascii_punctuation() {
        assert_eq!(Normalizer::normalize_name("O'Brien"), "obrien");
        assert_eq!(Normalizer::normalize_name("Mary-Jane"), "maryjane");
        assert_eq!(Normalizer::normalize_name("Dr. Who"), "dr who");
    }

    /// Pins the diacritic-stripping round-trip: accented Latin letters fold to
    /// their base form, and letters with no NFKD decomposition (`ł`) survive.
    #[test]
    fn normalize_name_strips_diacritics() {
        assert_eq!(Normalizer::normalize_name("José"), "jose");
        assert_eq!(Normalizer::normalize_name("Siân"), "sian");
        // Round-trip of common accented forms to their plain ASCII base.
        assert_eq!(Normalizer::normalize_name("naïve"), "naive");
        assert_eq!(Normalizer::normalize_name("crème"), "creme");
        // ŷ and ŵ decompose to base + combining mark; ł has no NFKD
        // decomposition, so it is only lowercased and survives intact.
        assert_eq!(Normalizer::normalize_name("Łŷŵ"), "ływ");
    }

    /// Pins that empty and whitespace-only inputs round-trip to the empty
    /// string rather than panicking or producing stray spaces.
    #[test]
    fn normalize_name_handles_empty_and_whitespace() {
        assert_eq!(Normalizer::normalize_name(""), "");
        assert_eq!(Normalizer::normalize_name("   "), "");
        assert_eq!(Normalizer::normalize_name("\t\n"), "");
    }

    /// Pins case-folding, including mixed-case surnames like `McDONALD`.
    #[test]
    fn normalize_name_lowercases() {
        assert_eq!(Normalizer::normalize_name("MARY"), "mary");
        assert_eq!(Normalizer::normalize_name("McDONALD"), "mcdonald");
    }

    /// Pins idempotence over a spread of tricky inputs: feeding the output
    /// back in must be a fixed point, because the matcher may re-normalise.
    #[test]
    fn normalize_name_is_idempotent() {
        for input in [
            "  John  Smith  ",
            "O'Brien-Jones",
            "JOSÉ MARÍA",
            "",
            "  ",
            "Siân",
        ] {
            let once = Normalizer::normalize_name(input);
            // Second pass must equal the first — the fixed-point guarantee.
            let twice = Normalizer::normalize_name(&once);
            assert_eq!(once, twice, "not idempotent for {input:?}");
        }
    }

    /// Pins the documented limitation that only ASCII punctuation is stripped:
    /// a curly apostrophe (U+2019) survives, so upstream must ASCII-fold first.
    #[test]
    fn normalize_name_does_not_normalise_unicode_punctuation() {
        // Curly apostrophe (U+2019) is intentionally not stripped.
        // This is documented in agents/normalization.md as a known limitation.
        let with_curly = Normalizer::normalize_name("O\u{2019}Brien");
        assert!(with_curly.contains('\u{2019}'));
    }

    // ---------- normalize_postcode ----------

    /// Pins that postcodes are uppercased for case-insensitive comparison.
    #[test]
    fn normalize_postcode_uppercases() {
        assert_eq!(Normalizer::normalize_postcode("cf10 1aa"), "CF101AA");
    }

    /// Pins that all whitespace (including tabs and the conventional UK space)
    /// is removed, so spaced and unspaced postcodes compare equal.
    #[test]
    fn normalize_postcode_strips_all_whitespace() {
        assert_eq!(Normalizer::normalize_postcode("CF10 1AA"), "CF101AA");
        assert_eq!(Normalizer::normalize_postcode(" CF10  1AA "), "CF101AA");
        assert_eq!(Normalizer::normalize_postcode("CF10\t1AA"), "CF101AA");
    }

    /// Pins that empty / whitespace-only postcodes normalise to the empty string.
    #[test]
    fn normalize_postcode_handles_empty() {
        assert_eq!(Normalizer::normalize_postcode(""), "");
        assert_eq!(Normalizer::normalize_postcode("   "), "");
    }

    /// Pins postcode idempotence across spaced, unspaced and padded inputs.
    #[test]
    fn normalize_postcode_is_idempotent() {
        for input in ["cf10 1aa", "SW1A 2AA", "  EH8 9YL  ", ""] {
            let once = Normalizer::normalize_postcode(input);
            let twice = Normalizer::normalize_postcode(&once);
            assert_eq!(once, twice);
        }
    }

    // ---------- normalize_phone ----------

    /// Pins that the UK national trunk `0` is dropped from a domestic mobile.
    #[test]
    fn normalize_phone_strips_uk_trunk_prefix() {
        assert_eq!(Normalizer::normalize_phone("07700 900123"), "7700900123");
    }

    /// Pins that the `+44` international form reduces to the same bare NSN as
    /// the national form above.
    #[test]
    fn normalize_phone_strips_plus_44_international() {
        assert_eq!(Normalizer::normalize_phone("+44 7700 900123"), "7700900123");
    }

    /// Pins that the `0044` international access form also reduces to the NSN.
    #[test]
    fn normalize_phone_strips_0044_international() {
        assert_eq!(
            Normalizer::normalize_phone("0044 7700 900123"),
            "7700900123"
        );
    }

    /// Pins that brackets and spaces (a UK landline written `(029) 2034 5678`)
    /// are discarded down to digits, with the trunk `0` removed.
    #[test]
    fn normalize_phone_handles_brackets_and_spaces() {
        assert_eq!(Normalizer::normalize_phone("(029) 2034 5678"), "2920345678");
    }

    /// Pins that input with no digits (empty or punctuation-only) yields "".
    #[test]
    fn normalize_phone_handles_empty() {
        assert_eq!(Normalizer::normalize_phone(""), "");
        assert_eq!(Normalizer::normalize_phone("---"), "");
    }

    /// Pins the length guard on the `44` rule: a short `44…` number is treated
    /// as domestic digits, not an international prefix, so `44` is kept.
    #[test]
    fn normalize_phone_does_not_strip_44_if_too_short() {
        // 44 followed by fewer than 10 more digits: keep the 44 (not international prefix).
        assert_eq!(Normalizer::normalize_phone("4412345"), "4412345");
    }

    /// Pins idempotence of the legacy normaliser across all input layouts.
    #[test]
    fn normalize_phone_is_idempotent() {
        for input in [
            "07700 900123",
            "+44 7700 900123",
            "0044 7700 900123",
            "(029) 2034 5678",
            "",
        ] {
            let once = Normalizer::normalize_phone(input);
            let twice = Normalizer::normalize_phone(&once);
            assert_eq!(once, twice, "not idempotent for {input:?}");
        }
    }

    /// Pins the `len > 1` guard: a lone `0` is preserved rather than stripped
    /// to the empty string.
    #[test]
    fn normalize_phone_keeps_lone_zero() {
        // A bare "0" is not stripped (guard: len > 1).
        assert_eq!(Normalizer::normalize_phone("0"), "0");
    }

    // ---------- phonetic_code ----------

    /// Pins the core Soundex purpose: `Smith` and `Smyth` share a code so the
    /// matcher can catch spelling variants.
    #[test]
    fn phonetic_code_groups_smith_and_smyth() {
        assert_eq!(
            Normalizer::phonetic_code("Smith"),
            Normalizer::phonetic_code("Smyth")
        );
    }

    /// Pins that `Stephen`/`Steven` (ph vs v) collapse to the same code.
    #[test]
    fn phonetic_code_groups_stephen_and_steven() {
        assert_eq!(
            Normalizer::phonetic_code("Stephen"),
            Normalizer::phonetic_code("Steven")
        );
    }

    /// Pins that unrelated surnames produce distinct codes, so Soundex does
    /// not over-collapse and create false matches.
    #[test]
    fn phonetic_code_distinguishes_different_families() {
        assert_ne!(
            Normalizer::phonetic_code("Jones"),
            Normalizer::phonetic_code("Smith")
        );
        assert_ne!(
            Normalizer::phonetic_code("Anderson"),
            Normalizer::phonetic_code("Zimmerman")
        );
    }

    /// Regression net pinning exact Soundex codes from the underlying crate,
    /// so an accidental algorithm change is caught immediately.
    #[test]
    fn phonetic_code_specific_values() {
        // Pinned values from the underlying soundex crate; act as a regression net.
        assert_eq!(Normalizer::phonetic_code("Smith"), "S530");
        assert_eq!(Normalizer::phonetic_code("Smyth"), "S530");
        assert_eq!(Normalizer::phonetic_code("Jones"), "J520");
        assert_eq!(Normalizer::phonetic_code("Johnson"), "J525");
    }

    /// Pins that empty / whitespace-only names yield "" rather than a default
    /// Soundex value (which would falsely group all blank names).
    #[test]
    fn phonetic_code_handles_empty() {
        assert_eq!(Normalizer::phonetic_code(""), "");
        assert_eq!(Normalizer::phonetic_code("   "), "");
    }

    /// Pins that coding is case-insensitive (normalisation lowercases first).
    #[test]
    fn phonetic_code_is_case_insensitive() {
        assert_eq!(
            Normalizer::phonetic_code("SMITH"),
            Normalizer::phonetic_code("smith")
        );
    }

    // ---------- normalize_phone_e164 ----------

    /// Pins that the three UK input layouts (`+44`, `0044`, national `0…`)
    /// plus a punctuated variant all converge on one E.164 string — the whole
    /// point of the international-aware normaliser.
    #[test]
    fn e164_uk_layouts_canonicalise_identically() {
        // Ofcom reserved drama number; never a real subscriber.
        let canonical = Some("+447700900123".to_string());
        assert_eq!(
            Normalizer::normalize_phone_e164("+44 7700 900123", Some("GB")),
            canonical,
        );
        assert_eq!(
            Normalizer::normalize_phone_e164("0044 7700 900123", Some("GB")),
            canonical,
        );
        assert_eq!(
            Normalizer::normalize_phone_e164("07700 900123", Some("GB")),
            canonical,
        );
        assert_eq!(
            Normalizer::normalize_phone_e164("(07700) 900-123", Some("GB")),
            canonical,
        );
    }

    /// Pins France (dial code 33, trunk `0`): `+33`, `0033` and national
    /// `01…` layouts all canonicalise identically.
    #[test]
    fn e164_french_layouts_canonicalise_identically() {
        let canonical = Some("+33123456789".to_string());
        assert_eq!(
            Normalizer::normalize_phone_e164("+33 1 23 45 67 89", Some("FR")),
            canonical,
        );
        assert_eq!(
            Normalizer::normalize_phone_e164("0033 1 23 45 67 89", Some("FR")),
            canonical,
        );
        assert_eq!(
            Normalizer::normalize_phone_e164("01 23 45 67 89", Some("FR")),
            canonical,
        );
    }

    /// Pins the `trunk_prefix: None` case (Spain): the leading digit of a
    /// national number must be preserved, not mistaken for a trunk `0`.
    #[test]
    fn e164_spain_has_no_national_trunk_prefix() {
        // Spain switched to no trunk-0 in 1998; a bare 9-digit national
        // number is the canonical form.
        assert_eq!(
            Normalizer::normalize_phone_e164("912 345 678", Some("ES")),
            Some("+34912345678".to_string()),
        );
        assert_eq!(
            Normalizer::normalize_phone_e164("+34 912 345 678", None),
            Some("+34912345678".to_string()),
        );
    }

    /// Pins the 3-digit dial code path (Ireland, 353): longest-prefix lookup
    /// must select `353`, not a shorter code that is a prefix of it.
    #[test]
    fn e164_ireland_three_digit_dial_code() {
        assert_eq!(
            Normalizer::normalize_phone_e164("+353 1 234 5678", None),
            Some("+35312345678".to_string()),
        );
        assert_eq!(
            Normalizer::normalize_phone_e164("01 234 5678", Some("IE")),
            Some("+35312345678".to_string()),
        );
    }

    /// Pins the shared-dial-code NANP case: US and Canada both use `1`, have
    /// no trunk prefix, and the `555` exchange keeps the example non-routable.
    #[test]
    fn e164_nanp_handles_us_and_canada() {
        assert_eq!(
            Normalizer::normalize_phone_e164("(415) 555-1234", Some("US")),
            Some("+14155551234".to_string()),
        );
        assert_eq!(
            Normalizer::normalize_phone_e164("+1 415 555 1234", None),
            Some("+14155551234".to_string()),
        );
        // Canada uses the same dial code (1); the canonical E.164 form is
        // identical regardless of whether the caller said US or CA.
        assert_eq!(
            Normalizer::normalize_phone_e164("(416) 555-1234", Some("CA")),
            Some("+14165551234".to_string()),
        );
    }

    // ---------- T-19: 35-scheme jurisdiction coverage ----------

    /// Pins the non-`0` trunk-prefix branch: Lithuania uses `8`, so a national
    /// `8…` mobile must canonicalise to the same E.164 form as `+370…`.
    #[test]
    fn e164_lithuania_uses_eight_as_trunk_prefix() {
        // Lithuania's national trunk prefix is `8`, not `0`. National
        // dialling form `8 612 34567` (mobile) canonicalises to the same
        // E.164 string as the explicit `+370 612 34567` form.
        assert_eq!(
            Normalizer::normalize_phone_e164("8 612 34567", Some("LT")),
            Some("+37061234567".to_string()),
        );
        assert_eq!(
            Normalizer::normalize_phone_e164("+370 612 34567", None),
            Some("+37061234567".to_string()),
        );
    }

    /// Pins Greece (`trunk_prefix: None`): the leading area-code digit is part
    /// of the NSN and must not be stripped.
    #[test]
    fn e164_greece_has_no_national_trunk_prefix() {
        // GR national-significant numbers begin with the area code (the
        // leading zero seen in older publications is no longer a trunk).
        assert_eq!(
            Normalizer::normalize_phone_e164("+30 210 123 4567", None),
            Some("+302101234567".to_string()),
        );
        assert_eq!(
            Normalizer::normalize_phone_e164("210 123 4567", Some("GR")),
            Some("+302101234567".to_string()),
        );
    }

    /// Pins Romania (dial 40, trunk `0`): the national trunk zero is removed.
    #[test]
    fn e164_romania_strips_trunk_zero() {
        assert_eq!(
            Normalizer::normalize_phone_e164("0721 234 567", Some("RO")),
            Some("+40721234567".to_string()),
        );
        assert_eq!(
            Normalizer::normalize_phone_e164("+40 721 234 567", None),
            Some("+40721234567".to_string()),
        );
    }

    /// Pins Czechia (3-digit dial 420, no trunk): national and international
    /// forms agree.
    #[test]
    fn e164_czech_no_trunk_prefix() {
        assert_eq!(
            Normalizer::normalize_phone_e164("+420 234 567 890", None),
            Some("+420234567890".to_string()),
        );
        assert_eq!(
            Normalizer::normalize_phone_e164("234 567 890", Some("CZ")),
            Some("+420234567890".to_string()),
        );
    }

    /// Pins Iceland's short 7-digit NSN against its `min_nsn`/`max_nsn` bounds.
    #[test]
    fn e164_iceland_seven_digit_nsn() {
        assert_eq!(
            Normalizer::normalize_phone_e164("+354 412 3456", None),
            Some("+3544123456".to_string()),
        );
    }

    /// Pins that adjacent 3-digit dial codes (Croatia 385 vs Slovenia 386)
    /// resolve to distinct countries and distinct E.164 strings.
    #[test]
    fn e164_distinguishes_overlapping_three_digit_dial_codes() {
        // Croatia (385) vs Slovenia (386): adjacent dial codes, both
        // use a `0` trunk, but the canonical E.164 forms remain distinct.
        let hr = Normalizer::normalize_phone_e164("+385 91 234 5678", None);
        let si = Normalizer::normalize_phone_e164("+386 41 234 567", None);
        assert!(hr.is_some());
        assert!(si.is_some());
        assert_ne!(hr, si);
    }

    /// Pins the core multinational win: the same national-format digits under
    /// two different default countries must not collide in E.164 form.
    #[test]
    fn e164_distinguishes_countries_with_overlapping_national_digits() {
        // The "same" national-format digits in two countries must yield
        // different E.164 strings — this is precisely the disambiguation
        // the new normaliser provides.
        let uk = Normalizer::normalize_phone_e164("07700 900123", Some("GB"));
        let fr = Normalizer::normalize_phone_e164("07700 90012", Some("FR"));
        assert!(uk.is_some());
        assert!(fr.is_some());
        assert_ne!(uk, fr);
    }

    /// Pins the `None` contract for ambiguity: national-format input with no
    /// default country cannot be resolved, so the function refuses to guess.
    #[test]
    fn e164_returns_none_when_default_country_missing_and_no_marker() {
        // Ambiguous national-format input with no default country.
        assert_eq!(Normalizer::normalize_phone_e164("07700 900123", None), None);
    }

    /// Pins `None` for a dial code (`+999`) that is not in the country table.
    #[test]
    fn e164_returns_none_for_unknown_dial_code() {
        assert_eq!(Normalizer::normalize_phone_e164("+999 1234567", None), None);
    }

    /// Pins `None` for inputs with no usable digits (empty, lone `+`, punctuation).
    #[test]
    fn e164_returns_none_for_empty_or_punctuation_only() {
        assert_eq!(Normalizer::normalize_phone_e164("", Some("GB")), None);
        assert_eq!(Normalizer::normalize_phone_e164("+", Some("GB")), None);
        assert_eq!(Normalizer::normalize_phone_e164("()-", Some("GB")), None);
    }

    /// Pins the NSN length bounds: a number too short or too long for the
    /// country's plan fails confidently rather than emitting a bad E.164.
    #[test]
    fn e164_returns_none_for_too_short_or_too_long_nsn() {
        // GB NSN must be 7..=11 digits.
        assert_eq!(Normalizer::normalize_phone_e164("+44 1", None), None);
        assert_eq!(
            Normalizer::normalize_phone_e164("+44 123456789012345", None),
            None,
        );
    }

    /// Pins `None` when the default-country ISO code is not in the table.
    #[test]
    fn e164_rejects_unknown_default_country() {
        // "XX" is not in the table; without an explicit international
        // marker the function cannot guess.
        assert_eq!(
            Normalizer::normalize_phone_e164("07700 900123", Some("XX")),
            None,
        );
    }

    /// Pins idempotence of E.164: feeding a canonical `+CC…` string back in
    /// (even with a mismatched default country) returns the same string,
    /// because the explicit `+` marker overrides the default.
    #[test]
    fn e164_is_idempotent_on_canonical_form() {
        for input in [
            "+44 7700 900123",
            "+33 1 23 45 67 89",
            "(415) 555-1234",
            "+353 1 234 5678",
            "+34 912 345 678",
        ] {
            let once = Normalizer::normalize_phone_e164(input, Some("GB")).expect("parses");
            // Re-normalising the canonical output is a fixed point.
            let twice = Normalizer::normalize_phone_e164(&once, Some("GB")).expect("idempotent");
            assert_eq!(once, twice, "not idempotent for {input:?}");
        }
    }

    /// Pins that the default-country ISO lookup is case-insensitive (`gb` == `GB`).
    #[test]
    fn e164_default_country_lookup_is_case_insensitive() {
        let lower = Normalizer::normalize_phone_e164("07700 900123", Some("gb"));
        let upper = Normalizer::normalize_phone_e164("07700 900123", Some("GB"));
        assert_eq!(lower, upper);
        assert!(lower.is_some());
    }

    /// Pins the `00CC…` international-access path: `00` is consumed and the
    /// remainder is parsed like an explicit `+` number, with no default needed.
    #[test]
    fn e164_handles_double_zero_international_access_form() {
        assert_eq!(
            Normalizer::normalize_phone_e164("00 33 1 23 45 67 89", None),
            Some("+33123456789".to_string()),
        );
    }

    // ---------- expand_street_abbreviations ----------

    /// Pins the street-type expansion table for the common abbreviations
    /// (St→street, Rd→road, Blvd→boulevard, Ave→avenue, Ln→lane).
    #[test]
    fn expand_street_replaces_common_abbreviations() {
        assert_eq!(
            Normalizer::expand_street_abbreviations("123 High St"),
            "123 High street",
        );
        assert_eq!(
            Normalizer::expand_street_abbreviations("10 Downing Rd"),
            "10 Downing road",
        );
        assert_eq!(
            Normalizer::expand_street_abbreviations("12 Sunset Blvd"),
            "12 Sunset boulevard",
        );
        assert_eq!(
            Normalizer::expand_street_abbreviations("1 Park Ave"),
            "1 Park avenue",
        );
        assert_eq!(
            Normalizer::expand_street_abbreviations("5 Cherry Ln"),
            "5 Cherry lane",
        );
    }

    /// Pins directional abbreviation expansion (N→north, SW→southwest), which
    /// shares the table with street types.
    #[test]
    fn expand_street_replaces_directionals() {
        assert_eq!(
            Normalizer::expand_street_abbreviations("45 N Park Ave"),
            "45 north Park avenue",
        );
        assert_eq!(
            Normalizer::expand_street_abbreviations("100 SW 5th St"),
            "100 southwest 5th street",
        );
    }

    /// Pins that a single trailing `.` or `,` on a token is stripped before
    /// table lookup (`St.` and `Blvd,` still expand).
    #[test]
    fn expand_street_strips_trailing_period_or_comma() {
        assert_eq!(
            Normalizer::expand_street_abbreviations("123 High St."),
            "123 High street",
        );
        assert_eq!(
            Normalizer::expand_street_abbreviations("12 Sunset Blvd,"),
            "12 Sunset boulevard",
        );
    }

    /// Pins that non-abbreviation tokens pass through verbatim, preserving
    /// their original casing.
    #[test]
    fn expand_street_passes_unknown_tokens_through() {
        assert_eq!(
            Normalizer::expand_street_abbreviations("Buckingham Palace"),
            "Buckingham Palace",
        );
    }

    /// Pins idempotence: long forms are not re-expanded, so re-running is safe.
    #[test]
    fn expand_street_is_idempotent_on_already_expanded_input() {
        for input in [
            "123 High St",
            "45 N Park Ave",
            "10 Downing Rd",
            "Buckingham Palace",
        ] {
            let once = Normalizer::expand_street_abbreviations(input);
            let twice = Normalizer::expand_street_abbreviations(&once);
            assert_eq!(once, twice, "not idempotent for {input:?}");
        }
    }

    /// Pins empty / whitespace-only input to "".
    #[test]
    fn expand_street_handles_empty_and_whitespace_only() {
        assert_eq!(Normalizer::expand_street_abbreviations(""), "");
        assert_eq!(Normalizer::expand_street_abbreviations("   "), "");
    }

    // ---------- normalize_address_line ----------

    /// Pins that the full address pipeline makes abbreviated and spelled-out
    /// forms (`High St` vs `High Street`) compare equal.
    #[test]
    fn normalize_address_line_unifies_abbreviated_and_full_forms() {
        assert_eq!(
            Normalizer::normalize_address_line("123 High St"),
            Normalizer::normalize_address_line("123 High Street"),
        );
        assert_eq!(
            Normalizer::normalize_address_line("45 N Park Ave"),
            Normalizer::normalize_address_line("45 North Park Avenue"),
        );
    }

    /// Pins that the name-normalisation tail (punctuation, case, whitespace)
    /// is applied after abbreviation expansion.
    #[test]
    fn normalize_address_line_handles_punctuation_and_case() {
        assert_eq!(
            Normalizer::normalize_address_line("10, DOWNING Street."),
            "10 downing street",
        );
    }

    /// Pins address-line idempotence across the combined pipeline.
    #[test]
    fn normalize_address_line_is_idempotent() {
        for input in [
            "123 High St",
            "  45 N Park Ave  ",
            "10, Downing Street.",
            "",
        ] {
            let once = Normalizer::normalize_address_line(input);
            let twice = Normalizer::normalize_address_line(&once);
            assert_eq!(once, twice, "not idempotent for {input:?}");
        }
    }

    // ---------- parse_address_line ----------

    /// Pins the simplest decomposition: leading number → `house_number`, rest
    /// → normalised `street`, no `unit`.
    #[test]
    fn parse_address_extracts_simple_house_number() {
        let p = Normalizer::parse_address_line("123 High Street");
        assert_eq!(p.house_number.as_deref(), Some("123"));
        assert_eq!(p.unit, None);
        assert_eq!(p.street, "high street");
    }

    /// Pins the single-letter house-number suffix (`10A`), uppercased.
    #[test]
    fn parse_address_handles_alphanumeric_house_number() {
        let p = Normalizer::parse_address_line("10A Downing St");
        assert_eq!(p.house_number.as_deref(), Some("10A"));
        assert_eq!(p.street, "downing street");
    }

    /// Pins the look-ahead guard that stops the suffix rule from eating a
    /// street-name word: `10 Apple…` keeps `10` and leaves `Apple` in `street`.
    #[test]
    fn parse_address_does_not_greedily_consume_street_name() {
        // "10 Apple Tree Lane" — `Apple` must not be absorbed into the
        // house number because two consecutive alphabetic characters
        // signal it's part of the street name.
        let p = Normalizer::parse_address_line("10 Apple Tree Lane");
        assert_eq!(p.house_number.as_deref(), Some("10"));
        assert_eq!(p.street, "apple tree lane");
    }

    /// Pins the UK `Flat` unit prefix: keyword + identifier extracted and
    /// lowercased, then `house_number` and `street` parsed from the remainder.
    #[test]
    fn parse_address_recognises_flat_prefix() {
        let p = Normalizer::parse_address_line("Flat 2A, 10 Downing Street");
        assert_eq!(p.unit.as_deref(), Some("flat 2a"));
        assert_eq!(p.house_number.as_deref(), Some("10"));
        assert_eq!(p.street, "downing street");
    }

    /// Pins the US `Apt` unit prefix on a typical US address.
    #[test]
    fn parse_address_recognises_apt_prefix() {
        let p = Normalizer::parse_address_line("Apt 5, 1600 Pennsylvania Ave");
        assert_eq!(p.unit.as_deref(), Some("apt 5"));
        assert_eq!(p.house_number.as_deref(), Some("1600"));
        assert_eq!(p.street, "pennsylvania avenue");
    }

    /// Pins the remaining recognised unit keywords (Suite/Ste/Unit/Room) all
    /// take the same extraction path.
    #[test]
    fn parse_address_recognises_suite_and_ste_and_unit_and_room() {
        for input in [
            "Suite 12, 100 Main St",
            "Ste 12, 100 Main St",
            "Unit 12, 100 Main St",
            "Room 12, 100 Main St",
        ] {
            let p = Normalizer::parse_address_line(input);
            assert!(p.unit.is_some(), "no unit for {input:?}");
            assert_eq!(p.house_number.as_deref(), Some("100"));
            assert_eq!(p.street, "main street");
        }
    }

    /// Pins graceful degradation: an input with no leading number and no unit
    /// keyword puts everything in `street`.
    #[test]
    fn parse_address_no_leading_number_falls_back_to_street_only() {
        let p = Normalizer::parse_address_line("Buckingham Palace");
        assert_eq!(p.house_number, None);
        assert_eq!(p.unit, None);
        assert_eq!(p.street, "buckingham palace");
    }

    /// Pins that empty input yields all-`None` fields and an empty `street`.
    #[test]
    fn parse_address_empty_input_yields_empty_street() {
        let p = Normalizer::parse_address_line("");
        assert_eq!(p.house_number, None);
        assert_eq!(p.unit, None);
        assert_eq!(p.street, "");
    }

    /// Pins the `Serialize`/`Deserialize` contract: a parsed line round-trips
    /// through JSON unchanged, so it can be embedded in downstream models.
    #[test]
    fn parse_address_round_trips_through_serde() {
        let p = Normalizer::parse_address_line("Flat 2A, 10A Downing Street");
        let json = serde_json::to_string(&p).unwrap();
        // Deserialising the JSON must reproduce the exact same struct.
        let back: ParsedAddressLine = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }

    /// Pins that a lowercase house-number suffix (`10a`) is uppercased to `10A`.
    #[test]
    fn parse_address_uppercases_house_number_suffix() {
        let p = Normalizer::parse_address_line("10a Downing St");
        assert_eq!(p.house_number.as_deref(), Some("10A"));
    }

    // ---------- normalize_email ----------

    /// Pins the baseline email canonicalisation: surrounding whitespace is
    /// trimmed and the whole address is lowercased so two records that
    /// differ only in case/padding compare equal.
    #[test]
    fn normalize_email_lowercases_and_trims() {
        assert_eq!(
            Normalizer::normalize_email("  Alice@Example.ORG  ", false),
            Some("alice@example.org".into()),
        );
    }

    /// Pins idempotence on already-canonical input: a well-formed,
    /// lowercase address must pass through unchanged (no spurious mangling).
    #[test]
    fn normalize_email_preserves_well_formed_input() {
        assert_eq!(
            Normalizer::normalize_email("alice@example.org", false),
            Some("alice@example.org".into()),
        );
    }

    /// Pins the structural-validity gate: a string with no `@` is not an
    /// address and must return `None` rather than a bogus canonical value.
    #[test]
    fn normalize_email_rejects_missing_at_sign() {
        assert_eq!(Normalizer::normalize_email("no-at-sign", false), None);
    }

    /// Pins that both halves of the address are required: an empty
    /// localpart (`@example.org`) or empty domain (`alice@`) is rejected,
    /// since neither can identify a mailbox.
    #[test]
    fn normalize_email_rejects_empty_localpart_or_domain() {
        assert_eq!(Normalizer::normalize_email("@example.org", false), None);
        assert_eq!(Normalizer::normalize_email("alice@", false), None);
    }

    /// Pins that exactly one `@` is required: an ambiguous `a@b@c` cannot
    /// be split into a single localpart/domain pair, so it is rejected.
    #[test]
    fn normalize_email_rejects_multiple_at_signs() {
        assert_eq!(Normalizer::normalize_email("a@b@c", false), None);
    }

    /// Pins that empty and whitespace-only input yield `None`: trimming
    /// must not turn blank input into a spurious empty address.
    #[test]
    fn normalize_email_rejects_empty_and_whitespace() {
        assert_eq!(Normalizer::normalize_email("", false), None);
        assert_eq!(Normalizer::normalize_email("   ", false), None);
    }

    /// Pins Gmail-specific dot folding: Gmail ignores dots in the
    /// localpart, so `j.smith` and `jsmith` are the same mailbox. With
    /// folding enabled the normaliser strips localpart dots so the two
    /// forms compare equal.
    #[test]
    fn normalize_email_gmail_dot_folding_strips_dots_in_localpart() {
        assert_eq!(
            Normalizer::normalize_email("j.smith@gmail.com", true),
            Some("jsmith@gmail.com".into()),
        );
        assert_eq!(
            Normalizer::normalize_email("J.S.M.I.T.H@gmail.com", true),
            Some("jsmith@gmail.com".into()),
        );
    }

    /// Pins Gmail `+tag` sub-addressing folding: everything from the `+`
    /// to the `@` is a delivery tag, not part of the mailbox, so it is
    /// dropped under folding. Also covers the `googlemail.com` alias domain.
    #[test]
    fn normalize_email_gmail_dot_folding_strips_plus_tag() {
        assert_eq!(
            Normalizer::normalize_email("jsmith+work@gmail.com", true),
            Some("jsmith@gmail.com".into()),
        );
        assert_eq!(
            Normalizer::normalize_email("j.smith+anything@googlemail.com", true),
            Some("jsmith@googlemail.com".into()),
        );
    }

    /// Pins that dot/plus folding is Gmail-only: non-Gmail domains keep
    /// their localpart verbatim, because dots and `+` tags are significant
    /// elsewhere and folding them would merge distinct mailboxes.
    #[test]
    fn normalize_email_gmail_dot_folding_does_not_touch_other_domains() {
        assert_eq!(
            Normalizer::normalize_email("j.smith@example.org", true),
            Some("j.smith@example.org".into()),
        );
        assert_eq!(
            Normalizer::normalize_email("jsmith+work@example.org", true),
            Some("jsmith+work@example.org".into()),
        );
    }

    /// Pins that folding is opt-in: with the flag off, even a Gmail
    /// address keeps its localpart dots, so callers who do not want the
    /// Gmail special-case get conservative, literal behaviour.
    #[test]
    fn normalize_email_dot_folding_off_preserves_localpart_dots() {
        assert_eq!(
            Normalizer::normalize_email("j.smith@gmail.com", false),
            Some("j.smith@gmail.com".into()),
        );
    }

    /// Pins idempotence (`f(f(x)) == f(x)`) across the matrix of
    /// folding on/off and Gmail/non-Gmail: re-normalising an already
    /// canonical address must be a no-op, which is what lets the matcher
    /// compare canonical forms safely.
    #[test]
    fn normalize_email_is_idempotent_on_canonical_form() {
        for (input, fold) in [
            ("Alice@Example.ORG", false),
            ("j.smith@gmail.com", true),
            ("jsmith+x@gmail.com", true),
            ("user@host.tld", false),
        ] {
            let once = Normalizer::normalize_email(input, fold).expect("parses");
            let twice = Normalizer::normalize_email(&once, fold).expect("idempotent");
            assert_eq!(once, twice, "not idempotent for {input:?} fold={fold}");
        }
    }

    /// Pins a folding edge case: if stripping dots leaves an empty
    /// localpart (input was all dots), the result is not a valid address
    /// and must be `None` rather than `@gmail.com`.
    #[test]
    fn normalize_email_gmail_dot_folding_rejects_empty_localpart_after_folding() {
        // A localpart that is entirely dots (or all stripped to empty) is
        // not a valid address.
        assert_eq!(Normalizer::normalize_email("...@gmail.com", true), None);
    }

    /// Pins a tokeniser ambiguity guard: the street-type abbreviation
    /// `St` (Street/Saint) must not be mistaken for the unit prefix `Ste`
    /// (Suite). Only a literal `ste` token sets `unit`.
    #[test]
    fn parse_address_does_not_treat_st_as_unit_prefix() {
        // The "St" street-type abbreviation must not be confused with the
        // "Ste" unit prefix. Only a literal "ste" token triggers the unit
        // path.
        let p = Normalizer::parse_address_line("St Mary's Road");
        assert_eq!(p.unit, None);
    }

    /// Pins tolerance for a common data-entry error: a leftover national
    /// trunk `0` immediately after the country code (`+44 0 7700 …`) is
    /// dropped so the number still canonicalises to valid E.164.
    #[test]
    fn e164_strips_trunk_zero_after_country_code() {
        // Some entry systems mistakenly keep the national trunk 0 after
        // the country code (e.g. "+44 0 7700 900123"). The normaliser
        // tolerates this.
        assert_eq!(
            Normalizer::normalize_phone_e164("+44 0 7700 900123", None),
            Some("+447700900123".to_string()),
        );
    }
}
