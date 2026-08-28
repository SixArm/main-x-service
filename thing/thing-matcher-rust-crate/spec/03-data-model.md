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
| `relationships` | `Vec<RelationshipRef>` | Typed thing-to-thing references (containment / part-of). | Landed in v0.7.0. Typed-set Jaccard over the `(relation, thing_id)` pairs (§3.3.1, §6.6) is specified; a supporting signal, not identifying on its own. `None` when either side is empty. Default empty, `#[serde(default)]`. |
| `tags` | `Vec<String>` | Short free-text operational labels (grouping / triage / workflow). | Landed in v0.7.0. Set Jaccard over the case-insensitively normalised tag sets (§6.8) is specified; a supporting signal, not identifying on its own. `None` when either side is empty. Defaults to empty, `#[serde(default)]`. |
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

### 3.3.1 `RelationshipRef` and `RelationKind`

Landed in v0.7.0. Both types live in `crate::models`, re-exported from the crate root.

```rust
pub struct RelationshipRef {
    pub relation: RelationKind,
    pub thing_id: String,
}

#[non_exhaustive]
pub enum RelationKind {
    Contains,
    ContainedIn,
    SuperPart,
    SubPart,
}
```

`relationships: Vec<RelationshipRef>` holds typed thing-to-thing references, each `RelationshipRef { relation: RelationKind, thing_id: String }` where `thing_id` is an opaque registry id (whitespace-trimmed, non-empty). `RelationKind` is a `#[non_exhaustive]` enum mirroring the service: `Contains` / `ContainedIn` are containment inverses (A `Contains` B ⇔ B `ContainedIn` A), and `SuperPart` / `SubPart` are part-of inverses (schema.org `hasPart` / `isPartOf` — A `SuperPart` B is the whole, B `SubPart` A the sub-part). The matcher does **not** resolve, invert, or transitively close the references — it compares the two things' relationship **sets** as opaque, distinct `(relation, thing_id)` keys (§6.6). A supporting signal, not an identifying field on its own. The enum is extensible (e.g. `SimilarTo`, `Replaces` / `ReplacedBy` later) — hence `#[non_exhaustive]`.

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
| `relationships_weight` | `f64` | `0.05` | `0.05` | `0.05` |
| `tags_weight` | `f64` | `0.05` | `0.05` | `0.05` |
| `use_phonetic_matching` | `bool` | `false` | `false` | `true` |
| `name_algorithm` | `SimilarityAlgorithm` | `Combined` | `Combined` | `Combined` |
| `strict_mode` | `bool` | `false` | `true` | `false` |

**`relationships_weight` and `tags_weight` landed in v0.7.0.** Every row in this table is live in the current release.

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
    pub relationships_score:             Option<f64>,
    pub tags_score:                      Option<f64>,
}
```

`MatchBreakdown` carries **12 score fields**, each `Option<f64>` — matching the `pub struct MatchBreakdown` above exactly (`relationships_score` and `tags_score` landed in v0.7.0, per §5.9.1/§5.9.2 and §6.6/§6.8). Per field: `Some(s)` means the field was scored, `s ∈ [0.0, 1.0]`. `None` means at least one side was absent / empty, so the field did not participate in the weighted sum. Downstream services MUST NOT discard the breakdown — it is the audit trail for the `score`.

Note for implementers: unlike `Thing`, `MatchBreakdown` does **not** carry `#[non_exhaustive]` (§7.3), so the v0.7.0 addition of `relationships_score` / `tags_score` was a breaking change under strict SemVer, permitted by this crate's pre-1.0 "minor bumps may break" policy (`agents/release.md`).

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

