# Normalization — Agent Guide

See [`../spec.md`](../spec.md) §14 for the formal rules. This guide is the operational view.

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

`Normalizer::normalize_phone_e164` consults a table of **39 jurisdictions** that covers every country for which the crate parses a national identifier scheme (T-19, §21.4). That includes the six original identifier jurisdictions (UK, FR, ES, IE, UK NI via the GB dial code, US), the major worker-mobility partners (CA, DE, IT, NL, BE, PT, CH, AT, SE, NO, DK, FI, PL, AU, NZ, JP, CN, IN, BR, MX, ZA), and the 13 jurisdictions added in T-19 (BG, CZ, EE, GR, HR, IS, LI, LT, LV, MT, RO, SI, SK). Each entry records:

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

This means cross-country deployments should set `phone_default_country` to the worker population's predominant jurisdiction (or `None` to refuse to guess). The fallback path preserves behaviour for inputs the country table does not cover.

## What We Do Not Normalise (Yet)

- **`local_id`** — intentionally not normalised AND not scored. Different practices may issue colliding IDs.

(Note: `middle_name` is now scored as part of the given-name component — see spec FR-49 / §12.2. It is normalised via the same `normalize_name` pipeline as the given and family names.)

## Adding a New Normaliser

1. Add the new public method on `Normalizer` in `src/normalizer.rs`.
2. Add unit tests covering: empty input, all-whitespace input, a realistic happy-path input, and a diacritic case if the field can contain Unicode names.
3. Update [`../spec.md`](../spec.md) §14 with the algorithm steps and a table of examples.
4. If a scoring path uses the new normaliser, document it in spec §12.2.

## Pitfalls

- ❌ Don't collapse double-barrelled surnames (`Lloyd-Webber`) into a single word without thinking — current `normalize_name` drops the hyphen, yielding `lloydwebber`. This is intentional but worth knowing.
- ❌ Don't apply name normalisation to NHS numbers — the `nhs-number` crate has its own parser. Calling `normalize_name` on a digit string would happen to work but couples concerns.
- ❌ Don't lowercase postcodes; the canonical form is uppercase.
- ❌ Don't trust that `Char::is_ascii_punctuation` covers Unicode punctuation. It does not — `’` (curly apostrophe, U+2019) would survive. If you need broader stripping, propose it via the spec.
- ❌ Don't compare `normalize_phone` output across countries — it's UK-centric and will silently collapse French / Italian national numbers to lookalike digit strings. Use `normalize_phone_e164` (or rely on the matcher, which already does) for multi-country data.
- ❌ Don't add a new country to `COUNTRY_PHONE_TABLE` without explicit trunk-prefix and NSN-range provenance. Guessing the range produces false-negative matches when legitimate numbers fall outside it.
- ❌ Don't extend the address parser to apply position-aware heuristics for `"St"` (Saint vs Street) without a corresponding spec update. The current rule "always expand to Street" is deliberately simple; introducing context-sensitivity risks regressions.
- ❌ Don't add house-number ranges to the `house_number` field (`"123-125 High St"`). The current parser captures only the leading number; widening this without thought changes the equality semantics of the matcher's line-1 sub-score.

## Idempotence

All normalisers SHOULD be idempotent: `f(f(x)) == f(x)`. If you add one that isn't, document the reason in `spec.md` §14 and add a property test.
