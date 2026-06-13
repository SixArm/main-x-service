## 10. Open questions

The following design questions are deliberately unresolved. Proposing a resolution is welcome; do so in a PR rather than a unilateral code change.

- **OQ-A — Category hierarchy.** Today `PlaceCategory` is a flat enum; a `Cafe` and a `Restaurant` either match or they don't. Should the spec define a hierarchy (e.g. `Cafe < FoodService < CommercialVenue`) and allow partial credit when categories agree at an ancestor level? Trade-off: explainability vs recall.
- **OQ-B — Country-code canonicalisation at construction.** `country_code_as_iso_3166_1_alpha_2` is stored as supplied; only the matcher trims and lowercases. Should `PlaceBuilder::country_code_as_iso_3166_1_alpha_2` canonicalise (uppercase, validate as exactly two ASCII letters) at construction time? Trade-off: round-trip honesty vs caller convenience.
- **OQ-C — Multi-polygon / area definitions.** `area_as_metre_2` is a scalar. Some place classes are better described by an explicit footprint (a park, a campus, a country). Should `Place` gain an optional polygonal extent? If so, in what format (WKT, GeoJSON, raw `Vec<(f64, f64)>`), and how would the matcher use it (point-in-polygon, polygon overlap)?
- **OQ-D — Locale-aware street-type vocabulary.** Today only English abbreviations are expanded (`St`, `Rd`, `Ave`, …). Should the crate gain locale-aware vocabularies for `rue` / `straße` / `via` / `calle` / `straat`? If so, opt-in via a new `MatchConfig` field, gated behind a Cargo feature, or always-on?
- **OQ-E — Phonetic-encoder choice.** American Soundex is tuned for English. A locale-aware encoder (Double Metaphone, Daitch-Mokotoff) would improve recall for non-English names. Add behind a Cargo feature flag with the default unchanged?
- **OQ-F — `local_id` scoring opt-in.** `local_id` is currently never scored because different sources may issue colliding values. Should a caller be able to opt in to scoring `local_id` when they know they are comparing records from a single source?
- **OQ-G — Address `line2`, `county`, `country` scoring.** These fields are stored but not scored (§6.4). Should they contribute, and if so with what sub-weights?
- **OQ-H — Date-line tolerance for `coordinates_scale_metres` defaults.** The default `50.0` m is tuned for venue precision; the rendering of dense urban chains may need a per-category default.

---

