# Thing matcher — specification

**Crate:** `thing-matcher` &nbsp;·&nbsp; **Version targeted:** `0.4.0` &nbsp;·&nbsp; **Status:** authoritative

This document is the living, single source of truth (SSOT) for the `thing-matcher` Rust crate. Every other document in the repository (`README.md`, `index.md`, `AGENTS.md`, `AGENTS/*.md`, `CHANGELOG.md`) summarises or quotes this file — none contradicts or extends it. When prose elsewhere disagrees with this file, this file wins; when this file disagrees with the code, see §9.

The keywords **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are to be interpreted in the sense of RFC 2119 / RFC 8174.

---

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

## 3. Data model

The public types live in `crate::models`. All derive `Serialize + Deserialize`. Construction MUST go through the `ThingBuilder` rather than struct literal syntax (`Thing` is `#[non_exhaustive]`).

### 3.1 `Thing`

| Field | Type | Sense | Notes |
|---|---|---|---|
| `name` | `Option<String>` | Primary canonical name (schema.org `name`). | Required for `validate()`. |
| `alternate_names` | `Vec<String>` | Aliases / endonyms / translations (schema.org `alternateName`). | Best-of pair-wise scoring against the primary name. |
| `description` | `Option<String>` | Free-form description (schema.org `description`). | Scored with `Combined` similarity after `normalize_text`. |
| `disambiguating_description` | `Option<String>` | Short disambiguating description (schema.org `disambiguatingDescription`). | Scored with `Combined` similarity. |
| `identifiers` | `Vec<Identifier>` | Typed external IDs (schema.org `identifier` as `PropertyValue`). | Sharing any one `(property_id, value)` pair across two things is a deterministic match. |
| `url` | `Option<String>` | Canonical URL (schema.org `url`). | After normalisation, equality contributes to the score AND to deterministic-match. |
| `image` | `Option<String>` | Representative image URL (schema.org `image`). | After normalisation, equality contributes to the score. |
| `same_as` | `Vec<String>` | Authoritative external URLs (schema.org `sameAs`). | Jaccard set similarity scores the pair; any shared URL is a deterministic match. |
| `main_entity_of_page` | `Option<String>` | URL of the page for which this is the main entity (schema.org `mainEntityOfPage`). | Exact match after URL normalisation. |
| `additional_types` | `Vec<String>` | Subtype URIs from external vocabularies (schema.org `additionalType`). | Jaccard set similarity after URL normalisation. |
| `subject_of` | `Vec<String>` | URLs of works or events about this thing (schema.org `subjectOf`). | Data-only; not scored today. |
| `owner` | `Option<String>` | Owner — person or organisation (schema.org `owner`). | Data-only; not scored. The crate does not model `Person` / `Organization` separately. |
| `local_id` | `Option<String>` | Local identifier issued by the originating system. | Data-only; not normalised, not scored. Different organisations may issue colliding values. |

The `Thing` struct is `#[non_exhaustive]`: external consumers MUST use `Thing::builder()` to construct values, so future field additions can ship as minor releases without breaking downstream code.

`Thing::validate(&self)` returns `Err(MatchingError::MissingField(_))` when `name` is absent. The matcher does NOT invoke `validate()` automatically — call it at ingest time, not on every comparison.

### 3.2 `ThingBuilder`

Fluent builder for `Thing`. All string setters accept `impl Into<String>` so call-sites may pass `&str`, `String`, or `&String` interchangeably. List setters come in two flavours: `field(Vec<T>)` to replace and `add_field(T)` to append. The builder yields a `Thing` via `.build()`.

Builder defaults: every field starts as `None` / empty.

### 3.3 `Identifier`

```rust
pub struct Identifier {
    pub property_id: String,
    pub value: String,
}
```

Two `Identifier`s are equal iff both `property_id` and `value` are equal — equality is structural, no per-scheme canonicalisation is performed. Construct via `Identifier::new(property_id, value) -> Option<Self>`; both components are trimmed of surrounding whitespace, and the constructor returns `None` if either trimmed component is empty. `Identifier` derives `Hash`, so it may be used in `HashSet` / `HashMap`.

The `property_id` SHOULD be a stable vocabulary name (`"wikidata"`, `"isbn"`, `"doi"`, `"gtin"`, `"openlibrary"`) or a fully-qualified URL. The matcher treats it as an opaque case-sensitive string — `"wikidata"` and `"WikiData"` are distinct schemes.

### 3.4 `MatchConfig`

Tunable configuration for the matching engine. All weights are dimensionless and contribute to a renormalised weighted sum — they do not need to add to `1.0`. See §5.10.

| Field | Type | Default | Strict | Lenient |
|---|---|---:|---:|---:|
| `match_threshold` | `f64` | `0.80` | `0.95` | `0.65` |
| `name_weight` | `f64` | `0.30` | `0.30` | `0.30` |
| `description_weight` | `f64` | `0.10` | `0.10` | `0.10` |
| `disambiguating_description_weight` | `f64` | `0.05` | `0.05` | `0.05` |
| `identifiers_weight` | `f64` | `0.25` | `0.25` | `0.25` |
| `url_weight` | `f64` | `0.05` | `0.05` | `0.05` |
| `same_as_weight` | `f64` | `0.15` | `0.15` | `0.15` |
| `image_weight` | `f64` | `0.03` | `0.03` | `0.03` |
| `main_entity_of_page_weight` | `f64` | `0.02` | `0.02` | `0.02` |
| `additional_types_weight` | `f64` | `0.05` | `0.05` | `0.05` |
| `use_phonetic_matching` | `bool` | `false` | `false` | `true` |
| `name_algorithm` | `SimilarityAlgorithm` | `Combined` | `Combined` | `Combined` |
| `strict_mode` | `bool` | `false` | `true` | `false` |

`MatchConfig` derives `Serialize + Deserialize` and carries `#[serde(default)]`, so partial JSON documents merge over `MatchConfig::default()`. The matcher MUST treat any negative weight as zero — but the public API SHOULD reject negative values at construction time.

### 3.5 `SimilarityAlgorithm`

```rust
pub enum SimilarityAlgorithm {
    JaroWinkler,
    Levenshtein,
    Exact,
    Combined,   // default
}
```

`Combined` averages Jaro-Winkler and Levenshtein, slightly downweighting Levenshtein. See §6.1.

### 3.6 `MatchResult`

```rust
pub struct MatchResult {
    pub score: f64,                // renormalised, in [0.0, 1.0]
    pub is_match: bool,            // score >= threshold (AND deterministic, under strict_mode)
    pub confidence: Confidence,    // High / Medium / Low band
    pub breakdown: MatchBreakdown, // per-field detail
}
```

`confidence` carries `#[serde(default = "default_confidence")]`, so legacy JSON payloads lacking the field deserialise to `Confidence::Low` ("needs re-scoring").

### 3.7 `MatchBreakdown`

```rust
pub struct MatchBreakdown {
    pub name_score:                      Option<f64>,
    pub name_phonetic_score:             Option<f64>,
    pub description_score:               Option<f64>,
    pub disambiguating_description_score: Option<f64>,
    pub identifiers_score:               Option<f64>,
    pub url_score:                       Option<f64>,
    pub same_as_score:                   Option<f64>,
    pub image_score:                     Option<f64>,
    pub main_entity_of_page_score:       Option<f64>,
    pub additional_types_score:          Option<f64>,
}
```

Per field: `Some(s)` means the field was scored, `s ∈ [0.0, 1.0]`. `None` means at least one side was absent / empty, so the field did not participate in the weighted sum. Downstream services MUST NOT discard the breakdown — it is the audit trail for the `score`.

### 3.8 `Confidence`

```rust
pub enum Confidence { High, Medium, Low }
```

Banding:

| Score | Band |
|---|---|
| `score >= 0.90` | `High` |
| `0.75 <= score < 0.90` | `Medium` |
| `score < 0.75` | `Low` |

Bands are **fixed** across all `MatchConfig` presets — they do NOT follow `match_threshold`. `Confidence::from_score(score)` is the public constructor. NaN / negative inputs degrade to `Low`; values above `1.0` degrade to `High`.

### 3.9 `MatchingError`

```rust
#[non_exhaustive]
pub enum MatchingError {
    MissingField(String),
}
```

Single open variant today. `#[non_exhaustive]` is the SemVer covenant: future fallible code paths can add variants without breaking consumers.

---

## 4. Normalisation

All comparisons are done **after** normalisation. The normalisation routines live in `crate::normalizer` and are exposed through the `Normalizer` unit type. Every routine is **idempotent** (`f(f(x)) == f(x)`), **deterministic**, and allocates at most one new `String`.

| Routine | Use | Behaviour |
|---|---|---|
| `Normalizer::normalize_name(&str)` | Names and alternate names. | NFKD decompose → drop combining marks → drop ASCII punctuation → lowercase → collapse and trim ASCII whitespace. |
| `Normalizer::normalize_text(&str)` | `description`, `disambiguating_description`. | Lowercase → NFKD decompose → collapse whitespace → trim. Punctuation is **retained** so descriptions remain readable. |
| `Normalizer::normalize_url(&str)` | `url`, `image`, `main_entity_of_page`, every entry of `same_as` and `additional_types`. | Lowercase scheme + host; drop trailing slash on the path root. No DNS-aware normalisation, no percent-encoding canonicalisation, no punycode decoding. |
| `Normalizer::phonetic_code(&str)` | Soundex bonus (§6.5). | Classic 4-character Soundex code: first letter + three digits (`0` padding when fewer consonant digits are available). Diacritics are stripped via `normalize_name` first. |

Detailed per-rule behaviour (every NFKD edge case, every URL handling exception, exact whitespace handling) lives in [`AGENTS/normalization.md`](AGENTS/normalization.md). Behaviours that consumers MUST rely on:

- **Whitespace.** Inputs MAY contain leading, trailing, or internal runs of any whitespace; all of it is canonicalised to single ASCII spaces, then trimmed.
- **Diacritics.** Latin diacritics (Spanish `ó`, German `ü`, French `é`, …) are stripped from names and Soundex codes; this is intentional and stable.
- **Punctuation.** ASCII apostrophes, hyphens, full stops, commas, parentheses, etc. are stripped from names. The curly apostrophe `’` (U+2019) is NOT recognised — upstream code MUST convert smart quotes to ASCII first.
- **URLs.** Equality is host- and scheme-insensitive, but path-, query-, and fragment-sensitive. Two URLs differing only by `?utm_source=…` are NOT equal.
- **Empty handling.** `normalize_name("")` and `normalize_name("   ")` both return `""`. The scoring layer treats empty / whitespace-only fields as "missing".

---

## 5. Matching engine

### 5.1 `MatchingEngine::deterministic_match(&self, &Thing, &Thing) -> bool`

Returns `true` iff any of the following hold:

1. The two things share any `(property_id, value)` pair in their `identifiers` lists. Property IDs are compared as case-sensitive opaque strings; values are compared after trimming (done at construction by `Identifier::new`).
2. The two things share any `sameAs` URL after `normalize_url`.
3. Both things have a `url` and the two URLs are equal after `normalize_url`.

Otherwise returns `false`.

Properties:

- **Reflexive.** `deterministic_match(t, t)` is `true` whenever `t` has at least one identifier, sameAs URL, or canonical url.
- **Symmetric.** `deterministic_match(a, b) == deterministic_match(b, a)`.
- **Cheap.** O(n·m) over the smaller cross-product of identifier and URL lists; no string-similarity work; no allocation beyond the per-URL normalisation buffer.

### 5.2 `MatchingEngine::match_things(&self, &Thing, &Thing) -> MatchResult`

The probabilistic path. Produces a `MatchResult` (§3.6) carrying a renormalised score (§5.10), the `is_match` boolean, the `Confidence` band, and the per-field `MatchBreakdown`.

Under `MatchConfig::strict_mode = true`, `is_match` requires `score >= threshold` **AND** `deterministic_match(a, b)`. Probabilistic `score` and `confidence` are unchanged under strict mode — only `is_match` becomes more conservative.

### 5.3 Batch entry points

```rust
fn match_one_to_many(&self, query: &Thing, candidates: &[Thing]) -> Vec<MatchResult>;
fn rank_one_to_many(&self, query: &Thing, candidates: &[Thing]) -> Vec<(usize, MatchResult)>;
```

- `match_one_to_many` returns one `MatchResult` per candidate, in input order.
- `rank_one_to_many` returns `(original_index, MatchResult)` tuples sorted by descending score; ties break by ascending original index so the result is fully deterministic.

The engine is immutable after construction and `Send + Sync`, so consumers MAY wrap either call in `rayon::par_iter`, `tokio::spawn_blocking`, or similar without changes to this crate.

### 5.4 Constructors

```rust
MatchingEngine::new(MatchConfig)        // explicit
MatchingEngine::default_config()        // = new(MatchConfig::default())
```

The engine owns only a `MatchConfig`, so cloning is cheap. A consumer MAY hold one engine per (strict / default / lenient) preset across the lifetime of its process.

### 5.5 Determinism, safety, IO

- **Deterministic.** Same inputs produce the same outputs, in the same byte order. No clocks, no RNGs, no environment variables.
- **No `unsafe`.** The crate top-level declares `#![forbid(unsafe_code)]`.
- **No IO.** The library never logs, reads files, opens sockets, or queries DNS.
- **Panic-free.** Every fallible input returns either `None` from a scorer or a `MatchingError`. The library MUST NOT panic on any in-range `f64` or any valid UTF-8 string.
- **`Send + Sync`.** Every public type implements both. The engine is safe to share across threads by reference.

### 5.6 String similarity primitives

`Scorer` (in `crate::scorer`) exposes:

- `Scorer::jaro_winkler_similarity(&a, &b) -> f64`
- `Scorer::levenshtein_similarity(&a, &b)  -> f64`
- `Scorer::exact_match(&a, &b)             -> f64`  // `1.0` or `0.0`
- `Scorer::combined_similarity(&a, &b)     -> f64`  // `0.6·JW + 0.4·Lev`
- `Scorer::jaccard_similarity(&[T], &[T])  -> f64`  // for `same_as`, `additional_types`

All similarity values are in `[0.0, 1.0]`. Empty-string handling: `jaro_winkler` / `levenshtein` / `combined` return `1.0` when both inputs are empty and `0.0` when exactly one is empty; `exact_match` returns `1.0` only when both inputs are equal (including both empty).

### 5.7 Phonetic matching

When `MatchConfig::use_phonetic_matching = true`, the engine computes Soundex codes (`Normalizer::phonetic_code`) for every name on each side (primary + alternate names) and sets `name_phonetic_score = 1.0` iff any cross-pair shares a non-empty Soundex code; `0.0` otherwise. The result is added as a **bonus** in §5.10 — it never lowers the score.

When `use_phonetic_matching = false`, `name_phonetic_score` is `None` and the bonus is omitted entirely.

### 5.8 Identifier scoring (`identifiers_score`)

| Condition | `identifiers_score` |
|---|---|
| Either side's `identifiers` list is empty | `None` |
| Both sides non-empty AND any `(property_id, value)` pair is shared | `Some(1.0)` |
| Both sides non-empty AND no pair is shared | `Some(0.0)` |

Note the **asymmetry vs. deterministic match**: an empty `identifiers` list on either side suppresses the probabilistic contribution entirely (so the empty side does not unfairly tank the score), but `deterministic_match` would still consider the URL signals. This is intentional.

### 5.9 URL scoring

| Field | Behaviour |
|---|---|
| `url_score` | `None` if either side absent; `1.0` if both `normalize_url`-equal; `0.0` otherwise. |
| `image_score` | Same shape as `url_score`. |
| `main_entity_of_page_score` | Same shape as `url_score`. |
| `same_as_score` | Jaccard over the union of normalised `same_as` URLs. `None` only if BOTH sides are empty; otherwise `Some(intersection / union)`. |
| `additional_types_score` | Same shape as `same_as_score`. |

### 5.10 Renormalised weighted sum

```
weighted_sum  = 0.0
total_weight  = 0.0

for each scored field (name, description, disambig, identifiers,
                       url, same_as, image, main_entity_of_page,
                       additional_types):
    if breakdown.field_score is Some(s):
        weighted_sum += s * config.field_weight
        total_weight += config.field_weight

if phonetic_enabled AND breakdown.name_phonetic_score == Some(s) AND s > 0.9:
    weighted_sum += s * 0.05
    total_weight += 0.05

score = if total_weight > 0.0 { weighted_sum / total_weight } else { 0.0 }
```

Properties:

- **Missing fields do not penalise.** A `None` field is skipped entirely — neither numerator nor denominator changes.
- **Phonetic is a bonus.** The Soundex contribution adds to both `weighted_sum` and `total_weight` only when its score is above `0.9`, which means it can only push the renormalised score upward (since it adds a value above `0.9` weighted by `0.05`).
- **Empty input.** If every field is `None`, the score is `0.0` and `is_match` is `false`.
- **Bounded.** The score is mathematically guaranteed to lie in `[0.0, 1.0]`.

### 5.11 Score → `is_match` decision

```
above_threshold = score >= config.match_threshold

is_match = if config.strict_mode {
    above_threshold && deterministic_match(a, b)
} else {
    above_threshold
}
```

`confidence = Confidence::from_score(score)` independently of `match_threshold` (see §3.8).

---

## 6. Per-field scoring algorithms

This section pins the exact algorithm for each scored field. Detailed pseudocode and edge-case tables are in [`AGENTS/matching-algorithm.md`](AGENTS/matching-algorithm.md).

### 6.1 `name_score`

Best-of cartesian product over `(primary name + alternate names)` on each side. For each `(n1, n2)` pair: apply `Normalizer::normalize_name` to both, then run the configured `name_algorithm`:

- `JaroWinkler` — `Scorer::jaro_winkler_similarity` over the normalised strings.
- `Levenshtein` — `Scorer::levenshtein_similarity` (edit distance normalised by max length).
- `Exact` — `1.0` if equal, `0.0` otherwise.
- `Combined` (default) — `0.6 · jaro_winkler + 0.4 · levenshtein`.

`name_score` is `None` iff either side has no non-empty name (after trimming).

### 6.2 `description_score`, `disambiguating_description_score`

`Scorer::combined_similarity` over the two strings after `Normalizer::normalize_text`. `None` if either side is absent.

### 6.3 `identifiers_score`

See §5.8.

### 6.4 `url_score`, `image_score`, `main_entity_of_page_score`

See §5.9 — exact equality after `Normalizer::normalize_url`. `None` if either side is absent.

### 6.5 `same_as_score`, `additional_types_score`

Jaccard set similarity (`|A ∩ B| / |A ∪ B|`) over the URL lists after `Normalizer::normalize_url`. `None` only when both lists are empty.

### 6.6 `name_phonetic_score`

See §5.7. `None` when `use_phonetic_matching` is `false` or either side has no names.

---

## 7. Quality attributes and tuning

### 7.1 Performance budget

- A single `match_things` call SHOULD complete in `< 50 µs` on a 2024-vintage Apple Silicon laptop with default config and typical (≤ 5-name, ≤ 10-identifier, ≤ 5-sameAs) records.
- `rank_one_to_many` SHOULD scale linearly in the candidate count — no shared state between candidates.

### 7.2 Determinism budget

The crate produces identical bytes for identical inputs across runs, processes, and machines. There is no thread-local state, no global cache, no order-dependent collection iteration.

### 7.3 Stability

Once `0.4.0` ships, the public types (`Thing`, `MatchConfig`, `MatchResult`, `MatchBreakdown`, `Identifier`, `Confidence`, `MatchingError`, `SimilarityAlgorithm`, `MatchingEngine`, `Normalizer`, `Scorer`) and their semantics are stable under SemVer. Field additions to `Thing` and `MatchBreakdown` go via `#[non_exhaustive]` so they are not breaking. New `MatchingError` variants are non-breaking for the same reason.

### 7.4 Tuning guidance

Detailed tuning guidance (when to reach for `strict()`, when for `lenient()`, when to raise `identifiers_weight`, when to enable phonetic matching) lives in [`AGENTS/matching-algorithm.md`](AGENTS/matching-algorithm.md). Defaults are calibrated for general-purpose catalogue deduplication.

---

## 8. Public API surface

| Item | Module | Kind |
|---|---|---|
| `Thing`, `ThingBuilder`, `Identifier` | `crate::models` | structs |
| `MatchingEngine`, `MatchConfig`, `MatchResult`, `MatchBreakdown`, `Confidence` | `crate::matcher` | structs + enum |
| `Normalizer` | `crate::normalizer` | unit type with associated fns |
| `Scorer`, `SimilarityAlgorithm` | `crate::scorer` | unit type + enum |
| `MatchingError`, `Result` | `crate::error` | enum + alias |

All of the above are re-exported from the crate root, so a downstream consumer can write `use thing_matcher::{Thing, MatchingEngine, MatchConfig};` without reaching into submodules.

The full doc set is generated by `cargo doc --no-deps`; every public item carries a doc comment with at least one runnable example. Doctests MUST pass on every CI run (§11).

---

## 9. Specification / code drift discipline

When the spec disagrees with the code, the spec wins **only after** the disagreement is resolved in writing: a task in §13 OR a PR that updates one of {spec, code, tests} in the same commit set as the other two.

Three-part-PR rule: a behavioural change is one PR that contains a spec edit, a code edit, and a test edit. PRs touching `src/matcher.rs` or `src/scorer.rs` without a corresponding spec edit MUST be flagged in review.

---

## 10. Open questions

- **OQ-A — Soundex vs. Metaphone for non-English names.** Soundex was designed for English surnames and is known to be weak for many non-English orthographies. Should `MatchConfig` gain a `phonetic_encoder` enum (Soundex / Double Metaphone / NYSIIS)? Decision deferred until a multilingual evaluation corpus is available.
- **OQ-B — Cross-scheme identifier resolution.** Should the crate ship an opt-in helper that recognises `(isbn, 0-201-89683-4)` and `(isbn, 9780201896831)` as the same identifier under ISBN-10 ↔ ISBN-13 canonicalisation? Today's stance: keep canonicalisation upstream and out of this crate.
- **OQ-C — Per-scheme identifier weights.** Some `property_id` values (`"isbn"`, `"doi"`, `"gtin"`) are globally unique by construction; others (`"sku"`, `"mpn"`) are not. Should the matcher tag schemes as "globally unique" and treat shared values in that bucket as a stronger signal? Today: every shared `(property_id, value)` pair short-circuits to `deterministic_match = true` regardless.
- **OQ-D — `description` vs. `disambiguating_description` interaction.** When both fields are present on both sides, the score includes both contributions independently. Should `disambiguating_description` be promoted to a tie-breaker only? Today: both contribute via the standard weighted sum.

---

## 11. Worked examples

Worked examples are kept in `tests/integration_tests.rs` and in the crate-level doctests (`src/lib.rs`, `src/matcher.rs`). The summary below sketches each scenario; the test files are the runnable source.

### 11.1 Canonical probabilistic match

Two records of the same item with matching `url` and overlapping names produce a high score, `Confidence::High`, `is_match = true`.

```rust
use thing_matcher::{MatchingEngine, Thing};

let a = Thing::builder()
    .name("Pride and Prejudice")
    .add_alternate_name("Stolz und Vorurteil")
    .url("https://example.org/book/9780141439518")
    .build();
let b = Thing::builder()
    .name("Stolz und Vorurteil")
    .url("https://example.org/book/9780141439518")
    .build();

let result = MatchingEngine::default_config().match_things(&a, &b);
assert!(result.is_match);
```

### 11.2 Deterministic match via shared identifier

Two records of the same book with different titles ("Pride and Prejudice" / "Stolz und Vorurteil") but sharing an `(isbn, 9780141439518)` pair produce `deterministic_match = true` regardless of probabilistic score.

### 11.3 Deterministic match via shared `sameAs`

Both records carry `same_as = ["https://www.wikidata.org/wiki/Q170583"]` → `deterministic_match = true` even when names and URLs differ.

### 11.4 Renormalisation: missing fields do not penalise

Both records carry only `name`; everything else is `None`. The renormalised score is computed against only the `name_weight`, so an exact name match scores `1.0` rather than `0.30 / (0.30 + …)`.

### 11.5 Strict mode rejects fuzzy-only matches

Two records with high name similarity but no shared identifier / sameAs / url and `strict_mode = true` produce `is_match = false` even when `score > threshold`.

### 11.6 Phonetic bonus

`Normalizer::phonetic_code("Robert") == Normalizer::phonetic_code("Rupert") == "R163"`. With `use_phonetic_matching = true`, two records whose names produce the same Soundex code receive a small score uplift.

### 11.7 Batch ranking

`rank_one_to_many` against a 100-candidate slice returns `(index, MatchResult)` tuples sorted by descending score with deterministic tie-breaking on the original index.

---

## 12. Glossary cross-reference

- **`Thing` properties**: §3.1 lists every field with its schema.org property and its scoring sense.
- **Matching strategies**: deterministic = §5.1; probabilistic = §5.2; strict-mode combination = §5.11.
- **Normalisation routines**: §4 lists the four `Normalizer` entry points; per-rule detail in [`AGENTS/normalization.md`](AGENTS/normalization.md).
- **Similarity primitives**: §5.6 lists the four `Scorer` functions plus Jaccard.
- **Renormalisation**: §5.10. Renormalisation is what lets missing fields neither contribute nor penalise.
- **Confidence bands**: §3.8 — fixed bands independent of `match_threshold`.

---

## 13. References

- [`schema.org/Thing`](https://schema.org/Thing) — root vocabulary for the data model.
- [`schema.org/PropertyValue`](https://schema.org/PropertyValue) — model for `Identifier`.
- [`schema.org/sameAs`](https://schema.org/sameAs) — semantics of the `same_as` field.
- Jaro, M. A. (1989). *Advances in record-linkage methodology as applied to matching the 1985 census of Tampa, Florida.* — Jaro similarity.
- Winkler, W. E. (1990). *String comparator metrics and enhanced decision rules for the Fellegi-Sunter model of record linkage.* — Jaro-Winkler.
- RFC 2119 / RFC 8174 — RFC keyword interpretation.
- Sibling crate specs: [`person-matcher/spec.md`](../person-matcher-rust-crate/spec.md), [`worker-matcher/spec.md`](../worker-matcher-rust-crate/spec.md), [`place-matcher/spec.md`](../place-matcher-rust-crate/spec.md), [`event-matcher/spec.md`](../event-matcher-rust-crate/spec.md).
- [`AGENTS/matching-algorithm.md`](AGENTS/matching-algorithm.md) — per-field algorithm detail.
- [`AGENTS/normalization.md`](AGENTS/normalization.md) — per-rule normalisation detail.
- [`AGENTS/testing.md`](AGENTS/testing.md) — test layout and CI.
- [`AGENTS/spec-driven-development.md`](AGENTS/spec-driven-development.md) — three-part-PR discipline.
