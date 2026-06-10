## 10. Open questions

- **OQ-A — Soundex vs. Metaphone for non-English names.** Soundex was designed for English surnames and is known to be weak for many non-English orthographies. Should `MatchConfig` gain a `phonetic_encoder` enum (Soundex / Double Metaphone / NYSIIS)? Decision deferred until a multilingual evaluation corpus is available.
- **OQ-B — Cross-scheme identifier resolution.** Should the crate ship an opt-in helper that recognises `(isbn, 0-201-89683-4)` and `(isbn, 9780201896831)` as the same identifier under ISBN-10 ↔ ISBN-13 canonicalisation? Today's stance: keep canonicalisation upstream and out of this crate.
- **OQ-C — Per-scheme identifier weights.** Some `property_id` values (`"isbn"`, `"doi"`, `"gtin"`) are globally unique by construction; others (`"sku"`, `"mpn"`) are not. Should the matcher tag schemes as "globally unique" and treat shared values in that bucket as a stronger signal? Today: every shared `(property_id, value)` pair short-circuits to `deterministic_match = true` regardless.
- **OQ-D — `description` vs. `disambiguating_description` interaction.** When both fields are present on both sides, the score includes both contributions independently. Should `disambiguating_description` be promoted to a tie-breaker only? Today: both contribute via the standard weighted sum.

---

