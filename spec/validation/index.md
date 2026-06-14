# Data quality &amp; validation

Monorepo-wide reference for **data-quality validation, normalization,
and standardization** across the Main X Index service crates. Validation
is the boundary discipline that keeps malformed, out-of-range, or
internally-inconsistent records out of storage — and out of the matcher,
where a malformed deterministic identifier can poison scoring.

There is no shared validation crate. Each service owns its own
validation module; they share a *contract* (this document) rather than
code. The reference implementations are:

| Service | Module | Entry point |
| ------- | ------ | ----------- |
| person | [`person-service-rust-crate/src/validation/mod.rs`](../../person/person-service-rust-crate/src/validation/mod.rs) | `validate_person(&Person) -> Vec<ValidationError>` |
| place | [`place-service-rust-crate/src/validation/mod.rs`](../../place/place-service-rust-crate/src/validation/mod.rs) | `validate_place(&Place) -> Vec<ValidationError>` |
| care-pathway | [`care-pathway-service-rust-crate/src/validation.rs`](../../care-pathway/care-pathway-service-rust-crate/src/validation.rs) | `problems(&CarePathway) -> Vec<String>` |
| case | [`case-service-rust-crate/src/validation.rs`](../../case/case-service-rust-crate/src/validation.rs) | `problems(&Case) -> Vec<String>` |
| case-folder | [`case-folder-service-with-rust/src/nhs.rs`](../../case-folder/case-folder-service-with-rust/src/nhs.rs) | `is_valid_nhs_number(&str) -> bool` |

Related sibling topics: [REST conventions](../restful/index.md) ·
[matching](../matching/index.md) · [PostgreSQL](../postgresql/index.md).

---

## 1. The contract: collect-all, return `422`

Validation runs **at the request boundary**, inside the create and
update controllers, *before* the record is persisted or matched. Two
rules define the contract:

1. **Collect, do not short-circuit.** Each validator accumulates *every*
   rule violation into a list (a `Vec<ValidationError>` for person/place,
   a `Vec<String>` of human-readable problems for care-pathway/case)
   rather than returning on the first failure. The controller joins the
   list into one response, so an operator fixes all problems in a single
   round-trip instead of one-error-at-a-time. An empty list means valid.

2. **`422 Unprocessable Entity`, not `400`.** A non-empty problem list
   becomes a single `422`. The distinction is deliberate and matches the
   family convention in [REST conventions §status codes](../restful/index.md):

   | Code | When |
   | ---- | ---- |
   | `400 Bad Request` | The request is *malformed*: a body that does not deserialize, an invalid-UUID `pid`, or a **blank/missing `q` on `/search`** (an empty search would match everything). |
   | `422 Unprocessable Entity` | The body deserializes fine but is *semantically* invalid: a blank required field, an out-of-range coordinate, a bad check digit, `main_pid == duplicate_pid` on merge. |

   So a blank search query is a `400` (malformed request); a blank
   `name`/`title` in a create body is a `422` (semantic validation).

Per-field problems are tagged with the **offending field path**, indexed
where the field is a list — e.g. `geo.latitude`, `telecom[0].value`,
`condition_codes[2]`, `opening_hours[1].closes`, `identifiers[1]`. This
lets a front-end attach each message to the right input.

All validators are **pure** (no I/O), so they unit-test cheaply and are
reusable outside the HTTP layer. Each reference module carries a thorough
`#[cfg(test)]` suite pinning both acceptances and rejections.

---

## 2. Required-field enforcement

The minimum each entity must carry to be storable:

| Entity | Required field(s) | Rule |
| ------ | ----------------- | ---- |
| person | `name.family`, `name.given` | family name non-blank; at least one non-blank given name |
| place | `name` | non-blank after trim |
| care-pathway | `name` | non-blank after trim |
| case | `title` | non-blank after trim |

Nested required fields are enforced recursively: a person
`emergency_contacts[i]` requires a non-blank `name` and `relationship`;
an identity `documents[i]` requires a non-blank `number`; case/care-pathway
list entries (`identifiers[i].value`, `subjects[i]`, `keywords[i]`) must
each be non-blank — a blank deterministic identifier is rejected because
the matcher silently drops it, making it a latent data-quality defect.

---

## 3. Format &amp; checksum validators

The validators are intentionally **structural / checksum** checks — they
reject obviously-wrong input without consulting an external authority
(see §7 on what that defers). The interesting algorithms:

### GLN — GS1 mod-10 check digit (place)

A Global Location Number is exactly **13 ASCII digits** whose last digit
is the GS1 check digit. The 12 data digits are weighted right-to-left by
alternating `3, 1, 3, 1, …` (rightmost data digit ×3); the check digit
is whatever rounds the weighted sum up to the next multiple of 10:
`check = (10 - (sum % 10)) % 10`. Implemented in `gln_is_valid`; e.g.
`0614141999996` passes, `0614141999990` fails.

### NHS number — Modulus-11 (case-folder)

A UK NHS Number is 10 digits, formatted `XXX XXX XXXX`. Input is first
**normalized to bare digits** (spaces/hyphens stripped), so grouped and
punctuated forms validate identically. The first 9 digits are weighted
by descending `10, 9, …, 2` (weight for position `i` is `10 - i`); take
`sum % 11` as the remainder, then `check = 11 - remainder`:

- `check == 11` → check digit is `0`.
- **`check == 10` → the number is invalid** (no single digit can
  represent 10); short-circuit to `false` for *every* possible tenth
  digit.
- otherwise `check` is the check digit itself.

The number is valid iff that check digit equals the printed tenth digit.
Implemented in `is_valid_nhs_number` (with `normalise_nhs_number` /
`format_nhs_number` helpers).

### SNOMED CT — SCTID Verhoeff check digit (care-pathway)

A SNOMED CT identifier (SCTID) is **6–18 digits** whose final digit is a
[Verhoeff] dihedral-group (D5) check digit. The check is verified exactly
using the D5 multiplication table and the per-position permutation table,
processing digits right-to-left. Flipping any digit breaks the checksum
(e.g. `22298006` is valid, `22298007` is not). Implemented in
`is_valid_snomed` + `verhoeff_valid`.

### ICD-10 / ICD-11 structural patterns (care-pathway)

Clinical `condition_codes` are checked structurally per coding system,
after splitting off an optional post-`.` extension:

- **ICD-10**: `[A-Z] [0-9] [0-9A-Z]` stem, then an optional `.` plus 1–4
  alphanumerics — e.g. `I63`, `I63.9`, `C7A`, `S72.001A`.
- **ICD-11** (MMS stem): 2–7 alphanumerics whose **second character is
  always a letter** (the defining ICD-11 trait), excluding the letters
  `O` and `I` (ICD-11 omits them to avoid `0`/`1` confusion), then an
  optional `.` extension — e.g. `1A00`, `BA00`, `8B20.0`.
- **Custom** coding system: only required to be non-blank; no format
  imposed.

### UUID and DOI identifier shapes (care-pathway)

Deterministic-scheme `identifiers` drive the matcher's short-circuit, so
their shapes are checked:

- **UUID**: canonical **8-4-4-4-12** hex (36 chars, hyphens at indices
  8/13/18/23, hex elsewhere), case-insensitive, any version/variant.
  `urn:`/brace wrappers are intentionally rejected — store the bare UUID.
- **DOI**: the registrant prefix `10.`, then a non-empty dot-separated
  numeric registrant code, a `/`, and a non-empty suffix —
  e.g. `10.1000/xyz123`. The bare URL form (`https://doi.org/10…`) is
  intentionally not accepted; store the DOI.
- **Open-value-space schemes** (`Wikidata`, `GuidelineId`, `Uri`,
  `PathwayCode`, `LocalId`, `Custom`): only required to be non-blank.

### BCP-47 language tag syntax (care-pathway `in_language`)

Each `in_language` entry must be a **syntactically** valid BCP-47 tag: a
primary subtag of 2–3 or 5–8 ASCII letters (length 4 is reserved for
script subtags and is not a valid primary), then zero or more `-`-separated
subtags of 1–8 alphanumerics — covering `en`, `en-GB`, `zh-Hans`,
`de-DE-1996`. `en_GB` (wrong separator) and `en-` (empty subtag) fail.
Existence in the IANA registry is **not** checked (see §7).

### Opening hours — 24-hour `HH:MM` (place)

Schema.org stores opening hours as plain strings, so without a check any
text would be accepted. `time_is_valid` requires the canonical `HH:MM`
shape: exactly five chars, `:` at index 2, ASCII digits elsewhere, hours
`00..=23`, minutes `00..=59`. Rejects `24:00`, `12:60`, `9:00`
(not zero-padded), `0900` (no separator), `9am`, `+9:00`, `""`.

### Coordinate bounds (place)

When geo is present, WGS-84 ranges are enforced: latitude in `[-90, 90]`,
longitude in `[-180, 180]`. Boundary values (90, 180) are valid;
`geo.latitude = 91` is flagged.

### ISO-8601 dates

- **case `opened_date`**: when present, must be ISO-8601 `YYYY` (bare
  year) or `YYYY-MM-DD` (a *real* calendar date — month `1..=12`, day
  within the month's count, Gregorian leap-year rules; so `2024-13-99`
  and `2024-02-30` are rejected, `2024-02-29` is accepted).
  Implemented in `is_valid_iso_date` / `is_valid_ymd`.
- **person `birth_date`**: when present, must **not be in the future**
  (compared against `chrono` UTC today).

---

## 4. Normalization &amp; standardization at the boundary

Normalization canonicalizes free-text *before* storage so equal-but-
differently-typed inputs compare and store consistently. It runs
alongside validation at the boundary; it mutates/produces a canonical
form rather than reporting problems.

| Helper | Service | Behaviour |
| ------ | ------- | --------- |
| `normalize_phone(phone, cc)` | person | Strips non-digits, produces an **E.164-like** `+<digits>`; prepends the default country code when a bare 10-digit number lacks one; empty input → empty string. |
| `standardize_address(&Address)` | person | Title-cases the city, uppercases state/country, **expands street-type abbreviations** on line 1 (`St.`→`Street`, `Ave.`→`Avenue`, `Rd.`→`Road`, `Dr.`→`Drive`, `Blvd.`→`Boulevard`, …), trims throughout. |
| `normalize_place(&mut Place)` | place | Trims the name; title-cases `address_locality`; uppercases `address_region` / `address_country`. Idempotent. |

Normalization is idempotent — running it twice yields the same result.

---

## 5. Email / phone / address completeness

Boundary format checks on contact data:

- **Email** (person `telecom` where system is `Email`): must contain
  both `@` and `.`. (Structural, not a full RFC parse.)
- **Phone / SMS / fax** (person `telecom`): the value must carry at least
  **7 digits**. Place `telephone`, when non-empty, must start with `+`
  (international format).
- **URL** (place `url`): must start with `http://` or `https://` (scheme
  check only; the full URL is not parsed).
- **Address completeness**: a present address must carry at least one
  *locating* field — **locality (city), postal code, or country**. A lone
  street line is not enough to place a record. Enforced in both person
  (`addresses[i]`) and place (`address`).

---

## 6. Document validation (person)

Where identity documents are modeled (person), each `documents[i]` is
checked for:

- a **non-empty `number`**;
- **not already expired** — `expiry_date`, when present, must not be in
  the past (`chrono` UTC today);
- **issue before expiry** — when both dates are present, `issue_date`
  must not be after `expiry_date`.

---

## 7. What is deferred — syntax, not existence

The validators check **shape and checksum only**. They deliberately do
**not** verify that a code or tag *exists* in a published authority:

- **Terminology-server existence checks.** An ICD-10/11 or SNOMED CT code
  may be syntactically valid (and SNOMED's Verhoeff digit may check out)
  yet not correspond to a concept in any released edition. Verifying
  release membership needs a terminology server and is out of scope.
- **IANA language-subtag-registry existence.** A BCP-47 tag is checked
  for *syntax* only; whether the subtag is registered with IANA is not
  verified.
- **DOI / UUID resolution.** A DOI is checked for the `10.<registrant>/<suffix>`
  shape, not resolved against the DOI system; a UUID is checked for the
  canonical layout, not for uniqueness or version semantics.

These existence checks are tracked as deferred items in the respective
service specs (e.g. care-pathway / case spec §13).

---

## 8. Per-service validator summary

Which checks each service applies today. A blank cell means the entity
does not model that field, not that validation was skipped.

| Validator | person | place | care-pathway | case | case-folder |
| --------- | :----: | :---: | :----------: | :--: | :---------: |
| Required field (name/title/family) | ✓ | ✓ | ✓ | ✓ | |
| Coordinate bounds (lat/lon) | | ✓ | | | |
| GLN GS1 mod-10 | | ✓ | | | |
| NHS number Modulus-11 | | | | | ✓ |
| SNOMED CT Verhoeff | | | ✓ | | |
| ICD-10 / ICD-11 pattern | | | ✓ | | |
| UUID 8-4-4-4-12 | | | ✓ | | |
| DOI `10.x/y` | | | ✓ | | |
| BCP-47 language syntax | | | ✓ | | |
| Opening hours `HH:MM` | | ✓ | | | |
| ISO-8601 date | birth_date¹ | | | opened_date | |
| No future birth date | ✓ | | | | |
| Email format (`@` + `.`) | ✓ | | | | |
| Phone digit count / `+` prefix | ✓ | ✓ | | | ✓² |
| URL scheme (`http(s)://`) | | ✓ | | | |
| Address completeness | ✓ | ✓ | | | |
| Document number / expiry / issue-before-expiry | ✓ | | | | |
| Non-blank list entries (identifiers/subjects/keywords) | | | ✓ | ✓ | |
| Phone E.164-like normalization | ✓ | | | | |
| Address standardization (case + abbreviations) | ✓ | ✓³ | | | |

¹ person validates birth dates as non-future rather than parsing a string
(the model already holds a `chrono` date).
² case-folder normalizes/formats NHS numbers (`normalise_nhs_number`,
`format_nhs_number`) rather than validating a phone.
³ place normalizes locality/region/country case but does not expand
street abbreviations (place addresses model no street-type expansion).

---

## See also

- [REST conventions](../restful/index.md) — status-code policy (`400`
  vs `422`), error-body shape, the `validate()` / `bad_request()` helpers.
- [matching](../matching/index.md) — why malformed deterministic
  identifiers must be rejected before they reach the matcher.
- [PostgreSQL](../postgresql/index.md) — storage layer the validated
  records land in.

[Verhoeff]: https://en.wikipedia.org/wiki/Verhoeff_algorithm
