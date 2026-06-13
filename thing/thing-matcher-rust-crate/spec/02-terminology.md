## 2. Terminology

The following terms are used throughout this spec with the meanings defined here. Other documents in the repository MUST use the same vocabulary.

- **Thing** — a single record about an arbitrary discrete item, as represented by the `Thing` struct (§3.1). The data model is faithful to [`schema.org/Thing`](https://schema.org/Thing) — the root type from which all schema.org types descend.
- **Match** — the verdict that two `Thing` records refer to the same item. Verdicts come in two flavours, deterministic and probabilistic, with sharply different guarantees.
- **Deterministic match** — a boolean verdict from `MatchingEngine::deterministic_match` (§5.1). Returns `true` only when an objective, transitive criterion is satisfied (shared identifier pair, shared `sameAs` URL, or same canonical `url`). Never produces a score.
- **Probabilistic match** — a `MatchResult` from `MatchingEngine::match_things` (§5.2) carrying a score, an `is_match` boolean, a `Confidence` band, and a `MatchBreakdown`.
- **Per-field breakdown** — the `MatchBreakdown` struct (§3.7) containing one `Option<f64>` per scored component. `None` means "not scored on at least one side"; `Some(s)` carries a value in `[0.0, 1.0]`.
- **Renormalisation** — the rule in §5.10 by which the weighted sum is divided by the sum of *participating* weights, so missing fields neither contribute to nor penalise the overall score.
- **Identifier** — a typed external reference modelled on [`schema.org/PropertyValue`](https://schema.org/PropertyValue): a `(property_id, value)` pair where `property_id` is the vocabulary or issuer (`"wikidata"`, `"isbn"`, `"doi"`, `"gtin"`, …) and `value` is the identifier string itself.
- **sameAs URL** — a URL that authoritatively names the same thing on a third-party system (Wikipedia article, Wikidata entity, OCLC record, …). Maps to [`schema.org/sameAs`](https://schema.org/sameAs).
- **Canonical URL** — the URL of the thing's own primary web representation. Maps to [`schema.org/url`](https://schema.org/url).

---

