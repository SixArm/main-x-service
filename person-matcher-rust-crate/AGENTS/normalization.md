# Normalization — Agent Guide

See [`../spec.md`](../spec/index.md) §14 for the formal rules. This guide is the operational view.

## Why Normalise At All?

The research base (spec §5) is unanimous: most accuracy gains come from data standardisation, not from cleverer scoring. Garbage in, garbage out — no Jaro-Winkler trick rescues an apostrophe mismatch.

## What We Normalise

| Input                      | Algorithm                                                                                                                       | File                                             |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------ |
| Names                      | Lowercase + NFKD + drop combining marks + drop ASCII punctuation + collapse whitespace                                          | `src/normalizer.rs::normalize_name`              |
| Postcodes                  | Strip whitespace + uppercase                                                                                                    | `src/normalizer.rs::normalize_postcode`          |
| Phone numbers (legacy)     | Keep digits + strip `0044` / `+44` / leading `0`                                                                                | `src/normalizer.rs::normalize_phone`             |
| Phone numbers (E.164)      | Match `+CC` / `00CC` / default-country trunk, strip national trunk `0`, validate NSN length, return `+CCNNN…`                   | `src/normalizer.rs::normalize_phone_e164`        |
| Address line abbreviations | Whole-token expansion of `St`→`Street`, `Rd`→`Road`, `Ave`→`Avenue`, `N`→`North`, …                                             | `src/normalizer.rs::expand_street_abbreviations` |
| Address line (full)        | Abbreviation expansion + name-normalisation pipeline                                                                            | `src/normalizer.rs::normalize_address_line`      |
| Address line parsing       | Extract `house_number` (leading digits + optional `A`-style suffix), `unit` (Flat/Apt/Suite/…), `street` (normalised remainder) | `src/normalizer.rs::parse_address_line`          |
| Phonetic                   | Name normalisation then American Soundex (default — keep until empirical corpus exists; see T-9 / §21.4)                        | `src/normalizer.rs::phonetic_code`               |
| Email                      | Trim + lowercase + reject if missing `@` / empty localpart / empty domain; opt-in Gmail dot- and `+tag`-folding                 | `src/normalizer.rs::normalize_email`             |

## Worked Examples

- `"Siân"` → `"sian"` (NFKD splits `Siân` into `Sia` + combining circumflex + `n`; the combining mark is dropped.)
- `"O'Brien"` → `"obrien"` (apostrophe is ASCII punctuation.)
- `"MARY-JANE"` → `"maryjane"` (hyphen stripped, lowercased.)
- `"  John  Smith  "` → `"john smith"` (single spaces, trimmed.)
- `"CF10 1AA"` → `"CF101AA"`.
- `"+44 7700 900123"` → `"7700900123"` (legacy national-significant).
- `"+44 7700 900123"` → `"+447700900123"` (E.164, any default country).
- `"01 23 45 67 89"` with default `"FR"` → `"+33123456789"` (E.164).
- `"01 23 45 67 89"` with default `"GB"` → `"+44123456789"` (E.164) — the same digits canonicalise to a _different_ country, which is precisely the disambiguation E.164 buys.
- `"(415) 555-1234"` with default `"US"` → `"+14155551234"` (E.164, NANP has no national trunk).

## International Phone Numbers — Country Table

`Normalizer::normalize_phone_e164` consults a table of **39 jurisdictions** that covers every country for which the crate parses a national identifier scheme (T-19, §21.4). That includes the six original identifier jurisdictions (UK, FR, ES, IE, UK NI via the GB dial code, US), the major person-mobility partners (CA, DE, IT, NL, BE, PT, CH, AT, SE, NO, DK, FI, PL, AU, NZ, JP, CN, IN, BR, MX, ZA), and the 13 jurisdictions added in T-19 (BG, CZ, EE, GR, HR, IS, LI, LT, LV, MT, RO, SI, SK). Each entry records:

- ISO 3166-1 alpha-2 country code (`GB`, `FR`, `US`, …).
- Dial code (1–3 digits, no leading `+`).
- National trunk prefix (`Option<&'static str>` — typically `Some("0")`, but Lithuania uses `Some("8")` and several countries use `None`).
- NSN length range — the minimum and maximum number of digits in the national-significant number, used to reject malformed inputs.

The full table lives in `src/normalizer.rs` (`COUNTRY_PHONE_TABLE`); the canonical statement is `spec.md` §14.3.2. When adding a new country:

1. Add an entry to `COUNTRY_PHONE_TABLE`, citing the ITU-T E.164 country code and the national numbering plan's NSN range.
2. Extend `spec.md` §14.3.2's table and the supported-country list.
3. Add a unit test in `src/normalizer.rs::tests` for the within-country happy path and at least one integration test in `tests/integration_tests.rs` if matcher behaviour changes.

## Address Parsing

`Normalizer::parse_address_line` is a best-effort structural decomposition:

```text
"Flat 2A, 10 Downing Street"  →  ParsedAddressLine {
    unit:         Some("flat 2a"),
    house_number: Some("10"),
    street:       "downing street",
}
```

It runs in three stages:

1. **Unit prefix**: first token is matched against `{flat, apartment, apt, unit, suite, ste, room, rm}`. If recognised, the next alphanumeric run is the identifier; the result is `format!("{keyword} {identifier}")` lowercased.
2. **House number**: leading run of ASCII digits, optionally followed by a single alphabetic suffix (`"10A"`). The suffix is taken **only when not followed by another alphanumeric**, so `"10 Apple Tree Lane"` does not absorb the `A` of `Apple`.
3. **Street**: remainder, run through `normalize_address_line` (= `expand_street_abbreviations` + `normalize_name`).

The matcher's line-1 sub-score combines a Jaro-Winkler similarity on `parsed.street` with an exact-match score on `parsed.house_number`: 60% street + 40% house number when both sides have a house number, street similarity alone otherwise. See `spec.md` §12.4.1 for the canonical statement.

### Adding a New Street Abbreviation

1. Add an entry to `STREET_ABBREVIATIONS` in `src/normalizer.rs`. Keep the long form lowercase so it survives the downstream name pipeline unchanged.
2. Extend `spec.md` §14.4a.1's table.
3. Add a unit test covering the new abbreviation and at least one combined-pipeline assertion.

### Adding a New Unit Prefix

1. Add the keyword to `UNIT_PREFIXES` in `src/normalizer.rs`.
2. Extend `spec.md` §14.4a.3 step 2.
3. Add a unit test asserting `parse_address_line` returns the keyword + identifier.

## Matcher Wiring

`MatchingEngine::score_phone` prefers the E.164 form and falls back to the legacy national-significant form:

1. Compute `normalize_phone_e164(phone1, cc)` and `normalize_phone_e164(phone2, cc)` where `cc = MatchConfig::phone_default_country` (defaults to `Some("GB")`).
2. If both parse, `phone_score = 1.0` iff the canonical strings are equal, else `0.0`.
3. Otherwise compare `normalize_phone(phone1) == normalize_phone(phone2)`.

This means cross-country deployments should set `phone_default_country` to the person population's predominant jurisdiction (or `None` to refuse to guess). The fallback path preserves behaviour for inputs the country table does not cover.

## What We Do Not Normalise (Yet)

- **`local_id`** — intentionally not normalised AND not scored. Different practices may issue colliding IDs.

(Note: `middle_name` is now scored as part of the given-name component — see spec FR-49 / §12.2. It is normalised via the same `normalize_name` pipeline as the given and family names.)

## Adding a New Normaliser

1. Add the new public method on `Normalizer` in `src/normalizer.rs`.
2. Add unit tests covering: empty input, all-whitespace input, a realistic happy-path input, and a diacritic case if the field can contain Unicode names.
3. Update [`../spec.md`](../spec/index.md) §14 with the algorithm steps and a table of examples.
4. If a scoring path uses the new normaliser, document it in spec §12.2.

## Pitfalls

- ❌ Don't collapse double-barrelled surnames (`Lloyd-Webber`) into a single word without thinking — current `normalize_name` drops the hyphen, yielding `lloydwebber`. This is intentional but worth knowing.
- ❌ Don't apply name normalisation to United Kingdom National Health Service Numbers — the `united-kingdom-national-health-service-number` crate has its own parser. Calling `normalize_name` on a digit string would happen to work but couples concerns.
- ❌ Don't lowercase postcodes; the canonical form is uppercase.
- ❌ Don't trust that `Char::is_ascii_punctuation` covers Unicode punctuation. It does not — `’` (curly apostrophe, U+2019) would survive. If you need broader stripping, propose it via the spec.
- ❌ Don't compare `normalize_phone` output across countries — it's UK-centric and will silently collapse French / Italian national numbers to lookalike digit strings. Use `normalize_phone_e164` (or rely on the matcher, which already does) for multi-country data.
- ❌ Don't add a new country to `COUNTRY_PHONE_TABLE` without explicit trunk-prefix and NSN-range provenance. Guessing the range produces false-negative matches when legitimate numbers fall outside it.
- ❌ Don't extend the address parser to apply position-aware heuristics for `"St"` (Saint vs Street) without a corresponding spec update. The current rule "always expand to Street" is deliberately simple; introducing context-sensitivity risks regressions.
- ❌ Don't add house-number ranges to the `house_number` field (`"123-125 High St"`). The current parser captures only the leading number; widening this without thought changes the equality semantics of the matcher's line-1 sub-score.

## Idempotence

All normalisers SHOULD be idempotent: `f(f(x)) == f(x)`. If you add one that isn't, document the reason in `spec.md` §14 and add a property test.

---

## Detailed Normalisation Specifications

The following sections were lifted from `spec.md` §14 to keep the spec terse. They remain canonical for each normaliser's wire-level behaviour.

### Name Normalisation

Algorithm:

1. NFKD-decompose the input.
2. Drop characters classified as Unicode combining marks (`unicode_normalization::char::is_combining_mark`).
3. Drop ASCII punctuation (`char::is_ascii_punctuation`).
4. Lowercase the result.
5. Collapse runs of whitespace into single spaces; trim ends.

Examples:

| Input | Output |
|---|---|
| `"  John  Smith  "` | `"john smith"` |
| `"O'Brien"` | `"obrien"` |
| `"José"` | `"jose"` |
| `"MARY-JANE"` | `"maryjane"` |
| `"Siân"` | `"sian"` |

### Postcode Normalisation

1. Drop whitespace characters.
2. Uppercase.

`"CF10 1AA"` ⇒ `"CF101AA"`. `"cf10 1aa"` ⇒ `"CF101AA"`.

### Phone Normalisation

Two complementary normalisers cover phone-number handling:

#### Legacy national-significant form — `Normalizer::normalize_phone`

This is the UK-centric, infallible form used as a fallback by the matcher when the international form cannot parse.

1. Keep only ASCII digits.
2. If result starts with `0044` and is longer than 4 digits, drop the `0044` prefix.
3. Else, if result starts with `44` and is at least 12 digits, drop the `44` prefix.
4. Else, if result starts with `0` and is longer than 1 digit, drop the leading `0`.
5. Return the result.

Examples:

| Input | Output |
|---|---|
| `"07700 900123"` | `"7700900123"` |
| `"+44 7700 900123"` | `"7700900123"` |
| `"0044 7700 900123"` | `"7700900123"` |
| `"(029) 2034 5678"` | `"2920345678"` |

#### International E.164 form — `Normalizer::normalize_phone_e164`

Returns `Some("+CCNNN…")` when the input parses against a country in the supported table, otherwise `None`. The function accepts:

- `+CC…` — explicit international, the canonical input form.
- `00CC…` — international access code (common across Europe).
- `0…` — national format with a national trunk prefix; interpreted relative to `default_country` (passed by the caller, sourced from `MatchConfig::phone_default_country` in the matcher).
- `NSN…` — bare national-significant number; interpreted relative to `default_country`.

Algorithm:

1. Strip every character that is not an ASCII digit; remember whether the original input contained `+`.
2. If `+` was present, match the longest dial-code prefix from the supported table against the leading digits.
3. Else, if the digits begin with `00`, drop those two and match the longest dial-code prefix against what remains.
4. Else, if a `default_country` is supplied, look it up in the table.
5. If no country is found, return `None`.
6. Strip a single occurrence of the country's national trunk prefix from the remaining digits, if one is configured. The trunk prefix is country-specific (typically `"0"`, but Lithuania uses `"8"`); the field on `CountryPhoneInfo` is `trunk_prefix: Option<&'static str>`.
7. Reject when the remaining national-significant number is outside the country's `min_nsn..=max_nsn` length range.
8. Return `Some(format!("+{dial_code}{nsn}"))`.

Supported countries (ISO 3166-1 alpha-2 code, dial code, trunk prefix, NSN range; **39 jurisdictions** — one for every national identifier scheme the crate parses):

`GB +44 trunk-0 7..=11`, `FR +33 trunk-0 9..=9`, `DE +49 trunk-0 7..=13`, `ES +34 no-trunk 9..=9`, `IE +353 trunk-0 7..=11`, `IT +39 no-trunk 6..=12`, `NL +31 trunk-0 9..=9`, `BE +32 trunk-0 8..=9`, `PT +351 no-trunk 9..=9`, `CH +41 trunk-0 9..=9`, `AT +43 trunk-0 4..=13`, `SE +46 trunk-0 7..=13`, `NO +47 no-trunk 8..=8`, `DK +45 no-trunk 8..=8`, `FI +358 trunk-0 5..=12`, `PL +48 no-trunk 9..=9`, `AU +61 trunk-0 9..=9`, `NZ +64 trunk-0 8..=10`, `US +1 no-trunk 10..=10`, `CA +1 no-trunk 10..=10`, `JP +81 trunk-0 9..=10`, `CN +86 trunk-0 5..=12`, `IN +91 trunk-0 10..=10`, `BR +55 trunk-0 10..=11`, `MX +52 no-trunk 10..=10`, `ZA +27 trunk-0 9..=9`,

`BG +359 trunk-0 8..=9`, `CZ +420 no-trunk 9..=9`, `EE +372 no-trunk 7..=8`, `GR +30 no-trunk 10..=10`, `HR +385 trunk-0 8..=9`, `IS +354 no-trunk 7..=9`, `LI +423 no-trunk 7..=9`, **`LT +370 trunk-8 8..=8`**, `LV +371 no-trunk 8..=8`, `MT +356 no-trunk 8..=8`, `RO +40 trunk-0 9..=9`, `SI +386 trunk-0 8..=8`, `SK +421 trunk-0 9..=9` (added in T-19).

New countries SHOULD be added with explicit trunk-prefix and NSN-range provenance. Lithuania is the canonical example of a non-`0` trunk prefix; the abstraction (`Option<&'static str>`) supports any documented convention.

Examples (with `default_country = Some("GB")` unless noted):

| Input | `default_country` | Output |
|---|---|---|
| `"+44 7700 900123"` | any | `Some("+447700900123")` |
| `"0044 7700 900123"` | any | `Some("+447700900123")` |
| `"07700 900123"` | `"GB"` | `Some("+447700900123")` |
| `"07700 900123"` | `None` | `None` (ambiguous) |
| `"+33 1 23 45 67 89"` | any | `Some("+33123456789")` |
| `"01 23 45 67 89"` | `"FR"` | `Some("+33123456789")` |
| `"912 345 678"` | `"ES"` | `Some("+34912345678")` |
| `"(415) 555-1234"` | `"US"` | `Some("+14155551234")` |
| `"+999 1234567"` | any | `None` (unknown dial code) |
| `""` | any | `None` |

NANP (`+1`) numbers are returned with US's `iso_alpha2` because both US and CA share the same dial code; the canonical E.164 output is identical for both jurisdictions, which is the property the matcher relies on.

### Email Normalisation

`Normalizer::normalize_email(email, gmail_dot_folding) -> Option<String>` returns the canonical lowercase form of an email address, or `None` when the input is structurally invalid.

Algorithm:

1. Trim surrounding whitespace.
2. Lowercase the entire address (RFC 5321 makes the domain case-insensitive; real-world data overwhelmingly treats the localpart case-insensitively too).
3. Split on `@`. Reject (`None`) unless there is exactly one `@` and both localpart and domain are non-empty.
4. If `gmail_dot_folding` is `true` and the domain is `gmail.com` or `googlemail.com`:
   - Truncate the localpart at the first `+` (drops `+tag` suffix).
   - Remove every `.` from the localpart.
   - Reject if the resulting localpart is empty.

Examples:

| Input | `gmail_dot_folding` | Output |
|---|---|---|
| `"  Alice@Example.ORG  "` | any | `Some("alice@example.org")` |
| `"j.smith@gmail.com"` | `false` | `Some("j.smith@gmail.com")` |
| `"j.smith@gmail.com"` | `true` | `Some("jsmith@gmail.com")` |
| `"jsmith+work@gmail.com"` | `true` | `Some("jsmith@gmail.com")` |
| `"j.smith@example.org"` | `true` | `Some("j.smith@example.org")` (not Gmail; no folding) |
| `"no-at-sign"` | any | `None` |
| `"@example.org"` | any | `None` |
| `"a@b@c"` | any | `None` |
| `""` | any | `None` |

`MatchingEngine::match_persons` calls `normalize_email` on both sides; `MatchBreakdown::email_score` is `Some(1.0)` for equal canonical forms, `Some(0.0)` for distinct canonical forms when both parse, and `None` when either input is absent or fails to parse.

`local_id` is **not** scored. Different organisations may issue colliding values (a person's MRN at site A and another person's MRN at site B can be byte-equal), so positional matching would produce false positives.

### Address Line Normalisation

`Normalizer::normalize_address_line(line)` and `Normalizer::parse_address_line(line)` are the two public entry points for structural address handling.

#### Abbreviation expansion — `expand_street_abbreviations`

Tokenise on whitespace. For each token, strip at most one trailing `.` or `,` and look up the result case-insensitively in the **street-abbreviation table**:

| Abbreviation | Expansion | Abbreviation | Expansion |
|---|---|---|---|
| `st`, `str` | `street` | `n` | `north` |
| `rd` | `road` | `s` | `south` |
| `ave`, `av` | `avenue` | `e` | `east` |
| `blvd`, `bvd` | `boulevard` | `w` | `west` |
| `ln` | `lane` | `ne` | `northeast` |
| `dr` | `drive` | `nw` | `northwest` |
| `ct` | `court` | `se` | `southeast` |
| `pl` | `place` | `sw` | `southwest` |
| `sq` | `square` | | |
| `ter`, `terr` | `terrace` | `hwy` | `highway` |
| `pkwy` | `parkway` | `mt` | `mount` |
| `mtn` | `mountain` | `cres` | `crescent` |
| `gdns` | `gardens` | `gdn` | `garden` |
| `gr` | `grove` | `cl` | `close` |
| `pk` | `park` | `plz` | `plaza` |
| `expy` | `expressway` | `trl` | `trail` |

Matched tokens are replaced with the lower-case long form; unrecognised tokens pass through unchanged. Tokens are re-joined by single spaces. The expansion is **always token-level** and does not apply position-aware heuristics. The well-known ambiguous case `"St"` (Saint vs Street) is always expanded to `street`; the resulting canonical form is consistent on both sides of a comparison.

#### Address-line normalisation — `normalize_address_line`

`expand_street_abbreviations(line) → normalize_name(...)`. Idempotent.

Examples:

| Input | Output |
|---|---|
| `"123 High St"` | `"123 high street"` |
| `"45 N Park Ave"` | `"45 north park avenue"` |
| `"10, DOWNING Street."` | `"10 downing street"` |

#### Address-line parsing — `parse_address_line`

Returns `ParsedAddressLine { house_number: Option<String>, unit: Option<String>, street: String }`.

Algorithm:

1. Trim leading whitespace.
2. **Unit prefix**: read the first whitespace-separated token. Strip at most one trailing `.` or `,`. If the lowercase form matches one of `flat`, `apartment`, `apt`, `unit`, `suite`, `ste`, `room`, `rm`, read the next alphanumeric run as the unit identifier. Store `format!("{keyword} {identifier}")` lowercased.
3. Skip a single leading `,` and any whitespace.
4. **House number**: read the leading run of ASCII digits; if non-empty, also consume a single trailing ASCII alphabetic character (e.g. `"10A"`) **only when not followed by another alphanumeric** (otherwise we would absorb the first letter of the street name, as in `"10 Apple Tree Lane"`). Uppercase the result.
5. Skip a single leading `,` and any whitespace.
6. **Street**: `normalize_address_line` of the remainder.

`ParsedAddressLine` is `Serialize + Deserialize` and re-exported from the crate root.

#### Limitations and pitfalls

- `"St"` (Saint) is always expanded to `street`. The canonical form is consistent on both sides; fuzzy matching tolerates the resulting inconsistency.
- Multi-line addresses are not parsed; consumers must split them upstream.
- The unit prefix dictionary is English-language. Non-English unit terms (`"Wohnung"`, `"Appartement"`) are not recognised and are passed through verbatim into the street field.
- House numbers that include hyphens (`"123-125 High St"`) are partially parsed: the leading number is captured but the range information is dropped into the street remainder.

#### Matcher integration

`MatchingEngine::compare_addresses` calls `Normalizer::parse_address_line` on both `line1` strings and combines the street similarity with the house-number exact-match score. The `city` and `postcode` comparisons are unchanged.

`MatchingEngine::match_persons` consults E.164 first and falls back to the legacy form. Specifically, with phone strings `phone1` and `phone2` and default country `cc = MatchConfig::phone_default_country`:

1. Compute `e1 = normalize_phone_e164(phone1, cc)` and `e2 = normalize_phone_e164(phone2, cc)`.
2. If both are `Some`, score `phone_score = 1.0 if e1 == e2 else 0.0`.
3. Otherwise compare `normalize_phone(phone1) == normalize_phone(phone2)`.

This preserves the prior single-country behaviour for inputs the country table does not cover, while adding cross-country disambiguation for inputs it does.

### Phonetic Code

1. Apply name normalisation.
2. If empty, return empty.
3. Apply `soundex::american_soundex`.

The "American" Soundex is used pragmatically; a locale-aware phonetic algorithm (Double Metaphone, NYSIIS, or similar) is tracked in `AGENTS/roadmap-research.md` (T-9) as a candidate replacement or augmentation.

### National Identifier Normalisation

Each scheme has its own canonical form. Two inputs that represent the same identifier in different textual layouts MUST canonicalise to the same string.

**UK United Kingdom National Health Service Number** (`parse_united_kingdom_national_health_service_number`):
1. Delegated to `united_kingdom_national_health_service_number::NHSNumber::from_str`, which accepts the 10-digit compact form (`"9434765919"`) or the 12-character spaced form (`"943 476 5919"`).
2. Canonical form: 10 digits, no spaces.

**France NIR** (`parse_fr_nir`):
1. Strip all Unicode whitespace.
2. Uppercase letters.
3. Reject unless the result is ASCII and exactly 15 characters.
4. Build a numeric body from positions 0..13: if positions 5..7 are `"2A"` replace with `"19"`; if `"2B"` replace with `"18"`; otherwise require all 13 characters to be digits.
5. Reject unless positions 13..15 are both ASCII digits.
6. Validate `97 - (N mod 97) == key`, where `N` is the numeric body parsed as `u64` and `key` is positions 13..15.
7. Canonical form: the cleaned, uppercased 15-character string.

**España TSI / CIP-SNS** (`parse_es_tsi`):
1. Strip Unicode whitespace and ASCII hyphens (`-`).
2. Uppercase letters.
3. Reject unless the result is ASCII, contains only ASCII alphanumerics, and has length in `10..=20`.
4. Canonical form: the cleaned, uppercased string.

**Éire IHI** (`parse_ie_ihi`):
1. Keep only ASCII digits.
2. Reject unless the result has exactly 7 digits.
3. Canonical form: the 7-digit string.

**UK NI H&C Number** (`parse_uk_hc_number`):
1. Identical algorithm to UK United Kingdom National Health Service Number (`parse_united_kingdom_national_health_service_number`).
2. Exposed as a distinct function so that the calling code retains scheme provenance — a United Kingdom National Health Service Number and an H&C Number with the same 10 digits refer to different persons in different registries and MUST NOT cross-match.

**US SSN** (`parse_us_ssn`):
1. Keep only ASCII digits.
2. Reject unless the result has exactly 9 digits.
3. Reject if the area number (digits 0..3) is `000`, `666`, or in `900..=999`.
4. Reject if the group number (digits 3..5) is `00`.
5. Reject if the serial number (digits 5..9) is `0000`.
6. Canonical form: the 9-digit compact string `"AAAGGSSSS"`.

**Australia IHI** (`parse_au_ihi`):
1. Keep only ASCII digits.
2. Reject unless the result has exactly 16 digits.
3. Apply the Luhn algorithm (ISO/IEC 7812-1) over all 16 digits with weights `2, 1, 2, 1, …` from the left; products `≥ 10` reduced by digit-sum.
4. Canonical form: the 16-digit compact string. The structural convention that real IHIs begin with `800360` is NOT enforced.

**Germany KVNR** (`parse_de_kvnr`):
1. Strip whitespace; uppercase letters.
2. Reject unless the result is ASCII and has exactly 10 characters: one letter followed by 9 digits.
3. Map the leading letter to a 2-digit ordinal (`A=01`, `B=02`, …, `Z=26`); concatenate with positions 2..=9 of the KVNR → 10 digits.
4. Apply alternating weights `1, 2, 1, 2, …, 1, 2`; reduce products `≥ 10` by digit-sum; sum.
5. The check digit (position 10 of the KVNR) MUST equal `sum mod 10`.
6. Canonical form: the 10-character uppercase string.

**Italy *Codice Fiscale*** (`parse_it_cf`):
1. Strip whitespace; uppercase letters.
2. Reject unless the result is ASCII, exactly 16 characters, and entirely alphanumeric.
3. For each of the first 15 characters, look up a numeric value via the standard tables: odd-positioned characters (1-indexed positions 1, 3, …, 15) use the scattered "odd" table; even-positioned characters (2, 4, …, 14) map digits/letters to their natural value.
4. Sum the 15 values; take mod 26.
5. Map `0..=25` to `A..=Z`. The result MUST equal the 16th character.
6. Canonical form: the 16-character uppercase string.

**Netherlands BSN** (`parse_nl_bsn`):
1. Keep only ASCII digits.
2. Reject unless the result has exactly 9 digits.
3. Reject the all-zero string `000000000`.
4. Apply the "11-test": `9·d₁ + 8·d₂ + 7·d₃ + 6·d₄ + 5·d₅ + 4·d₆ + 3·d₇ + 2·d₈ − d₉ ≡ 0 (mod 11)`.
5. Canonical form: the 9-digit compact string.

**Sweden *Personnummer*** (`parse_se_personnummer`):
1. Keep only ASCII digits.
2. Accept exactly 10 or 12 digits; reject anything else.
3. For Luhn validation use the 10-digit form (drop the leading century from a 12-digit input).
4. Apply Luhn with weights `2, 1, 2, 1, …` from the left over the 10 digits; products `≥ 10` reduced by digit-sum; the total mod 10 must be `0`.
5. Canonical form preserves the input length: 10-digit input yields a 10-character string; 12-digit input yields a 12-character string. Records using mixed layouts will not deterministically match on this field.

**UK Scotland CHI Number** (`parse_uk_chi_number`):
1. Keep only ASCII digits.
2. Reject unless the result has exactly 10 digits.
3. Multiply the first 9 digits by weights `10, 9, 8, 7, 6, 5, 4, 3, 2`; sum; take mod 11.
4. The check digit (position 10) MUST equal `(11 − (sum mod 11)) mod 11`. A computed check of `10` indicates an invalid identifier and is rejected.
5. Canonical form: the 10-digit compact string.
6. The CHI Number shares the Mod-11 algorithm with the UK United Kingdom National Health Service Number and UK NI H&C Number but is scheme-local; cross-scheme matching is forbidden.

For the remaining 24 schemes (T-27 / T-28 / T-17.1) the per-parser algorithm is summarised in `spec.md` §6.4 and lives canonically in `src/identifiers.rs` rustdoc. See `AGENTS/national-person-identifiers.md` for the cross-reference table.
