## 14. Normalization Specification

Verbatim per-subsection algorithms (name / postcode / phone-legacy + phone-E.164 + 39-country table / email / address-line parser / phonetic / per-scheme national-identifier normalisation) are archived in [`AGENTS/normalization.md`](../AGENTS/normalization.md). Per-scheme rules also catalogued in [`AGENTS/national-person-identifiers.md`](../AGENTS/national-person-identifiers.md). Update both surfaces in lockstep.

Public entry points: `Normalizer::normalize_name` (NFKD → drop combining marks → drop ASCII punctuation → lowercase → collapse whitespace); `normalize_postcode` (strip whitespace + uppercase); `normalize_phone` (legacy, UK-centric); `normalize_phone_e164` (matches `+CC` / `00CC` / `default_country` against 39-country table; strips trunk; validates NSN; returns `+CCNNN…`); `normalize_email` (trim + lowercase + `@` validation; opt-in Gmail dot/+tag folding); `normalize_address_line` / `parse_address_line` (expand abbreviations + name-normalise; parser returns `ParsedAddressLine { house_number, unit, street }`); `phonetic_code` (American Soundex on name-normalised input; T-9.1 adds opt-in alternatives); `identifiers::parse_<cc>_<scheme>` (per-scheme canonical form per FR-12..FR-91).

Design axiom (per §5): most accuracy gains come from data standardisation. Two inputs representing the same value in different textual layouts MUST canonicalise to the same string.

