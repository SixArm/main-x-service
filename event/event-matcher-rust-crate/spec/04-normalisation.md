## 4. Normalisation

All normalisers in `Normalizer` are stateless associated functions and **idempotent**: `f(f(x)) == f(x)`. They are also **deterministic** (no clocks, no RNGs) and allocate at most a single new `String`.

> Per-rule example tables are maintained in [`agents/normalization.md`](../agents/normalization.md); this section pins the algorithms.

### 4.1 Names — `Normalizer::normalize_name`

Pipeline (in order): NFKD-decompose (`é` → `e` + combining acute); drop combining marks (diacritics); drop ASCII punctuation; lowercase; collapse consecutive whitespace to single ASCII spaces and trim. Example: `"O'Brien"` → `"obrien"`; `"José"` → `"jose"`; `"Łukasz"` → `"łukasz"` (`ł` has no NFKD decomposition).

**Known limit:** non-ASCII punctuation (e.g. the curly apostrophe `’` U+2019) is **not** stripped. Upstream code should convert to ASCII first.

### 4.2 Postcodes — `Normalizer::normalize_postcode`

Drop all whitespace; uppercase. No locale-specific validation. `"cf10 1aa"` → `"CF101AA"`.

### 4.3 Phone numbers

Two normalisers are provided.

#### 4.3.1 Legacy — `Normalizer::normalize_phone`

Keep ASCII digits only; strip the international or trunk prefix:

1. If the result starts with `0044`, drop those four characters.
2. Else, if the result starts with `44` and is at least 12 digits long, drop the leading `44`.
3. Else, if the result starts with `0` and is longer than one digit, drop the leading `0`.

UK-centric and infallible. Used as a fallback when E.164 normalisation fails. International numbers from other countries pass through unchanged.

#### 4.3.2 E.164 — `Normalizer::normalize_phone_e164`

```rust
fn normalize_phone_e164(phone: &str, default_country: Option<&str>) -> Option<String>;
```

Match `+CC` / `00CC` / default-country trunk; strip the national trunk prefix; validate the national-significant-number (NSN) length against a per-country range; return `+CCNNN…`. Returns `None` for empty input, unparseable inputs, or unsupported countries.

`default_country` is an ISO 3166-1 alpha-2 code (`"GB"`, `"FR"`, `"US"`, …); pass `None` to refuse to assume a default — only explicit `+CC` / `00CC` inputs will parse.

Supported countries (per the in-code `COUNTRY_PHONE_TABLE`, **39 entries**): `GB`, `FR`, `DE`, `ES`, `IE`, `IT`, `NL`, `BE`, `PT`, `CH`, `AT`, `SE`, `NO`, `DK`, `FI`, `PL`, `AU`, `NZ`, `US`, `CA`, `JP`, `CN`, `IN`, `BR`, `MX`, `ZA`, `BG`, `CZ`, `EE`, `GR`, `HR`, `IS`, `LI`, `LT`, `LV`, `MT`, `RO`, `SI`, `SK`. Each entry pins the dial code, the national trunk prefix (`"0"` for most of Europe and Asia; `"8"` for Lithuania; `None` for NANP / Spain / Portugal / several others), and the min / max NSN length. Note: `Event` carries no phone field, so the matching engine does not consult either phone normaliser; both remain public library utilities for callers.

### 4.4 Email — `Normalizer::normalize_email`

```rust
fn normalize_email(email: &str, gmail_dot_folding: bool) -> Option<String>;
```

1. Trim surrounding whitespace.
2. Lowercase the whole address.
3. Reject inputs without exactly one `@`, or with an empty localpart or domain (return `None`).
4. If `gmail_dot_folding` is `true` AND the domain is `gmail.com` or `googlemail.com`, strip every `.` from the localpart and drop any `+tag` suffix.

Examples: `"  Alice@Example.ORG  "` → `Some("alice@example.org")`; `"j.smith+work@gmail.com"` with folding → `Some("jsmith@gmail.com")`; non-Gmail domains never fold; `"no-at-sign"` and `"a@b@c"` return `None`. See [`agents/normalization.md`](../agents/normalization.md) for the full table.

### 4.5 Address-line parsing — `ParsedAddressLine`, `Normalizer::parse_address_line`

```rust
pub struct ParsedAddressLine {
    pub house_number: Option<String>,
    pub unit: Option<String>,
    pub street: String,
}
```

Best-effort structural decomposition:

1. **Unit prefix.** First token matched against `{flat, apartment, apt, unit, suite, ste, room, rm}`. If recognised, the next alphanumeric run is the identifier; result is `format!("{keyword} {identifier}")` lowercased.
2. **House number.** Leading run of ASCII digits, optionally followed by a single alphabetic suffix (`"10A"`). The suffix is taken **only when not followed by another alphanumeric**, so `"10 Apple Tree Lane"` does not absorb the `A` of `Apple`. Uppercased.
3. **Street.** The remainder, run through `Normalizer::normalize_address_line` (= `expand_street_abbreviations` + `normalize_name`).

Parsing is **format-only**: no postal reference is consulted. Inputs that do not match the regular structure (e.g. a postcode-only string, a city name) degrade gracefully: `house_number` and `unit` are `None`, and `street` carries the normalised input.

#### 4.5.1 `expand_street_abbreviations`

Whole-token expansion of English abbreviations: `St` → `street`, `Rd` → `road`, `Ave` → `avenue`, `Blvd` → `boulevard`, `N` → `north`, etc. The ambiguous `"St"` (Street vs Saint) is **always** expanded to `street`; pre-process upstream if you need finer disambiguation (see OQ-D).

### 4.6 Phonetic code — `Normalizer::phonetic_code`

Normalise the name (§4.1), then apply American Soundex (`soundex::american_soundex`). Empty input returns `""` (not a default Soundex value).

Soundex is tuned for English-language names; non-English phonemes may be lost. A locale-aware encoder (Double Metaphone, Daitch-Mokotoff) is tracked as OQ-E.

### 4.7 ISO 8601 date-times — `Normalizer::parse_iso8601_unix_seconds`

```rust
fn parse_iso8601_unix_seconds(input: &str) -> Option<i64>;
```

Total, dependency-free parser from an ISO 8601 / RFC 3339 date or date-time string to Unix seconds. Accepted shapes: `YYYY-MM-DD` (anchors at midnight UTC), `YYYY-MM-DDTHH:MM:SS` with optional fractional seconds (truncated), and a trailing `Z` or `±HH:MM` / `±HHMM` / `±HH` offset; the `T` separator may also be lowercase `t` or a single space. Returns `None` for malformed input or out-of-range components (month not in `1..=12`, day beyond the calendar month — leap years honoured, hour not in `0..=23`, minute not in `0..=59`, second not in `0..=60` — leap seconds permitted).

Deterministic and idempotent under canonicalisation: distinct textual layouts denoting the same instant (e.g. `2024-06-26T09:00:00Z` and `2024-06-26T11:00:00+02:00`) MUST return the same number. Consumed by `Scorer::seconds_between` (§6.3) and by deterministic rule 2 (§5.1).

---

