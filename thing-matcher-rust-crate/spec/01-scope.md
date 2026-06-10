## 1. Scope

### 1.1 In scope

- Pairwise matching of two `Thing` records to decide whether they refer to the same item.
- A **deterministic** strategy returning `bool`.
- A **probabilistic** strategy returning a renormalised score in `[0.0, 1.0]` with a per-field `MatchBreakdown` for explainability.
- Batch entry points: scoring and ranking a single query against a slice of candidates.
- Supporting text-normalisation primitives (names, free text, URLs, phonetic codes).
- Set-similarity primitives (Jaccard over URL lists).
- Configurable weights, threshold, and similarity algorithm via `MatchConfig`.
- Serde round-trip of every public data type.

### 1.2 Out of scope

- **Persistent storage and indexing** — the crate never reads or writes external state.
- **Full-text search** and **candidate suggestions** — the crate scores known pairs in memory.
- **Per-scheme identifier canonicalisation** — the crate compares `(property_id, value)` pairs as opaque strings; vocabularies that need canonicalisation (e.g. ISBN-10 ↔ ISBN-13) MUST be canonicalised upstream before being handed to the matcher.
- **Cross-scheme identifier resolution** — `(wikidata, Q243)` never matches `(viaf, 156122861)` even when both point at the same real-world thing.
- **Candidate blocking** — pre-filtering large candidate sets is a consumer concern.
- **Machine learning** — the algorithm is rule-based; weights are tuneable but the structure is fixed.
- **Domain-specific subtype matching** — the crate treats every `Thing` as a generic schema.org root. Specialised subtypes (Person, Place, Event, …) have dedicated sibling crates in the same family.

### 1.3 Audience

Data engineers, data-stewardship teams, and deduplication-pipeline authors who need an explainable, deterministic library for joining or de-duplicating heterogeneous records of "things" — books, papers, artworks, software, devices, products, digital assets — drawn from disparate source systems.

---

