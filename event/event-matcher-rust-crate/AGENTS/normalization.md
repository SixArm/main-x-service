# Normalisation — agent guide

See [`../spec.md`](../spec/index.md) §4 for the formal rules. This guide is the operational view, with extra examples per rule.

## Why normalise at all?

The research base is unanimous: most accuracy gains come from data standardisation, not from cleverer scoring. Garbage in, garbage out — no Jaro-Winkler trick rescues an apostrophe mismatch or a casing difference in a postcode.

## What we normalise

| Input | Algorithm | Source | Spec |
|---|---|---|---|
| Names | NFKD + drop combining marks + drop ASCII punctuation + lowercase + collapse whitespace | `src/normalizer.rs::normalize_name` | §4.1 |
| Postcodes | Strip whitespace + uppercase | `src/normalizer.rs::normalize_postcode` | §4.2 |
| Phone numbers (legacy) | Keep digits + strip `0044` / `+44` / leading `0` (UK-centric) | `src/normalizer.rs::normalize_phone` | §4.3.1 |
| Phone numbers (E.164) | Match `+CC` / `00CC` / default-country trunk, strip national trunk prefix, validate NSN length, return `+CCNNN…` | `src/normalizer.rs::normalize_phone_e164` | §4.3.2 |
| Address-line abbreviations | Whole-token expansion of `St` → `Street`, `Rd` → `Road`, `Ave` → `Avenue`, `N` → `North`, … | `src/normalizer.rs::expand_street_abbreviations` | §4.5.1 |
| Address line (full) | Abbreviation expansion + name-normalisation pipeline | `src/normalizer.rs::normalize_address_line` | §4.5 |
| Address-line parsing | Extract `house_number` (leading digits + optional alphabetic suffix), `unit` (Flat / Apt / Suite / …), `street` (normalised remainder) | `src/normalizer.rs::parse_address_line` | §4.5 |
| Email | Trim + lowercase + reject if missing `@` / empty localpart / empty domain; opt-in Gmail dot- and `+tag`-folding | `src/normalizer.rs::normalize_email` | §4.4 |
| Phonetic | Name normalisation then American Soundex | `src/normalizer.rs::phonetic_code` | §4.6 |

All normalisers are **idempotent**: `f(f(x)) == f(x)`. If you add one that isn't, document the reason in `spec.md` and add a property test.

## Input → output tables

### Names

| Input | Output |
|---|---|
| `"Eiffel Tower"` | `"eiffel tower"` |
| `"  John  Smith  "` | `"john smith"` |
| `"O'Brien"` | `"obrien"` |
| `"MARY-JANE"` | `"maryjane"` |
| `"José"` | `"jose"` |
| `"Siân"` | `"sian"` |
| `"Łukasz"` | `"łukasz"` (`ł` has no NFKD decomposition) |
| `""` / `"   "` | `""` |

### Postcodes

| Input | Output |
|---|---|
| `"CF10 1AA"` | `"CF101AA"` |
| `"cf101aa"` | `"CF101AA"` |
| `"  sw1a 2aa "` | `"SW1A2AA"` |
| `""` | `""` |

### Phone numbers (legacy)

| Input | Output |
|---|---|
| `"07700 900123"` | `"7700900123"` |
| `"+44 7700 900123"` | `"7700900123"` |
| `"0044 7700 900123"` | `"7700900123"` |
| `"(029) 2034 5678"` | `"2920345678"` |

### Phone numbers (E.164)

| Input | `default_country` | Output |
|---|---|---|
| `"+44 7700 900123"` | any | `Some("+447700900123")` |
| `"07700 900123"` | `Some("GB")` | `Some("+447700900123")` |
| `"01 23 45 67 89"` | `Some("FR")` | `Some("+33123456789")` |
| `"(415) 555-1234"` | `Some("US")` | `Some("+14155551234")` |
| `"07700 900123"` | `None` | `None` (no default and no international marker) |
| `""` | any | `None` |

### Email

| Input | `gmail_dot_folding` | Output |
|---|---|---|
| `"  Alice@Example.ORG  "` | `false` | `Some("alice@example.org")` |
| `"j.smith@gmail.com"` | `true` | `Some("jsmith@gmail.com")` |
| `"jsmith+work@googlemail.com"` | `true` | `Some("jsmith@googlemail.com")` |
| `"j.smith@example.org"` | `true` | `Some("j.smith@example.org")` (non-Gmail domain unaffected) |
| `"no-at-sign"` | any | `None` |
| `"a@b@c"` | any | `None` |

## International phone numbers

`Normalizer::normalize_phone_e164` consults `COUNTRY_PHONE_TABLE` in `src/normalizer.rs`. Each entry records:

- ISO 3166-1 alpha-2 country code (`GB`, `FR`, `US`, …).
- Dial code (1–3 digits, no leading `+`).
- National trunk prefix (`Option<&'static str>` — typically `Some("0")`, but Lithuania uses `Some("8")` and several countries use `None`).
- NSN length range — minimum and maximum digits in the national-significant number, used to reject malformed inputs.

Supported jurisdictions are enumerated in `spec.md` §4.3.2.

When adding a new country:

1. Add an entry to `COUNTRY_PHONE_TABLE`, citing the ITU-T E.164 country code and the national numbering plan's NSN range.
2. Extend `spec.md` §4.3.2 supported-country list.
3. Add a unit test in `src/normalizer.rs::tests` for the within-country happy path and at least one integration test in `tests/integration_tests.rs` if matcher behaviour changes.

## Address parsing

`Normalizer::parse_address_line` is a best-effort structural decomposition:

```text
"Flat 2A, 10 Downing Street"  →  ParsedAddressLine {
    unit:         Some("flat 2a"),
    house_number: Some("10"),
    street:       "downing street",
}
```

Stages (`spec.md` §4.5):

1. **Unit prefix.** First token matched against `{flat, apartment, apt, unit, suite, ste, room, rm}`. If recognised, the next alphanumeric run is the identifier; result is `format!("{keyword} {identifier}")` lowercased.
2. **House number.** Leading run of ASCII digits, optionally followed by a single alphabetic suffix (`"10A"`). The suffix is taken **only when not followed by another alphanumeric**, so `"10 Apple Tree Lane"` does not absorb the `A` of `Apple`.
3. **Street.** Remainder, run through `normalize_address_line` (= `expand_street_abbreviations` + `normalize_name`).

The matcher's line-1 sub-score combines a Jaro-Winkler similarity on `parsed.street` with an exact-match score on `parsed.house_number`: `0.6 × street + 0.4 × house number` when both sides have a house number; street similarity alone otherwise (`spec.md` §6.4).

### Adding a new street abbreviation

1. Add an entry to `STREET_ABBREVIATIONS` in `src/normalizer.rs`. Keep the long form lowercase so it survives the downstream name pipeline unchanged.
2. Extend `spec.md` §4.5.1.
3. Add a unit test covering the new abbreviation and at least one combined-pipeline assertion.

### Adding a new unit prefix

1. Add the keyword to `UNIT_PREFIXES` in `src/normalizer.rs`.
2. Extend `spec.md` §4.5.
3. Add a unit test asserting `parse_address_line` returns the keyword + identifier.

## Email normalisation

`Normalizer::normalize_email(email, gmail_dot_folding)` (`spec.md` §4.4):

1. Trim surrounding whitespace.
2. Lowercase the whole string.
3. Reject inputs with anything other than exactly one `@`, an empty localpart, or an empty domain (return `None`).
4. If `gmail_dot_folding` is `true` and the domain is `gmail.com` or `googlemail.com`, strip every `.` from the localpart and drop any `+tag` suffix.

The matcher emits `Some(1.0)` / `Some(0.0)` for normalised equality, `None` when either side is missing or fails to canonicalise (`spec.md` §6.9).

## Phonetic codes

`Normalizer::phonetic_code(name)` runs the name through `normalize_name` and then American Soundex. The matcher only consults phonetic codes when `MatchConfig::use_phonetic_matching` is on. The phonetic bonus is bounded (`0.05`-weighted, only fires when the gate `> 0.9`) and never lowers the overall score (`spec.md` §6.2).

American Soundex is tuned for English-language names and may lose information on non-English digraphs. A locale-aware encoder (Double Metaphone, Daitch-Mokotoff) is tracked as OQ-E.

## Matcher wiring (phone path)

`MatchingEngine::score_phone` prefers the E.164 form and falls back to the legacy national-significant form (`spec.md` §6.8):

1. Compute `normalize_phone_e164(phone1, cc)` and `normalize_phone_e164(phone2, cc)` where `cc = MatchConfig::phone_default_country` (default `Some("GB")`).
2. If both parse, `phone_score = 1.0` iff the canonical strings are equal, else `0.0`.
3. Otherwise compare `normalize_phone(phone1) == normalize_phone(phone2)`.

Cross-country deployments should set `phone_default_country` to the predominant jurisdiction (or `None` to refuse to guess). The fallback preserves behaviour for inputs the country table does not cover.

## What we do not normalise

- **`local_id`** — intentionally not normalised AND not scored. Different sources may issue colliding values.
- **`country_code_as_iso_3166_1_alpha_2`** — stored as supplied (`"GB"`, `"gb"`, etc.); the matcher compares case-insensitively after trim but does NOT rewrite the stored value. See OQ-B.
- **`PlaceId::value`** — trimmed at construction, otherwise compared verbatim. Different schemes have different canonical forms; the crate makes no per-scheme assumptions.

## Adding a new normaliser

1. Add the new public method on `Normalizer` in `src/normalizer.rs`.
2. Add unit tests covering: empty input, all-whitespace input, a realistic happy-path input, and a diacritic case if the field can contain Unicode names.
3. Update [`../spec.md`](../spec/index.md) §4 with the algorithm steps and a table of examples.
4. If a scoring path uses the new normaliser, document it in `spec.md` §6.

## Pitfalls

- Don't collapse double-barrelled names (`Lloyd-Webber`) into a single word without thinking — current `normalize_name` drops the hyphen, yielding `lloydwebber`. This is intentional but worth knowing.
- Don't lowercase postcodes; the canonical form is uppercase.
- Don't trust that `char::is_ascii_punctuation` covers Unicode punctuation. It does not — `’` (curly apostrophe, U+2019) would survive. If you need broader stripping, propose it via the spec.
- Don't compare `normalize_phone` output across countries — it's UK-centric and will silently collapse French / Italian national numbers to lookalike digit strings. Use `normalize_phone_e164` (or rely on the matcher, which already does) for multi-country data.
- Don't add a new country to `COUNTRY_PHONE_TABLE` without explicit trunk-prefix and NSN-range provenance. Guessing the range produces false-negative matches when legitimate numbers fall outside it.
- Don't extend the address parser to apply position-aware heuristics for `"St"` (Saint vs Street) without a corresponding spec update (see OQ-D).
- Don't add house-number ranges to the `house_number` field (`"123-125 High St"`). The current parser captures only the leading number; widening this changes the equality semantics of the matcher's line-1 sub-score.
