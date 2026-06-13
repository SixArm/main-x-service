## 11. Worked examples

The examples below trace the spec through realistic place data. They are illustrative; the authoritative pinned behaviour lives in `tests/integration_tests.rs`, `tests/property_tests.rs`, and the doctests under `src/*.rs`. Further worked examples (Big Ben / Elizabeth Tower, Mt Snowdon / Yr Wyddfa, Wembley Stadium, batch ranking, strict mode) live in [`AGENTS/matching-algorithm.md`](../AGENTS/matching-algorithm.md).

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

Per-field sketch under default weights: `name_score ≈ 1.00` (the alternate matches exactly; weight `0.20`); `coordinates_score = exp(-(9/50)^2) ≈ 0.97` (≈ 9 m apart; weight `0.30`); `category_score = 1.0` (both `Monument`; weight `0.10`); `country_code_score = 1.0` (both `"FR"`; weight `0.05`); every other component is `None`. Renormalised: `(0.20 + 0.291 + 0.10 + 0.05) / 0.65 ≈ 0.984` ≥ `0.80` → `is_match = true`; `Confidence::High`.

### 11.2 Two Starbucks branches — same name, different cities

```rust
let manhattan = Place::builder()
    .name("Starbucks")
    .latitude(40.7589)
    .longitude(-73.9851)
    .category(PlaceCategory::Cafe)
    .country_code_as_iso_3166_1_alpha_2("US")
    .build();

let los_angeles = Place::builder()
    .name("Starbucks")
    .latitude(34.0522)
    .longitude(-118.2437)
    .category(PlaceCategory::Cafe)
    .country_code_as_iso_3166_1_alpha_2("US")
    .build();

let r = MatchingEngine::default_config().match_places(&manhattan, &los_angeles);
// r.is_match == false; coordinates_score decays essentially to 0
```

`name_score = 1.0`, `category_score = 1.0`, `country_code_score = 1.0` — yet `coordinates_score ≈ 0.0` (separation ~3940 km, far beyond `3 × scale`). With `coordinates_weight = 0.30` dominating, the renormalised total drops well below `0.80`.

### 11.3 Shared Google Place ID — deterministic short-circuit

```rust
let id = PlaceId::new(PlaceIdScheme::Google, "ChIJLU7jZClu5kcR4PcOOO6p3I0").unwrap();
let a = Place::builder().name("Eiffel Tower").add_place_id(id.clone()).build();
let b = Place::builder().name("Wholly Different Name").add_place_id(id).build();

assert!(MatchingEngine::default_config().deterministic_match(&a, &b));
```

`deterministic_match` short-circuits on the shared `(Google, "ChIJ…")` pair. Names and coordinates are irrelevant.

---

