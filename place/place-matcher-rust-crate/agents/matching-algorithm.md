# Matching algorithm — agent guide

The authoritative description lives in [`../spec.md`](../spec/index.md) §5–§7. This guide is the practitioner's view, with diagrams and worked examples drawn from `spec.md` §11.

## Strategies and surface

`MatchingEngine` exposes four entry points:

1. **`deterministic_match(&p1, &p2) -> bool`** — binary, fast, defensible. Use when a downstream system must see a clear yes/no.
2. **`match_places(&p1, &p2) -> MatchResult`** — score, threshold, per-field breakdown. Use to triage a single pair.
3. **`match_one_to_many(&query, candidates) -> Vec<MatchResult>`** — score the query against a candidate slice; output is parallel to the input.
4. **`rank_one_to_many(&query, candidates) -> Vec<(usize, MatchResult)>`** — same scoring, sorted by descending score with deterministic ascending-index tiebreak.

The engine is immutable and `Send + Sync`, so consumers can wrap the batch entry points in `rayon::par_iter` / `tokio::task::spawn_blocking` without changes to this crate. Blocking (candidate pre-filtering) is a consumer concern.

## Deterministic logic

`deterministic_match` returns `true` iff **any one** of the following holds:

- The two places share at least one `(scheme, value)` pair across their `place_ids` lists. Equality is structural — no per-scheme canonicalisation. `PlaceId::new` trims surrounding whitespace; otherwise the value is compared verbatim.
- Both places carry a `name` that normalises (via `Normalizer::normalize_name`) to the same string **AND** both carry an `address.postcode` that normalises (via `Normalizer::normalize_postcode`) to the same string.

`PlaceId` is **scheme-local**: a `(Google, "abc")` and an `(OsmNode, "abc")` never match. See `spec.md` §3.5 and §5.1.

## Probabilistic pipeline

```text
   p1, p2
     │
     ▼
┌───────────────────────────────┐
│ per-field scoring (§6)        │
│  name / phonetic / coords /   │
│  address / category / cc /    │
│  place_ids / phone / email    │
│  → MatchBreakdown             │
└───────────────────────────────┘
     │
     ▼
┌───────────────────────────────┐
│ weighted-sum reducer (§5.2.2) │
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

The renormalisation step is critical: missing data MUST NOT penalise the score. A place pair that only shares a name should score `1.0` on that name, not be dragged down by "missing coordinates" treated as a zero. (`spec.md` §5.4)

## Default weights

The canonical table lives in `spec.md` §7.1. Reproduced here for convenience:

| Component | Default weight | Algorithm |
|---|---|---|
| Name | `0.20` | Best-of-cartesian-product across `(name + alternate_names)` on both sides, via `MatchConfig::name_algorithm` (default `Combined` = `0.7 × JaroWinkler + 0.3 × Levenshtein`) after `normalize_name`. |
| Coordinates | `0.30`, scale `50.0` m | Gaussian decay `exp(-(d/s)^2)` over the Haversine distance (mean Earth radius `6_371_000` m). |
| Address | `0.10` | Weight-renormalised average across postcode (`0.5`), city (`0.3`), line 1 (`0.2`). Postcode is exact-equality on normalised form. City uses Jaro-Winkler on the normalised name. Line 1 runs `parse_address_line` on both sides; with house numbers present on both, `0.6 × jaro_winkler(street) + 0.4 × (house_number_a == house_number_b)`, else street similarity alone. An empty address scores neutral `0.5`. |
| Category | `0.10` | `1.0` if both set and structurally equal, `0.0` if both set and differ, `None` if either missing. |
| Country code | `0.05` | `1.0` if both set and equal after trim + ASCII lowercase, `0.0` otherwise, `None` if either missing. |
| Place IDs | `0.15` | `1.0` if both non-empty and any `(scheme, value)` pair is shared; `0.0` if both non-empty but no overlap; `None` if either empty. |
| Phone | `0.03` | Prefer E.164 equality via `Normalizer::normalize_phone_e164` against `MatchConfig::phone_default_country` (default `Some("GB")`). Fall back to `Normalizer::normalize_phone` equality when either side cannot be parsed to E.164. |
| Email | `0.02` | `Normalizer::normalize_email` (trim, lowercase, structural validation; optional Gmail dot- and `+tag`-folding behind `MatchConfig::gmail_dot_folding`) then exact equality. |
| Phonetic bonus | `+0.05`-weighted when gated | Soundex via `Normalizer::phonetic_code`, max equality across the name × alternate-name cartesian product. Bonus only — never lowers a score. |

## Presets

- **Default** — `match_threshold = 0.80`, all defaults from `MatchConfig::default`, phonetic off.
- **Strict** — `match_threshold = 0.95`, `strict_mode = true`. `is_match` also requires `deterministic_match`. `score` and `confidence` are unaffected.
- **Lenient** — `match_threshold = 0.65`, `use_phonetic_matching = true`.

## Coordinates scoring

```text
coordinates_score = exp(-(d / s)^2)
```

Contract (`Scorer::coordinates_score`, `spec.md` §6.3):

- `d == 0` → `1.0`.
- `d == s` → `1/e` (~`0.368`).
- `d == 3 × s` → ~`0.0001`.
- Negative `d`, non-positive `s`, or non-finite inputs → `0.0`.

The default scale `50` m is tuned for venue precision (cafes, monuments) where modern positioning is typically accurate to tens of metres. Widen the scale (e.g. `200` m) when matching coarsely-located records; tighten (e.g. `10` m) when chain branches on the same street must not collapse.

`Scorer::haversine_metres` uses the great-circle formula on a sphere of mean Earth radius `6_371_000` m. It is total over `f64`: non-finite inputs produce `NaN` (no panic). The implementation handles equator and date-line crossings correctly.

The matcher only produces a coordinates score when both records carry finite `latitude_as_decimal_degrees`, `longitude_as_decimal_degrees` values inside the conventional `[-90, 90]` / `[-180, 180]` ranges. Anything else yields `coordinates_score = None`.

## Best-of-cartesian-product name scoring

`collect_names(p)` gathers `p.name` plus `p.alternate_names`, skipping empty / whitespace-only strings. The matcher scores every pair `(n1, n2)` from `names(p1) × names(p2)` and takes the maximum.

This catches the common failure where the canonical name on one side appears as an alternate on the other (Big Ben vs Elizabeth Tower with "Big Ben" as an alternate — `spec.md` §11.2). It also handles translated endonyms (Eiffel Tower / La Tour Eiffel — `spec.md` §11.1) without manual disambiguation.

## When fields produce `None`

`MatchBreakdown` fields are `Option<f64>`:

- **`None`** — the field was not scored. Either the field was absent on at least one side, or the input was unparseable / out-of-range. The renormalisation rule means a `None` neither lifts nor lowers the overall score.
- **`Some(s)`** — the field was scored; `s` is in `[0.0, 1.0]`.

Per-field rules:

| Field | `None` when |
|---|---|
| `name_score` | Either side has no usable names. |
| `name_phonetic_score` | `use_phonetic_matching` is off, or either side has no usable names. |
| `coordinates_score` | Either side has missing / non-finite / out-of-range latitude or longitude. |
| `address_score` | Either side has no `address` value at all. |
| `category_score` | Either side has `category = None`. |
| `country_code_score` | Either side has `country_code_as_iso_3166_1_alpha_2 = None`. |
| `place_ids_score` | Either side has an empty `place_ids`. |
| `phone_score` | Either side has `phone = None`. |
| `email_score` | Either side has `email = None`, or either side fails to canonicalise. |

## Strict mode

When `MatchConfig::strict_mode = true`, the engine still computes the same probabilistic `score` and `confidence`, but it tightens the binary `is_match` decision to **also require `deterministic_match`**. A fuzzy near-match that clears the threshold but has no shared place ID and no normalised name + postcode agreement is rejected as `is_match = false`.

Consumers reading `MatchBreakdown` directly are unaffected — every per-field score, the overall `score`, and the `Confidence` band are identical across strict and non-strict configurations. Strict mode is a presentation-layer guard for the `is_match` boolean.

## Worked examples

The full set of worked examples lives in `spec.md` §11. Two highlights:

### Eiffel Tower / La Tour Eiffel (probabilistic, `spec.md` §11.1)

```rust
let a = Place::builder()
    .name("Eiffel Tower")
    .add_alternate_name("La Tour Eiffel")
    .latitude_as_decimal_degrees(48.858_222).longitude_as_decimal_degrees(2.294_500)
    .category(PlaceCategory::Monument)
    .country_code_as_iso_3166_1_alpha_2("FR")
    .build();
let b = Place::builder()
    .name("Tour Eiffel")
    .latitude_as_decimal_degrees(48.858_3).longitude_as_decimal_degrees(2.294_5)
    .category(PlaceCategory::Monument)
    .country_code_as_iso_3166_1_alpha_2("FR")
    .build();
let r = MatchingEngine::default_config().match_places(&a, &b);
assert!(r.is_match);
```

### Two Starbucks branches (negative, `spec.md` §11.3)

```rust
let manhattan = Place::builder()
    .name("Starbucks")
    .latitude_as_decimal_degrees(40.7589).longitude_as_decimal_degrees(-73.9851)
    .category(PlaceCategory::Cafe)
    .country_code_as_iso_3166_1_alpha_2("US")
    .build();
let los_angeles = Place::builder()
    .name("Starbucks")
    .latitude_as_decimal_degrees(34.0522).longitude_as_decimal_degrees(-118.2437)
    .category(PlaceCategory::Cafe)
    .country_code_as_iso_3166_1_alpha_2("US")
    .build();
let r = MatchingEngine::default_config().match_places(&manhattan, &los_angeles);
assert!(!r.is_match);
```

Same name, same category, same country — but `coordinates_score ≈ 0.0` at ~3940 km separation. The dominating coordinates weight drags the renormalised total well below `0.80`.

## Edge cases to remember

- `PlaceId::new` returns `None` for an empty value. The caller's place then has nothing to compare for that entry.
- An address with no comparable sub-components returns `0.5` (neutral), not `0.0`. This is by design — we don't punish places for empty address data.
- The phonetic bonus never lowers a score. If a place pair has a strong direct name match but mismatched Soundex codes (e.g. very different transliterations), the bonus simply does not fire.
- The score is `0.0` when no field participates (e.g. one side has only `phone`, the other has only `email`).

## When you change weights

- The default-weight table is part of the documented behaviour (spec, README). Update both when you change defaults.
- Add a `### Behaviour Change` subsection under the next CHANGELOG entry.
- Bump the minor version (pre-1.0 minor bumps may carry breaking behaviour by convention).

## Tuning guidance

| Symptom | Knob to move |
|---|---|
| Too many false positives in the top of the ranking | Raise `match_threshold`; raise `name_weight` or `place_ids_weight`; consider `strict()` preset. |
| Too many false negatives near the threshold | Lower `match_threshold`; widen `coordinates_scale_metres`; turn on `use_phonetic_matching`; consider `lenient()` preset. |
| Chain branches on the same street collapsing into one match | Tighten `coordinates_scale_metres` (e.g. `10.0`); raise `coordinates_weight`. |
| Coarsely-located records under-scoring on coordinates | Widen `coordinates_scale_metres` (e.g. `200.0`); lower `coordinates_weight`. |
| Category disagreement not being punished enough | Raise `category_weight`. |
| Cross-country phone collisions | Set `phone_default_country` to the predominant jurisdiction, or `None` to refuse to guess. |
| Gmail localpart variants not matching | Set `gmail_dot_folding = true`. |

## Open algorithm questions

See [`../spec.md`](../spec/index.md) §10 for the live list (OQ-A through OQ-H). When you have an opinion, propose a resolution in `spec.md` as a PR, not as a unilateral code change.
