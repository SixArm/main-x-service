# Place matcher Rust Crate

A Rust library for deciding whether two records describe the same geographic place.

> **Documentation index:** [`index.md`](./index.md) is the top-level map of every doc in this repo (spec, AGENTS guides, CHANGELOG, examples). Start there if you're new.

## What it does

`place-matcher` compares pairs of [`Place`] records — landmarks (Eiffel Tower, Big Ben), natural features (Yosemite, Lake Como), chain branches (Starbucks at a given address), administrative areas (cities, neighbourhoods) — and tells you whether they refer to the same place. It is built for de-duplication and record linkage across geographic data sources that disagree on names, encodings, coordinates, and identifier schemes.

The crate provides two strategies behind one engine:

- **Deterministic** — a hard `bool` from a shared external place ID (Google, OSM, Wikidata, …) or from an identical normalised name plus identical normalised postcode.
- **Probabilistic** — a weight-renormalised score in `[0.0, 1.0]` over name, coordinates (Haversine distance with a Gaussian decay), address, category, country code, external IDs, phone, and email, with a per-field [`MatchBreakdown`] so every decision is auditable.

The library is pure: no IO, no clocks, no RNGs, `#![forbid(unsafe_code)]`, `Send + Sync`. It is suitable as a leaf dependency under web servers, batch jobs, or notebooks.

## Installation

```toml
[dependencies]
place-matcher = "0.4"
```

## Quick start — probabilistic match

```rust
use place_matcher::{MatchingEngine, Place};

let a = Place::builder()
    .name("Eiffel Tower")
    .latitude(48.858_222)
    .longitude(2.294_500)
    .build();

let b = Place::builder()
    .name("La Tour Eiffel")
    .add_alternate_name("Eiffel Tower")
    .latitude(48.858_3)
    .longitude(2.294_5)
    .build();

let engine = MatchingEngine::default_config();
let result = engine.match_places(&a, &b);

assert!(result.is_match);
println!("score = {:.2}", result.score);
println!("name  = {:?}", result.breakdown.name_score);
println!("coord = {:?}", result.breakdown.coordinates_score);
```

The `MatchBreakdown` carries one `Option<f64>` per scored field; a `None` means the field was absent on at least one side and so did not contribute. Missing fields neither raise nor lower the score.

## Quick start — deterministic match

A shared external place ID (any `(scheme, value)` pair across the two records) is enough on its own:

```rust
use place_matcher::{MatchingEngine, Place, PlaceId, PlaceIdScheme};

let id = PlaceId::new(PlaceIdScheme::Wikidata, "Q243").unwrap();

let a = Place::builder().name("Eiffel Tower").add_place_id(id.clone()).build();
let b = Place::builder().name("Tour Eiffel").add_place_id(id).build();

let engine = MatchingEngine::default_config();
assert!(engine.deterministic_match(&a, &b));
```

Identical normalised name plus identical normalised postcode is also accepted as a deterministic match (useful when no shared external ID is available).

## The `Place` model

| Field | Type | Purpose |
|---|---|---|
| `name` | `Option<String>` | Primary canonical name. |
| `alternate_names` | `Vec<String>` | Aliases, endonyms, translations. The matcher takes the best score across the cartesian product. |
| `latitude`, `longitude` | `Option<f64>` | Decimal degrees. Non-finite or out-of-range values are treated as missing by the coordinates scorer. |
| `category` | `Option<PlaceCategory>` | Coarse-grained category (Hotel, Restaurant, Museum, …). `#[non_exhaustive]`. |
| `place_ids` | `Vec<PlaceId>` | External scheme-scoped identifiers (Google, OSM, Wikidata, GeoNames, Foursquare, Here, Mapbox, Other). |
| `address` | `Option<Address>` | Postal address. Partial addresses (postcode only, say) are first-class. |
| `phone` | `Option<String>` | Contact phone. Compared after E.164 normalisation when possible. |
| `email` | `Option<String>` | Contact email. Compared after canonical-form normalisation. |
| `local_id` | `Option<String>` | Originating-system identifier. Stored for round-trip but **not** scored — different sources may issue colliding values. |
| `altitude_as_metre` | `Option<f64>` | Object-above-surface height (rooftop, cruise level). Not scored. |
| `elevation_as_metre` | `Option<f64>` | Ground-surface reference height (mountain summit). Not scored. |
| `area_as_metre_2` | `Option<f64>` | Footprint in m². Not scored. |
| `country_code_as_iso_3166_1_alpha_2` | `Option<String>` | ISO 3166-1 alpha-2 (`"GB"`, `"US"`, `"FR"`). Compared case-insensitively. |
| `maximum_capacity_count` | `Option<u32>` | Legal / seating capacity. Not scored. |

Build records via the fluent [`Place::builder`]; all setters accept `impl Into<String>`.

## The match pipeline

1. Each scoring component yields `Some(score)` in `[0.0, 1.0]` or `None` (missing on at least one side).
2. The weighted sum runs over components that scored.
3. The sum of participating weights divides through — **renormalisation** ensures missing fields do not penalise.
4. A phonetic-name match (Soundex on normalised names) adds a `0.05`-weighted bonus when the gating phonetic score exceeds `0.9`, only when `use_phonetic_matching` is on. The bonus never lowers a score.
5. `is_match = score >= match_threshold` (strict mode additionally requires `deterministic_match`).
6. `confidence = Confidence::from_score(score)` — bands are fixed (`>= 0.90` High, `>= 0.75` Medium, else Low) and **independent** of `match_threshold`.

### Default weights

| Component | Weight | Notes |
|---|---|---|
| Name | `0.20` | Best of cartesian product across primary + alternates, via the configured `SimilarityAlgorithm` (default `Combined` = 0.7 × Jaro-Winkler + 0.3 × Levenshtein). |
| Coordinates | `0.30` | Gaussian decay `exp(-(d/s)^2)` over the Haversine distance. Default scale `s = 50` m. |
| Address | `0.10` | Postcode `0.5`, city `0.3`, line 1 `0.2` — weight-renormalised across populated sub-fields; empty address scores neutral `0.5`. |
| Category | `0.10` | `1.0` if equal, `0.0` if both set and differ, `None` if either missing. |
| Country code | `0.05` | Case-insensitive equality after trim. |
| Place IDs | `0.15` | `1.0` if any `(scheme, value)` pair is shared, `0.0` if both non-empty but no overlap, `None` if either empty. |
| Phone | `0.03` | E.164 equality preferred; falls back to the legacy national-significant form when E.164 cannot be derived. |
| Email | `0.02` | Canonical-form equality; optional Gmail dot-and-`+tag` folding. |
| Phonetic bonus | `+0.05` when gated | Bonus only — never lowers a score. |

## Configuration presets

```rust
use place_matcher::{MatchConfig, MatchingEngine};

let strict  = MatchingEngine::new(MatchConfig::strict());  // threshold 0.95, requires deterministic
let default = MatchingEngine::default_config();            // threshold 0.80
let lenient = MatchingEngine::new(MatchConfig::lenient()); // threshold 0.65, phonetic on
```

- **Default (0.80)** — balanced for everyday de-duplication.
- **Strict (0.95)** — for downstream systems that must rely on the answer; `is_match` additionally requires `deterministic_match`. `score` and `confidence` are unaffected.
- **Lenient (0.65)** — for triaging large candidate sets where false negatives are costlier than false positives.

Every field of `MatchConfig` is overridable; the config is `Serialize + Deserialize` (with `#[serde(default)]`) so tunings can live in a file:

```rust
use place_matcher::MatchConfig;

let cfg: MatchConfig = serde_json::from_str(r#"{
    "match_threshold": 0.85,
    "coordinates_scale_metres": 25.0
}"#).unwrap();
```

## Batch scoring

```rust
use place_matcher::{MatchingEngine, Place};

let engine = MatchingEngine::default_config();
let query = Place::builder().name("Eiffel Tower").build();
let candidates = vec![
    Place::builder().name("Big Ben").build(),
    Place::builder().name("Eiffel Tower").build(),
    Place::builder().name("Statue of Liberty").build(),
];

// Parallel to the input slice:
let results = engine.match_one_to_many(&query, &candidates);

// Sorted by descending score, deterministic tiebreak on original index:
let ranked = engine.rank_one_to_many(&query, &candidates);
let (best_idx, best) = &ranked[0];
println!("best is candidate[{best_idx}] with score {:.2}", best.score);
```

The engine is `Send + Sync`. Wrap calls in `rayon::par_iter` (or any parallelism primitive) without changes to this crate. Candidate pre-filtering — Soundex prefix blocking, country-code blocking, postcode outward-code blocking — is intentionally a consumer concern.

## Geographic primitives

`Scorer` exposes the geographic helpers the engine uses internally:

```rust
use place_matcher::Scorer;

let d = Scorer::haversine_metres(51.507_22, -0.127_5, 48.853_0, 2.349_2);
let s = Scorer::coordinates_score(d, 50.0);
println!("London-Paris: {:.1} km, score @ scale=50 m: {s:.6}", d / 1000.0);
```

`Scorer::haversine_metres` is total over `f64` (non-finite inputs return `NaN`, no panic) and works across the equator and the date line. `Scorer::coordinates_score` returns `exp(-(d/s)^2)` clamped to `[0.0, 1.0]`; pathological inputs (negative distance, non-positive scale, non-finite) return `0.0`.

## Determinism and safety

- **`#![forbid(unsafe_code)]`** at the crate root.
- **No IO.** The library does not read files, open sockets, or log.
- **No clocks, no RNGs, no environment variables.** Same inputs always produce the same outputs.
- **No panics** in library code; every fallible parser returns `None` and every fallible operation returns `Result`.
- **`Send + Sync`.** Engines are immutable after construction and cheap to clone.
- **Serde-clean.** Every public data type round-trips through `serde_json` (and any other `serde` format).

## Limitations / out of scope

- **Not a geocoder.** This crate does not turn addresses into coordinates or coordinates into addresses.
- **Not a router.** No path-finding, no distance-along-a-network — only great-circle (Haversine) distance.
- **Not an address-parsing service.** `Normalizer::parse_address_line` is a best-effort structural decomposition for matching purposes, not a postal-reference lookup.
- **No machine learning.** Scoring is rule-based and transparent; weights are tuneable but the algorithm is fixed.
- **No persistence layer.** The crate scores pairs in memory; storage and indexing belong upstream.
- **No locale-aware street-type vocabulary.** Only the English `St` / `Rd` / `Ave` / `N` / … abbreviations are expanded. Locale-aware vocabularies are an Open Question.

## License

MIT OR Apache-2.0 OR GPL-2.0 OR GPL-3.0 OR BSD-3-Clause — see [`LICENSE.md`](./LICENSE.md).

## Contributing

Contributions welcome. Before opening a PR:

- `cargo fmt`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`

See [`AGENTS.md`](./AGENTS.md) for the working guide and [`spec.md`](./spec.md) for the authoritative behaviour spec.

## Contact

Joel Parker Henderson — <joel@joelparkerhenderson.com>.
