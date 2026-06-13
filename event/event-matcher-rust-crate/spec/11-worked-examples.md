## 11. Worked examples

The examples below trace the spec through realistic event data. They are illustrative; the authoritative pinned behaviour lives in `tests/integration_tests.rs`, `tests/property_tests.rs`, and the doctests under `src/*.rs`.

### 11.1 Glastonbury Festival / Glasto — probabilistic match

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
// r.is_match == true; r.confidence == High
```

Sketch of the per-field scores under default weights:

| Component | Value | Weight |
|---|---|---|
| `name_score` | best of `{Glastonbury Festival 2024, Glasto 2024} × {Glasto 2024}` = `1.00` (alternate matches exactly) | `0.20` |
| `start_date_score` | `d = 900 s`, `s = 3600 s`, `exp(-(900/3600)^2) ≈ 0.94` | `0.25` |
| `end_date_score` | identical instants → `1.0` | `0.05` |
| `category_score` | both `Festival` → `1.0` | `0.08` |
| `country_code_score` | both `"GB"` → `1.0` | `0.04` |
| every other component | `None` (no data) | — |

Renormalised: `weighted_sum / total_weight ≈ (0.20 + 0.235 + 0.05 + 0.08 + 0.04) / 0.62 ≈ 0.976`. Default threshold `0.80` → `is_match = true`; `Confidence::High`.

### 11.2 Other illustrative scenarios

The following pinned scenarios live as runnable tests in `tests/integration_tests.rs` and as worked examples in [`AGENTS/matching-algorithm.md`](../AGENTS/matching-algorithm.md):

- **RustConf 2024 / RustConf '24** — cartesian-product name scoring across `name` × `alternate_names` picks the best pair.
- **Shared Eventbrite ID** — `deterministic_match` short-circuits to `true` even with wholly different names (`test_event_id_shared_scores_one_and_is_deterministic_match`); distinct schemes carrying the same value never match (`test_event_id_distinct_schemes_do_not_match`).
- **Same name + same start instant across offsets** — `"2024-09-10T09:00:00Z"` and `"2024-09-10T11:00:00+02:00"` parse to the same Unix instant, so deterministic rule 2 fires (`test_start_date_score_accepts_equivalent_offsets`, `test_deterministic_via_name_and_start_date`).
- **Same tour, different stops** — two listings sharing name and performers but a month apart: `start_date_score` decays to ~0 and drags the renormalised score below `0.80` despite `name_score = 1.0` (`test_start_date_score_far_apart_decays_to_zero`).
- **Postcode-dominated venue blend** — inside `location_score`, a matching postcode dominates the address sub-blend (`test_location_postcode_match_dominates_address_subscore`).
- **Capacities are data-only** — `maximum_attendee_capacity` and friends round-trip through serde but are not scored.
- **Batch screening (`rank_one_to_many`)** — deterministic descending-score ordering with ascending-index tiebreak (§5.3) (`test_rank_one_to_many_sorts_by_score_descending`).

### 11.3 Strict mode — fuzzy match without deterministic agreement

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

let default = MatchingEngine::default_config();
let strict  = MatchingEngine::new(MatchConfig::strict());

let r_default = default.match_events(&a, &b);
let r_strict  = strict.match_events(&a, &b);

assert!(r_default.is_match);    // probabilistic threshold cleared
assert!(!r_strict.is_match);    // strict additionally requires deterministic_match;
                                // no shared EventId, and the names differ
                                // under normalisation despite identical start instants
assert!((r_default.score - r_strict.score).abs() < 1e-12);  // score unaffected
```

Strict mode (§5.2.3) tightens only the `is_match` boolean. The renormalised `score`, the `confidence` band, and every field in `MatchBreakdown` are identical between the two configurations.

---
