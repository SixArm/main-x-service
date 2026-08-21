## 4. Normalisation

All normalisers in `Normalizer` are stateless associated functions and **idempotent** (`f(f(x)) == f(x)`), **deterministic** (no clocks, no RNGs), and allocate at most a single new `String`.

Operational examples per rule, plus full input/output tables, live in [`agents/normalization.md`](../agents/normalization.md). The behaviour-defining contracts are summarised here.

### 4.1 Names — `Normalizer::normalize_name`

Pipeline: NFKD-decompose, drop combining marks, drop ASCII punctuation, lowercase, collapse consecutive whitespace to single ASCII spaces, trim ends.

**Known limit:** non-ASCII punctuation (e.g. the curly apostrophe `’` U+2019) is **not** stripped. Upstream code should convert to ASCII first. Characters with no NFKD decomposition (e.g. `ł`) survive lowercased.

### 4.2 Postcodes — `Normalizer::normalize_postcode`

Drop all whitespace; uppercase. No locale-specific validation. `"cf10 1aa"` → `"CF101AA"`.

### 4.3 Phone numbers

Two normalisers are provided.

#### 4.3.1 Legacy — `Normalizer::normalize_phone`

Keep ASCII digits only; then strip prefixes in this order: leading `0044` (drop 4 chars); else leading `44` when result is ≥12 digits (drop 2 chars); else leading `0` when result is >1 digit (drop 1 char). UK-centric and infallible. Used as the fallback when E.164 normalisation fails. International numbers from other countries pass through unchanged.

#### 4.3.2 E.164 — `Normalizer::normalize_phone_e164`

```rust
fn normalize_phone_e164(phone: &str, default_country: Option<&str>) -> Option<String>;
```

Match `+CC` / `00CC` / default-country trunk; strip the national trunk prefix; validate the national-significant-number (NSN) length against a per-country range; return `+CCNNN…`. Returns `None` for empty input, unparseable inputs, or unsupported countries. `default_country` is an ISO 3166-1 alpha-2 code (`"GB"`, `"FR"`, `"US"`, …); pass `None` to refuse to assume a default.

Supported countries (per the in-code `COUNTRY_PHONE_TABLE`): `GB`, `FR`, `DE`, `ES`, `IE`, `IT`, `NL`, `BE`, `PT`, `CH`, `AT`, `SE`, `NO`, `DK`, `FI`, `PL`, `AU`, `NZ`, `US`, `CA`, `JP`, `CN`, `IN`, `BR`, `MX`, `ZA`, `BG`, `CZ`, `EE`, `GR`, `HR`, `IS`, `LI`, `LT`, `LV`, `MT`, `RO`, `SI`, `SK`. Each entry pins the dial code, the national trunk prefix (`"0"` for most of Europe and Asia; `"8"` for Lithuania; `None` for NANP / Spain / Portugal / several others), and the min / max NSN length.

The matching engine MUST prefer the E.164 form when both inputs canonicalise, and MUST fall back to the legacy form otherwise (§6.8).

### 4.4 Email — `Normalizer::normalize_email`

```rust
fn normalize_email(email: &str, gmail_dot_folding: bool) -> Option<String>;
```

Trim surrounding whitespace; lowercase the whole address; reject inputs without exactly one `@` or with an empty localpart or domain (return `None`). When `gmail_dot_folding` is `true` AND the domain is `gmail.com` or `googlemail.com`, strip every `.` from the localpart and drop any `+tag` suffix. Non-Gmail domains are unaffected by the folding flag.

### 4.5 Address-line parsing — `ParsedAddressLine`, `Normalizer::parse_address_line`

```rust
pub struct ParsedAddressLine {
    pub house_number: Option<String>,
    pub unit: Option<String>,
    pub street: String,
}
```

Best-effort structural decomposition (format-only — no postal reference is consulted):

1. **Unit prefix.** First token matched against `{flat, apartment, apt, unit, suite, ste, room, rm}`. If recognised, the next alphanumeric run is the identifier; result is `format!("{keyword} {identifier}")` lowercased.
2. **House number.** Leading run of ASCII digits, optionally followed by a single alphabetic suffix (`"10A"`). The suffix is taken **only when not followed by another alphanumeric**, so `"10 Apple Tree Lane"` does not absorb the `A` of `Apple`. Uppercased.
3. **Street.** The remainder, run through `Normalizer::normalize_address_line` (= `expand_street_abbreviations` + `normalize_name`).

Inputs that do not match the regular structure (e.g. a postcode-only string, a city name) degrade gracefully: `house_number` and `unit` are `None`, and `street` carries the normalised input.

#### 4.5.1 `expand_street_abbreviations`

Whole-token expansion of English abbreviations: `St` → `street`, `Rd` → `road`, `Ave` → `avenue`, `Blvd` → `boulevard`, `N` → `north`, etc. The ambiguous `"St"` (Street vs Saint) is **always** expanded to `street`; pre-process upstream if you need finer disambiguation (see OQ-D).

### 4.6 Phonetic code — `Normalizer::phonetic_code`

Normalise the name (§4.1), then apply American Soundex (`soundex::american_soundex`). Empty input returns `""` (not a default Soundex value). Soundex is tuned for English-language names; non-English phonemes may be lost. A locale-aware encoder (Double Metaphone, Daitch-Mokotoff) is tracked as OQ-E.

---

