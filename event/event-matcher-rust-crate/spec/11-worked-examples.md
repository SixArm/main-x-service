## 11. Worked examples

The examples below trace the spec through realistic place data. They are illustrative; the authoritative pinned behaviour lives in `tests/integration_tests.rs`, `tests/property_tests.rs`, and the doctests under `src/*.rs`.

### 11.1 Eiffel Tower / La Tour Eiffel — probabilistic match

```rust
let a = Place::builder()
    .name("Eiffel Tower")
    .add_alternate_name("La Tour Eiffel")
    .latitude(48.858_222)
    .longitude(2.294_500)
    .category(PlaceCategory::Monument)
    .country_code_as_iso_3166_1_alpha_2("FR")
    .build();

let b = Place::builder()
    .name("Tour Eiffel")
    .latitude(48.858_3)
    .longitude(2.294_5)
    .category(PlaceCategory::Monument)
    .country_code_as_iso_3166_1_alpha_2("FR")
    .build();

let r = MatchingEngine::default_config().match_places(&a, &b);
// r.is_match == true; r.confidence == High
```

Sketch of the per-field scores under default weights:

| Component | Value | Weight |
|---|---|---|
| `name_score` | best of `{Eiffel Tower, La Tour Eiffel} × {Tour Eiffel}` ≈ `1.00` (alternate matches exactly) | `0.20` |
| `coordinates_score` | `d ≈ 9 m`, `s = 50 m`, `exp(-(9/50)^2) ≈ 0.97` | `0.30` |
| `category_score` | both `Monument` → `1.0` | `0.10` |
| `country_code_score` | both `"FR"` → `1.0` | `0.05` |
| every other component | `None` (no data) | — |

Renormalised: `weighted_sum / total_weight ≈ (0.20 + 0.291 + 0.10 + 0.05) / 0.65 ≈ 0.984`. Default threshold `0.80` → `is_match = true`; `Confidence::High`.

### 11.2 Other illustrative scenarios

The following pinned scenarios live as runnable tests in `tests/integration_tests.rs` and as worked examples in [`AGENTS/matching-algorithm.md`](../AGENTS/matching-algorithm.md):

- **Big Ben / Elizabeth Tower** — cartesian-product name scoring across `name` × `alternate_names` picks the best pair (Big Ben ≈ Big Ben → ~1.0).
- **Two Starbucks branches — same name, different cities** — coordinates dominate and drag the renormalised score well below `0.80` despite `name_score = 1.0`, `category_score = 1.0`, `country_code_score = 1.0`.
- **Shared Google Place ID** — `deterministic_match` short-circuits to `true` even with wholly different names.
- **Snowdon / Yr Wyddfa** — bilingual names match via the alternate-name cartesian product; `elevation_as_metre` is data-only (§3.1.1).
- **Wembley stadium with capacity / footprint** — `area_as_metre_2`, `maximum_capacity_count` round-trip through serde but are not scored.
- **Batch screening (`rank_one_to_many`)** — deterministic descending-score ordering with ascending-index tiebreak (§5.3).

### 11.3 Strict mode — fuzzy match without deterministic agreement

```rust
let a = Place::builder()
    .name("The British Museum")
    .latitude(51.519_5)
    .longitude(-0.126_9)
    .category(PlaceCategory::Museum)
    .build();

let b = Place::builder()
    .name("British Museum")
    .latitude(51.519_5)
    .longitude(-0.126_9)
    .category(PlaceCategory::Museum)
    .build();

let default = MatchingEngine::default_config();
let strict  = MatchingEngine::new(MatchConfig::strict());

let r_default = default.match_places(&a, &b);
let r_strict  = strict.match_places(&a, &b);

assert!(r_default.is_match);    // probabilistic threshold cleared
assert!(!r_strict.is_match);    // strict additionally requires deterministic_match;
                                // no shared PlaceId, no shared postcode here
assert!((r_default.score - r_strict.score).abs() < 1e-12);  // score unaffected
```

Strict mode (§5.2.3) tightens only the `is_match` boolean. The renormalised `score`, the `confidence` band, and every field in `MatchBreakdown` are identical between the two configurations.

---

