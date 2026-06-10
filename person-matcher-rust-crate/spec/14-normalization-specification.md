## 14. Normalization Specification

Full algorithms are in [AGENTS/normalization.md](../AGENTS/normalization.md) under "Detailed Normalisation Specifications". Summary:

- **Names** (`normalize_name`) — NFKD + drop combining marks + drop ASCII punctuation + lowercase + collapse whitespace. (`José` → `jose`.)
- **Postcodes** (`normalize_postcode`) — drop whitespace, uppercase (`CF10 1AA` → `CF101AA`).
- **Phones legacy** (`normalize_phone`) — UK-centric: keep digits, strip `0044` / `44` / leading `0`; infallible fallback.
- **Phones E.164** (`normalize_phone_e164`) — match `+CC` / `00CC` / `default_country` against the 39-jurisdiction `COUNTRY_PHONE_TABLE`, strip national trunk prefix, validate NSN length; return `+CCNNN…` or `None`.
- **Email** (`normalize_email`) — trim + lowercase + structural validation; opt-in Gmail dot-/`+tag`-folding for `gmail.com` / `googlemail.com`.
- **Address lines** (`expand_street_abbreviations`, `normalize_address_line`, `parse_address_line`) — token-level abbreviation expansion + name normalisation; `parse_address_line` returns `ParsedAddressLine { house_number, unit, street }`.
- **Phonetic** (`phonetic_code`) — name normalisation then American Soundex.
- **National identifiers** (`identifiers::parse_<cc>_<scheme>`) — 42 per-scheme parsers; see `AGENTS/national-person-identifiers.md`.

Invariants: normalisers SHOULD be idempotent; identifier parsers are scheme-local (parsers sharing an algorithm MUST NOT cross-match); phone matching prefers E.164 with legacy fallback (FR-30); the 39-jurisdiction phone table covers every identifier-scheme jurisdiction (T-19); `local_id` is deliberately NOT normalised and NOT scored.

