## 1. Scope

### 1.1 In scope

- Pairwise matching of two `Place` records to decide whether they refer to the same geographic place.
- A **deterministic** strategy returning `bool`.
- A **probabilistic** strategy returning a renormalised score in `[0.0, 1.0]` with a per-field `MatchBreakdown` for explainability.
- Batch entry points: scoring and ranking a single query against a slice of candidates.
- Supporting text-normalisation primitives (names, postcodes, phones, emails, addresses, phonetic codes).
- Geographic primitives (Haversine distance on a sphere, Gaussian-decay similarity).
- Configurable weights, threshold, and similarity algorithm via `MatchConfig`.
- Serde round-trip of every public data type.

### 1.2 Out of scope

- **Geocoding** (address → coordinates) and **reverse geocoding** (coordinates → address).
- **Routing** or network distance — only great-circle (Haversine) distance is provided.
- **Address parsing as a service**: `Normalizer::parse_address_line` is a best-effort structural decomposition for matching purposes, not a postal-reference lookup.
- **Full-text search** and **place suggestions** — the crate scores known pairs in memory.
- **Persistent storage and indexing** — the crate never reads or writes external state.
- **Candidate blocking** — pre-filtering large candidate sets is a consumer concern.
- **Machine learning** — the algorithm is rule-based; weights are tuneable but the structure is fixed.
- **Locale-aware street-type vocabularies** — only English abbreviations are expanded.

### 1.3 Audience

Data engineers, GIS practitioners, and deduplication-pipeline authors who need an explainable, deterministic library for joining or de-duplicating geographic-place records drawn from heterogeneous sources.

---

