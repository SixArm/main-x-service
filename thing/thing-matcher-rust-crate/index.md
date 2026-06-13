# Thing matcher Rust Crate

A Rust library for deciding whether two records describe the same
[`schema.org/Thing`](https://schema.org/Thing).

## What it does

`thing-matcher` compares pairs of [`Thing`] records — books, articles,
landmarks, products, organisations, people, events, or any other entity
that can be described with the
[`schema.org/Thing`](https://schema.org/Thing) vocabulary — and tells you
whether they refer to the same item. It is built for de-duplication and
record linkage across data sources that disagree on names, encodings, and
identifier schemes.

The crate provides two strategies behind one engine:

- **Deterministic** — a hard `bool` when both records share a
  scheme-scoped `identifier` (Wikidata QID, ISBN, DOI, GTIN, …), or a
  `sameAs` reference URL, or a canonical `url`.
- **Probabilistic** — a weight-renormalised score in `[0.0, 1.0]` over
  `name`, `description`, `disambiguatingDescription`, `identifier`,
  `url`, `sameAs`, `image`, `mainEntityOfPage`, and `additionalType`,
  with a per-field [`MatchBreakdown`] so every decision is auditable.

The library is pure: no IO, no clocks, no RNGs, `#![forbid(unsafe_code)]`,
`Send + Sync`. It is suitable as a leaf dependency under web servers,
batch jobs, or notebooks.

## Installation

```toml
[dependencies]
thing-matcher = "0.6.1"
```

## Quick start — probabilistic match

```rust
use thing_matcher::{MatchingEngine, Thing};

let a = Thing::builder()
    .name("Eiffel Tower")
    .url("https://www.toureiffel.paris/")
    .build();

let b = Thing::builder()
    .name("La Tour Eiffel")
    .add_alternate_name("Eiffel Tower")
    .url("https://www.toureiffel.paris/")
    .build();

let engine = MatchingEngine::default_config();
let result = engine.match_things(&a, &b);

assert!(result.is_match);
println!("score = {:.2}", result.score);
println!("name  = {:?}", result.breakdown.name_score);
println!("url   = {:?}", result.breakdown.url_score);
```

The `MatchBreakdown` carries one `Option<f64>` per scored field; a `None`
means the field was absent on at least one side and so did not
contribute. Missing fields neither raise nor lower the score.

## Quick start — deterministic match

A shared external identifier — any `(property_id, value)` pair across
the two records — is enough on its own:

```rust
use thing_matcher::{Identifier, MatchingEngine, Thing};

let id = Identifier::new("wikidata", "Q243").unwrap();

let a = Thing::builder().name("Eiffel Tower").add_identifier(id.clone()).build();
let b = Thing::builder().name("Tour Eiffel").add_identifier(id).build();

let engine = MatchingEngine::default_config();
assert!(engine.deterministic_match(&a, &b));
```

A shared `sameAs` URL or a shared canonical `url` is also accepted as a
deterministic match — `sameAs` exists in `schema.org` precisely to point
at "a reference Web page that unambiguously indicates the item's
identity".

## The `Thing` model

The `Thing` data model mirrors the
[`schema.org/Thing`](https://schema.org/Thing) vocabulary:

| Rust field | schema.org property | Purpose |
|---|---|---|
| `name` | `name` | Primary canonical name. |
| `alternate_names` | `alternateName` | Aliases, endonyms, translations. The matcher takes the best score across the cartesian product. |
| `description` | `description` | Free-form description. Compared as text. |
| `disambiguating_description` | `disambiguatingDescription` | Short disambiguating description. Compared as text. |
| `identifiers` | `identifier` (as `PropertyValue`) | Scheme-scoped external identifiers. Sharing one is a deterministic match. |
| `url` | `url` | Canonical URL. Compared after URL normalisation. |
| `image` | `image` | URL of a representative image. |
| `same_as` | `sameAs` | Reference URLs that unambiguously identify the item. |
| `main_entity_of_page` | `mainEntityOfPage` | Page (URL) for which this thing is the main entity. |
| `additional_types` | `additionalType` | Additional types from external vocabularies (e.g. `https://schema.org/Landmark`). |
| `subject_of` | `subjectOf` | Works or events about this thing (URLs). |
| `owner` | `owner` | Person or organisation that owns this thing. |
| `local_id` | — | Originating-system identifier. Not scored. |

Build records via the fluent [`Thing::builder`]; all setters accept
`impl Into<String>`.

## The match pipeline

1. Each scoring component yields `Some(score)` in `[0.0, 1.0]` or `None`
   (missing on at least one side).
2. The weighted sum runs over components that scored.
3. The sum of participating weights divides through — **renormalisation**
   ensures missing fields do not penalise.
4. A phonetic-name match (Soundex on normalised names) adds a
   `0.05`-weighted bonus when the gating phonetic score exceeds `0.9`,
   only when `use_phonetic_matching` is on. The bonus never lowers a
   score.
5. `is_match = score >= match_threshold` (strict mode additionally
   requires `deterministic_match`).
6. `confidence = Confidence::from_score(score)` — bands are fixed (`>=
   0.90` High, `>= 0.75` Medium, else Low) and **independent** of
   `match_threshold`.

### Default weights

| Component | Weight | Notes |
|---|---|---|
| Name | `0.30` | Best of cartesian product across primary + alternates, via the configured `SimilarityAlgorithm` (default `Combined` = 0.7 × Jaro-Winkler + 0.3 × Levenshtein). |
| Description | `0.10` | `Combined` similarity over the normalised text. |
| Disambiguating description | `0.05` | `Combined` similarity over the normalised text. |
| Identifiers | `0.25` | `1.0` if any `(property_id, value)` pair is shared, `0.0` if both non-empty but no overlap, `None` if either empty. |
| URL | `0.05` | Exact equality after URL normalisation. |
| sameAs | `0.15` | Jaccard set similarity over normalised URLs. |
| Image | `0.03` | Exact equality after URL normalisation. |
| mainEntityOfPage | `0.02` | Exact equality after URL normalisation. |
| additionalType | `0.05` | Jaccard set similarity over normalised URIs. |
| Phonetic bonus | `+0.05` when gated | Bonus only — never lowers a score. |

## Configuration presets

```rust
use thing_matcher::{MatchConfig, MatchingEngine};

let strict  = MatchingEngine::new(MatchConfig::strict());  // threshold 0.95, requires deterministic
let default = MatchingEngine::default_config();            // threshold 0.80
let lenient = MatchingEngine::new(MatchConfig::lenient()); // threshold 0.65, phonetic on
```

- **Default (0.80)** — balanced for everyday de-duplication.
- **Strict (0.95)** — for downstream systems that must rely on the
  answer; `is_match` additionally requires `deterministic_match`. `score`
  and `confidence` are unaffected.
- **Lenient (0.65)** — for triaging large candidate sets where false
  negatives are costlier than false positives.

Every field of `MatchConfig` is overridable; the config is `Serialize +
Deserialize` (with `#[serde(default)]`) so tunings can live in a file:

```rust
use thing_matcher::MatchConfig;

let cfg: MatchConfig = serde_json::from_str(r#"{
    "match_threshold": 0.85,
    "name_weight": 0.50
}"#).unwrap();
```

## Batch scoring

```rust
use thing_matcher::{MatchingEngine, Thing};

let engine = MatchingEngine::default_config();
let query = Thing::builder().name("Eiffel Tower").build();
let candidates = vec![
    Thing::builder().name("Big Ben").build(),
    Thing::builder().name("Eiffel Tower").build(),
    Thing::builder().name("Statue of Liberty").build(),
];

// Parallel to the input slice:
let results = engine.match_one_to_many(&query, &candidates);

// Sorted by descending score, deterministic tiebreak on original index:
let ranked = engine.rank_one_to_many(&query, &candidates);
let (best_idx, best) = &ranked[0];
println!("best is candidate[{best_idx}] with score {:.2}", best.score);
```

The engine is `Send + Sync`. Wrap calls in `rayon::par_iter` (or any
parallelism primitive) without changes to this crate. Candidate
pre-filtering — Soundex prefix blocking, identifier-scheme blocking,
sameAs-prefix blocking — is intentionally a consumer concern.

## Determinism and safety

- **`#![forbid(unsafe_code)]`** at the crate root.
- **No IO.** The library does not read files, open sockets, or log.
- **No clocks, no RNGs, no environment variables.** Same inputs always
  produce the same outputs.
- **No panics** in library code; every fallible parser returns `None`
  and every fallible operation returns `Result`.
- **`Send + Sync`.** Engines are immutable after construction and cheap
  to clone.
- **Serde-clean.** Every public data type round-trips through
  `serde_json` (and any other `serde` format).

## Limitations / out of scope

- **Not a triple store.** This crate does not store or query
  schema.org graphs; it only compares pairs of in-memory `Thing`
  records.
- **No JSON-LD parser.** Inputs are built via the [`Thing::builder`] —
  callers are responsible for translating `schema.org` JSON-LD to the
  `Thing` shape if they need to ingest it.
- **No full URL canonicalisation.** [`Normalizer::normalize_url`]
  lowercases the scheme and host and trims a root trailing slash, but
  does not perform percent-encoding canonicalisation or punycode
  decoding.
- **No machine learning.** Scoring is rule-based and transparent;
  weights are tuneable but the algorithm is fixed.
- **No persistence layer.** The crate scores pairs in memory; storage
  and indexing belong upstream.

## License

MIT OR Apache-2.0 OR GPL-2.0 OR GPL-3.0 OR BSD-3-Clause — see
[`LICENSE.md`](./LICENSE.md).

## Contributing

Contributions welcome. Before opening a PR:

- `cargo fmt`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`

See [`AGENTS.md`](./AGENTS.md) for the working guide.

## Contact

Joel Parker Henderson — <joel@joelparkerhenderson.com>.
