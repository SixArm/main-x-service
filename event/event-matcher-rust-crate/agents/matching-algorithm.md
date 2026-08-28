# Matching algorithm — agent guide

The authoritative description lives in [`../spec.md`](../spec/index.md) §5–§7. This guide is the practitioner's view, with diagrams and worked examples drawn from `spec.md` §11.

## Strategies and surface

`MatchingEngine` exposes four entry points:

1. **`deterministic_match(&e1, &e2) -> bool`** — binary, fast, defensible. Use when a downstream system must see a clear yes/no.
2. **`match_events(&e1, &e2) -> MatchResult`** — score, threshold, per-field breakdown. Use to triage a single pair.
3. **`match_one_to_many(&query, candidates) -> Vec<MatchResult>`** — score the query against a candidate slice; output is parallel to the input.
4. **`rank_one_to_many(&query, candidates) -> Vec<(usize, MatchResult)>`** — same scoring, sorted by descending score with deterministic ascending-index tiebreak.

The engine is immutable and `Send + Sync`, so consumers can wrap the batch entry points in `rayon::par_iter` / `tokio::task::spawn_blocking` without changes to this crate. Blocking (candidate pre-filtering) is a consumer concern.

## Deterministic logic

`deterministic_match` returns `true` iff **any one** of the following holds:

- The two events share at least one `(scheme, value)` pair across their `event_ids` lists. Equality is structural — no per-scheme canonicalisation. `EventId::new` trims surrounding whitespace; otherwise the value is compared verbatim.
- Both events carry a `name` that normalises (via `Normalizer::normalize_name`) to the same string **AND** both carry a `start_date` that parses (via `Normalizer::parse_iso8601_unix_seconds`) to the same Unix instant. Textually different offsets of the same instant agree — `"2024-09-10T09:00:00Z"` matches `"2024-09-10T11:00:00+02:00"`.

`EventId` is **scheme-local**: an `(Eventbrite, "abc")` and a `(Meetup, "abc")` never match. See `spec.md` §3.8 and §5.1.

## Probabilistic pipeline

```text
   e1, e2
     │
     ▼
┌───────────────────────────────┐
│ per-field scoring (§6)        │
│  name / phonetic / start_date │
│  / end_date / location /      │
│  category / country_code /    │
│  event_ids / organizer /      │
│  performers / url /           │
│  relationships / tags         │
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

The renormalisation step is critical: missing data MUST NOT penalise the score. An event pair that only shares a name should score `1.0` on that name, not be dragged down by "missing location" treated as a zero. (`spec.md` §5.4)

## Default weights

The canonical table lives in `spec.md` §7.1. Reproduced here for convenience:

| Component | Default weight | Algorithm |
|---|---|---|
| Name | `0.20` | Best-of-cartesian-product across `(name + alternate_names)` on both sides, via `MatchConfig::name_algorithm` (default `Combined` = `0.7 × JaroWinkler + 0.3 × Levenshtein`) after `normalize_name`. |
| Start date | `0.25`, scale `3600.0` s | Gaussian decay `exp(-(d/s)^2)` over the absolute seconds difference between the two `start_date` instants (both parsed via `Normalizer::parse_iso8601_unix_seconds`). |
| End date | `0.05` | Same Gaussian shape and **same scale** as start date — only the weight differs. |
| Location | `0.15` | Weight-renormalised blend, sub-weights coordinates `0.5`, address `0.3`, venue name `0.15`, virtual URL `0.05` (see below). |
| Category | `0.08` | `1.0` if both set and structurally equal, `0.0` if both set and differ, `None` if either missing. |
| Country code | `0.04` | `1.0` if both set and equal after trim + ASCII lowercase, `0.0` otherwise, `None` if either missing. |
| Event IDs | `0.15` | `1.0` if both non-empty and any `(scheme, value)` pair is shared; `0.0` if both non-empty but no overlap; `None` if either empty. |
| Organizer | `0.04` | `Scorer::combined_similarity` on `normalize_name(organizer)` from each side. `None` if either side has no organizer. |
| Performers | `0.02` | Best-of cartesian product across performer lists after `normalize_name`. `None` if either list is empty. |
| URL | `0.02` | Exact equality after trimming whitespace. `None` if either is absent. |
| Relationships | `0.05` | Typed-set Jaccard `\|A ∩ B\| / \|A ∪ B\|` over `(relation, event_id)` pairs (`RelationshipRef` / `RelationKind`: `Outer` / `Inner` / `ImmediatelyBefore` / `ImmediatelyAfter`). `None` if either side has no relationships. A supporting signal, never identifying. See `spec.md` §6.11. |
| Tags | `0.05` | Plain set Jaccard `\|A ∩ B\| / \|A ∪ B\|` over the case-insensitively normalised (`str::to_lowercase`) tag sets. `None` if either side has no tags. A supporting signal, never identifying. See `spec.md` §6.12. |
| Phonetic bonus | `+0.05`-weighted when gated | Soundex via `Normalizer::phonetic_code`, max equality across the name × alternate-name cartesian product. Bonus only — never lowers a score. Only computed when `MatchConfig::use_phonetic_matching = true`. |

## Presets

- **Default** — `match_threshold = 0.80`, all defaults from `MatchConfig::default`, phonetic off.
- **Strict** — `match_threshold = 0.95`, `strict_mode = true`. `is_match` also requires `deterministic_match`. `score` and `confidence` are unaffected.
- **Lenient** — `match_threshold = 0.65`, `use_phonetic_matching = true`.

## Location scoring

`location_score` is `None` only when either side has `Event.location = None`. When both sides carry a `Location`, the sub-score is a weight-renormalised average:

- **Coordinates** (`0.5`) — `Scorer::coordinates_score(Scorer::haversine_metres(...), scale)`, `scale = MatchConfig::coordinates_scale_metres` (default `100.0` m). Participates only when both sides carry finite latitude **and** longitude inside `[-90, 90]` / `[-180, 180]`.
- **Address** (`0.3`) — weight-renormalised blend across postcode (`0.5`), city (`0.3`), line 1 (`0.2`), same shape as the address sub-scorer described below.
- **Venue name** (`0.15`) — `Scorer::combined_similarity` on `normalize_name(venue_name)`.
- **Virtual URL** (`0.05`) — exact equality after trim.

If no sub-component is populated on both sides, `location_score` is neutral `0.5`, not `0.0` — an empty `Location` on both sides is not evidence of disagreement.

## Coordinates scoring

```text
coordinates_score = exp(-(d / s)^2)
```

Contract (`Scorer::coordinates_score`, `spec.md` §6.4):

- `d == 0` → `1.0`.
- `d == s` → `1/e` (~`0.368`).
- `d == 3 × s` → ~`0.0001`.
- Negative `d`, non-positive `s`, or non-finite inputs → `0.0`.

The default scale `100` m suits single-venue, clock-scheduled events. Widen the scale for multi-building venues or stadium-scale fixtures; tighten it when multi-room venues on the same site must not collapse (`spec.md` §10 OQ-H tracks whether the default should vary by `EventCategory`).

`Scorer::haversine_metres` uses the great-circle formula on a sphere of mean Earth radius `6_371_000` m. It is total over `f64`: non-finite inputs produce `NaN` (no panic). The implementation handles equator and date-line crossings correctly.

## Temporal (start/end date) scoring

```text
start_date_score = exp(-(d / s)^2)   where d = |seconds_between(t1, t2)|
```

Same Gaussian-decay shape as coordinates, but over time: `s = MatchConfig::start_date_scale_seconds` (default `3600.0`, one hour), shared by both the `start_date` and `end_date` components — only the *weights* (`0.25` vs `0.05`) differ. `Scorer::seconds_between` parses both sides via `Normalizer::parse_iso8601_unix_seconds`; a missing or unparseable date on either side yields `None`, not `0.0`.

There is **no window-overlap** scoring in this crate — `start_date` and `end_date` are scored independently by endpoint (Gaussian-decay) proximity, not by the overlap fraction of the two `[start, end]` intervals. Whether to add overlap-fraction scoring is tracked as `spec.md` §10 OQ-C; it is not implemented.

## Best-of-cartesian-product name scoring

`collect_names(e)` gathers `e.name` plus `e.alternate_names`, skipping empty / whitespace-only strings. The matcher scores every pair `(n1, n2)` from `names(e1) × names(e2)` and takes the maximum.

This catches the common failure where the canonical name on one side appears as an alternate on the other (`"Glastonbury Festival 2024"` vs `"Glasto 2024"` with the full name carried as an alternate — `spec.md` §11.1).

## When fields produce `None`

`MatchBreakdown` fields are `Option<f64>`:

- **`None`** — the field was not scored. Either the field was absent on at least one side, or the input was unparseable / out-of-range. The renormalisation rule means a `None` neither lifts nor lowers the overall score.
- **`Some(s)`** — the field was scored; `s` is in `[0.0, 1.0]`.

Per-field rules:

| Field | `None` when |
|---|---|
| `name_score` | Either side has no usable names. |
| `name_phonetic_score` | `use_phonetic_matching` is off, or either side has no usable names. |
| `start_date_score` / `end_date_score` | Either side is missing, or fails to parse as ISO 8601, for the corresponding field. |
| `location_score` | Either side has `location = None`. (An empty-but-present `Location` still participates, degrading to neutral `0.5`.) |
| `category_score` | Either side has `category = None`. |
| `country_code_score` | Either side has `country_code_as_iso_3166_1_alpha_2 = None`. |
| `event_ids_score` | Either side has an empty `event_ids`. |
| `organizer_score` | Either side has `organizer = None`. |
| `performers_score` | Either side has an empty `performers`. |
| `url_score` | Either side has `url = None`. |

## Strict mode

When `MatchConfig::strict_mode = true`, the engine still computes the same probabilistic `score` and `confidence`, but it tightens the binary `is_match` decision to **also require `deterministic_match`**. A fuzzy near-match that clears the threshold but has no shared event ID and no normalised-name + same-start-instant agreement is rejected as `is_match = false`.

Consumers reading `MatchBreakdown` directly are unaffected — every per-field score, the overall `score`, and the `Confidence` band are identical across strict and non-strict configurations. Strict mode is a presentation-layer guard for the `is_match` boolean.

## Worked examples

The full set of worked examples lives in `spec.md` §11. Two highlights:

### Glastonbury Festival / Glasto (probabilistic, `spec.md` §11.1)

```rust
let a = Event::builder()
    .name("Glastonbury Festival 2024")
    .add_alternate_name("Glasto 2024")
    .start_date("2024-06-26T09:00:00Z")
    .end_date("2024-06-30T23:59:00Z")
    .category(EventCategory::Festival)
    .country_code_as_iso_3166_1_alpha_2("GB")
    .build();
let b = Event::builder()
    .name("Glasto 2024")
    .start_date("2024-06-26T09:15:00Z")
    .end_date("2024-06-30T23:59:00Z")
    .category(EventCategory::Festival)
    .country_code_as_iso_3166_1_alpha_2("GB")
    .build();
let r = MatchingEngine::default_config().match_events(&a, &b);
assert!(r.is_match); // score ≈ 0.976, Confidence::High
```

### Strict mode rejects a fuzzy-only match (negative, `spec.md` §11.3)

```rust
let a = Event::builder()
    .name("Cafe Centrale Concert")
    .start_date("2024-09-10T09:00:00Z")
    .category(EventCategory::MusicEvent)
    .build();
let b = Event::builder()
    .name("Cafe Central Concert")   // close, but not equal under normalisation
    .start_date("2024-09-10T09:00:00Z")
    .category(EventCategory::MusicEvent)
    .build();
let strict = MatchingEngine::new(MatchConfig::strict());
let r = strict.match_events(&a, &b);
assert!(!r.is_match); // requires deterministic_match too; names differ, no shared event id
```

Same start instant, near-identical names — but no shared `EventId` and the normalised names differ, so `deterministic_match` returns `false` and strict mode rejects the pair even though the probabilistic score alone clears the default threshold.

## Edge cases to remember

- `EventId::new` returns `None` for an empty value. The caller's event then has nothing to compare for that entry.
- A `Location` with no comparable sub-components returns `0.5` (neutral), not `0.0` — but an `Event.location` of `None` on either side skips the component entirely (`None`, not `0.5`). These are different states; don't conflate them.
- The phonetic bonus never lowers a score. If an event pair has a strong direct name match but mismatched Soundex codes (e.g. very different transliterations), the bonus simply does not fire.
- The score is `0.0` when no field participates (e.g. one side has only `organizer`, the other only `url`).

## When you change weights

- The default-weight table is part of the documented behaviour (spec, README). Update both when you change defaults.
- Add a `### Behaviour Change` subsection under the next CHANGELOG entry.
- Bump the minor version (pre-1.0 minor bumps may carry breaking behaviour by convention).

## Open algorithm questions

See [`../spec.md`](../spec/index.md) §10 for the live list (OQ-A through OQ-I). When you have an opinion, propose a resolution in `spec.md` as a PR, not as a unilateral code change.
