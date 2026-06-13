## 12. Glossary cross-reference

| Symbol | One-line meaning | Defined in |
|---|---|---|
| `Address` | Postal address with optional line1/line2/city/county/postcode/country. | §3.3 |
| `Confidence` | High / Medium / Low band derived from probabilistic score. | §3.6 |
| `MatchBreakdown` | Per-field `Option<f64>` contributions returned with every `MatchResult`. | §3.7 |
| `MatchConfig` | Tunable weights, threshold, algorithm, presets. | §3.8 / §7 |
| `MatchResult` | `{ score, is_match, confidence, breakdown }` returned by `match_places`. | §3.7 |
| `MatchingEngine` | The engine. Immutable after construction. `Send + Sync`. | §5 |
| `MatchingError` | Sum-type for fallible operations. Only variant today is `MissingField`. | §3.9 |
| `Normalizer` | Stateless namespace for text normalisation. | §4 |
| `ParsedAddressLine` | `{ house_number, unit, street }` decomposition of an address line. | §4.5 |
| `Place` | Core place record. 15 fields, every one optional or defaulting to empty. | §3.1 |
| `PlaceBuilder` | Fluent builder for `Place`. | §3.2 |
| `PlaceCategory` | 35 enumerated variants plus `Other(String)`. | §3.4 |
| `PlaceId` | `{ scheme, value }`. Scheme-local equality. | §3.5 |
| `PlaceIdScheme` | 9 enumerated variants plus `Other(String)`. | §3.5 |
| `Result<T>` | Alias for `std::result::Result<T, MatchingError>`. | §3.9 |
| `Scorer` | Stateless namespace for similarity primitives (string, geographic). | §6 |
| `SimilarityAlgorithm` | `JaroWinkler` / `Levenshtein` / `Exact` / `Combined`. | §6.1 |
| `deterministic_match` | Boolean verdict. Shared place ID OR equal normalised name + postcode. | §5.1 |
| `match_one_to_many` | Score query against candidate slice; preserve input order. | §5.3 |
| `match_places` | Probabilistic single-pair match. Returns `MatchResult`. | §5.2 |
| `rank_one_to_many` | Score and sort by descending score; deterministic ascending-index tiebreak. | §5.3 |
| Renormalisation | Divide weighted sum by sum of participating weights. Missing fields skip. | §5.2.2 |
| Strict mode | `is_match` additionally requires `deterministic_match`. | §5.2.3 |

---

