# Matching algorithm — agent guide

The authoritative description lives in [`../spec.md`](../spec/index.md) §5–§7. This guide is the practitioner's view, with diagrams and worked examples drawn from `spec.md` §11.

## Strategies and surface

`MatchingEngine` exposes four entry points:

1. **`deterministic_match(&t1, &t2) -> bool`** — binary, fast, defensible. Use when a downstream system must see a clear yes/no.
2. **`match_things(&t1, &t2) -> MatchResult`** — score, threshold, per-field breakdown. Use to triage a single pair.
3. **`match_one_to_many(&query, candidates) -> Vec<MatchResult>`** — score the query against a candidate slice; output is parallel to the input.
4. **`rank_one_to_many(&query, candidates) -> Vec<(usize, MatchResult)>`** — same scoring, sorted by descending score with deterministic ascending-index tiebreak.

The engine is immutable and `Send + Sync`, so consumers can wrap the batch entry points in `rayon::par_iter` / `tokio::task::spawn_blocking` without changes to this crate. Blocking (candidate pre-filtering) is a consumer concern.

## Deterministic logic

`deterministic_match` returns `true` iff **any one** of the following holds:

- The two things share at least one `(property_id, value)` pair across their `identifiers` lists. Equality is structural — no per-scheme canonicalisation. `Identifier::new` trims surrounding whitespace; otherwise the value is compared verbatim.
- The two things share at least one `same_as` URL after `Normalizer::normalize_url`.
- Both things have a `url` and the two URLs are equal after `Normalizer::normalize_url`.

`Identifier` is **scheme-local**: a `("wikidata", "Q243")` and a `("viaf", "Q243")` never match. See `spec.md` §3.3 and §5.1.

## Probabilistic pipeline

```text
   t1, t2
     │
     ▼
┌───────────────────────────────┐
│ per-field scoring (§6)        │
│  name / phonetic / desc /     │
│  disambig / identifiers /     │
│  url / same_as / image /      │
│  main_entity_of_page /        │
│  additional_types             │
│  → MatchBreakdown             │
└───────────────────────────────┘
     │
     ▼
┌───────────────────────────────┐
│ weighted-sum reducer (§5.10)  │
│  Σ score×weight / Σ weight    │
│  + 0.05-weighted phonetic     │
│    bonus iff phonetic > 0.9   │
└───────────────────────────────┘
     │
     ▼
┌───────────────────────────────┐
│ score, confidence, is_match   │
│ (strict mode AND-s with       │
│  deterministic_match)         │
└───────────────────────────────┘
```

The renormalisation step is critical: missing data MUST NOT penalise the score. A thing pair that only shares a name should score `1.0` on that name, not be dragged down by "missing description" treated as a zero. (`spec.md` §5.10)

## Default weights

The canonical table lives in `spec.md` §3.4. Reproduced here for convenience:

| Component | Default weight | Algorithm |
|---|---|---|
| Name | `0.30` | Best-of-cartesian-product across `(name + alternate_names)` on both sides, via `MatchConfig::name_algorithm` (default `Combined` = `0.6 × JaroWinkler + 0.4 × Levenshtein`) after `normalize_name`. |
| Description | `0.10` | `Scorer::combined_similarity` on `description` after `normalize_text`. |
| Disambiguating description | `0.05` | `Scorer::combined_similarity` on `disambiguating_description` after `normalize_text`. |
| Identifiers | `0.25` | `1.0` if both non-empty and any `(property_id, value)` pair is shared; `0.0` if both non-empty but no overlap; `None` if either empty. |
| URL | `0.05` | Exact equality after `normalize_url`. |
| sameAs | `0.15` | Jaccard set similarity over `same_as` URLs after `normalize_url`. `None` only when both empty. |
| Image | `0.03` | Exact equality after `normalize_url`. |
| mainEntityOfPage | `0.02` | Exact equality after `normalize_url`. |
| Additional types | `0.05` | Jaccard set similarity over `additional_types` URIs after `normalize_url`. `None` only when both empty. |
| Phonetic bonus | `+0.05`-weighted when gated | Soundex via `Normalizer::phonetic_code`, max equality across the name × alternate-name cartesian product. Bonus only — never lowers a score. Fires only when score > `0.9`. |

## Presets

- **Default** — `match_threshold = 0.80`, all defaults from `MatchConfig::default`, phonetic off.
- **Strict** — `match_threshold = 0.95`, `strict_mode = true`. `is_match` also requires `deterministic_match`. `score` and `confidence` are unaffected.
- **Lenient** — `match_threshold = 0.65`, `use_phonetic_matching = true`.

## URL scoring

`Normalizer::normalize_url` lowercases the scheme and host and drops a trailing slash on the path root. Two URLs are considered equal under URL scoring iff they normalise byte-for-byte equal.

This means:

- `HTTPS://Example.ORG/` and `https://example.org` are equal.
- `https://example.org/path` and `https://example.org/path/` are NOT equal — the trailing-slash trim applies only to the path root, not to internal segments.
- `https://example.org/?utm_source=foo` and `https://example.org/` are NOT equal — query strings are not stripped.
- The crate does NOT perform DNS-aware canonicalisation, percent-encoding canonicalisation, or punycode decoding. Consumers MUST canonicalise upstream if those equivalences matter.

`same_as_score` and `additional_types_score` are **Jaccard** similarities over the URL sets: `|A ∩ B| / |A ∪ B|`. They are `None` only when both sides are empty.

## Best-of-cartesian-product name scoring

`collect_names(t)` gathers `t.name` plus `t.alternate_names`, skipping empty / whitespace-only strings. The matcher scores every pair `(n1, n2)` from `names(t1) × names(t2)` and takes the maximum.

This catches the common failure where the canonical name on one side appears as an alternate on the other (Eiffel Tower / La Tour Eiffel, where one side carries "Eiffel Tower" as the primary and the other as the alternate). It also handles translations and endonym variants without manual disambiguation.

## When fields produce `None`

`MatchBreakdown` fields are `Option<f64>`:

- **`None`** — the field was not scored. Either the field was absent on at least one side, or both list fields were empty. The renormalisation rule means a `None` neither lifts nor lowers the overall score.
- **`Some(s)`** — the field was scored; `s` is in `[0.0, 1.0]`.

Per-field rules:

| Field | `None` when |
|---|---|
| `name_score` | Either side has no usable names. |
| `name_phonetic_score` | `use_phonetic_matching` is off, or either side has no usable names. |
| `description_score` | Either side has `description = None`. |
| `disambiguating_description_score` | Either side has `disambiguating_description = None`. |
| `identifiers_score` | Either side has an empty `identifiers` list. |
| `url_score` | Either side has `url = None`. |
| `same_as_score` | BOTH sides have an empty `same_as` list. |
| `image_score` | Either side has `image = None`. |
| `main_entity_of_page_score` | Either side has `main_entity_of_page = None`. |
| `additional_types_score` | BOTH sides have an empty `additional_types` list. |

Note the **single-side / both-sides asymmetry** for the set-similarity fields (`same_as_score`, `additional_types_score`): if one side has entries and the other has none, the score is `Some(0.0)` (intersection is empty), which is intentional — an empty alternate vocabulary on one side cannot match anything from the other, so it gets a real zero, not a "skip".

## Strict mode

When `MatchConfig::strict_mode = true`, the engine still computes the same probabilistic `score` and `confidence`, but it tightens the binary `is_match` decision to **also require `deterministic_match`**. A fuzzy near-match that clears the threshold but shares no identifier, no `same_as` URL, and no normalised `url` is rejected as `is_match = false`.

Consumers reading `MatchBreakdown` directly are unaffected — every per-field score, the overall `score`, and the `Confidence` band are identical across strict and non-strict configurations. Strict mode is a presentation-layer guard for the `is_match` boolean.

## Worked examples

The full set of worked examples lives in `spec.md` §11. Two highlights:

### Eiffel Tower / La Tour Eiffel (probabilistic, `spec.md` §11.1)

```rust
let a = Thing::builder()
    .name("Eiffel Tower")
    .add_alternate_name("La Tour Eiffel")
    .url("https://www.toureiffel.paris/")
    .add_same_as("https://www.wikidata.org/wiki/Q243")
    .add_additional_type("https://schema.org/Landmark")
    .build();
let b = Thing::builder()
    .name("Tour Eiffel")
    .url("https://www.toureiffel.paris/")
    .add_same_as("https://www.wikidata.org/wiki/Q243")
    .build();
let r = MatchingEngine::default_config().match_things(&a, &b);
assert!(r.is_match);
```

The shared `url` AND shared `same_as` URL both fire deterministic-match independently; the probabilistic path scores name (best-of-pair), URL, and same_as well above the threshold.

### Two different books, same title prefix (negative)

```rust
let novel = Thing::builder()
    .name("The Brothers Karamazov")
    .add_identifier(Identifier::new("isbn", "9780374528379").unwrap())
    .build();
let film = Thing::builder()
    .name("The Brothers")
    .add_identifier(Identifier::new("imdb", "tt0073700").unwrap())
    .build();
let r = MatchingEngine::default_config().match_things(&novel, &film);
assert!(!r.is_match);
```

The names share a prefix but the identifiers are scheme-local and the values differ, so `identifiers_score = Some(0.0)`. With only name + identifiers scoring, the weighted average lands well below `0.80`.

## Edge cases to remember

- `Identifier::new` returns `None` for an empty value or property_id. The caller's thing then has nothing to compare for that entry.
- `same_as_score` and `additional_types_score` are `None` only when BOTH sides are empty. A one-sided empty list still scores `Some(0.0)`. This is by design — an empty alternate vocabulary on one side genuinely cannot match anything.
- The phonetic bonus never lowers a score. If a thing pair has a strong direct name match but mismatched Soundex codes (e.g. very different transliterations), the bonus simply does not fire.
- The score is `0.0` when no field participates (e.g. one side has only `name`, the other has only `image`).

## When you change weights

- The default-weight table is part of the documented behaviour (spec, README). Update both when you change defaults.
- Add a `### Behaviour Change` subsection under the next CHANGELOG entry.
- Bump the minor version (pre-1.0 minor bumps may carry breaking behaviour by convention).

## Open algorithm questions

See [`../spec.md`](../spec/index.md) §10 for the live list (OQ-A through OQ-D). When you have an opinion, propose a resolution in `spec.md` as a PR, not as a unilateral code change.
