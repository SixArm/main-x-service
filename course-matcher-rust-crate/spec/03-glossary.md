## 3. Glossary

| Term | Meaning |
|---|---|
| **Deterministic scheme** | An identifier scheme whose values are unique-by-construction (DOI, Wikidata, …). A match short-circuits scoring to 1.0. |
| **Renormalisation** | Weighted sum / sum-of-weights over the present components, not the full configured weight table. |
| **Same-provider** | Two records sharing `provider_id`. Required for the course-code component to contribute. |
| **Confidence band** | Coarse `{High, Medium, Low}` classification of the final score. |
| **`is_match`** | Score ≥ `MatchConfig::threshold` (default 0.85). |

