## 7. Configuration

```rust
#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct MatchConfig {
    pub match_threshold: f64,
    pub name_weight: f64,
    pub start_date_weight: f64,
    pub start_date_scale_seconds: f64,
    pub end_date_weight: f64,
    pub location_weight: f64,
    pub coordinates_scale_metres: f64,
    pub category_weight: f64,
    pub country_code_weight: f64,
    pub event_ids_weight: f64,
    pub organizer_weight: f64,
    pub performers_weight: f64,
    pub url_weight: f64,
    pub use_phonetic_matching: bool,
    pub name_algorithm: SimilarityAlgorithm,
    pub strict_mode: bool,
}
```

`MatchConfig` carries `#[serde(default)]`; a partial JSON document fills missing fields from `MatchConfig::default()`.

### 7.1 Defaults — `MatchConfig::default()`

| Field | Default | Meaning |
|---|---|---|
| `match_threshold` | `0.80` | Score at or above which `is_match` is `true`. |
| `name_weight` | `0.20` | Weight of the name component (§6.1). |
| `start_date_weight` | `0.25` | Weight of the start-date component (§6.3). |
| `start_date_scale_seconds` | `3600.0` | Gaussian-decay scale `s` in seconds — one hour (§6.3). Shared by the end-date component. |
| `end_date_weight` | `0.05` | Weight of the end-date component (§6.3). |
| `location_weight` | `0.15` | Weight of the location component (§6.4). |
| `coordinates_scale_metres` | `100.0` | Gaussian-decay scale `s` in metres for the coordinates sub-score inside location (§6.4). |
| `category_weight` | `0.08` | Weight of the category component (§6.5). |
| `country_code_weight` | `0.04` | Weight of the country-code component (§6.6). |
| `event_ids_weight` | `0.15` | Weight of the event-IDs component (§6.7). |
| `organizer_weight` | `0.04` | Weight of the organizer component (§6.8). |
| `performers_weight` | `0.02` | Weight of the performers component (§6.9). |
| `url_weight` | `0.02` | Weight of the URL component (§6.10). |
| `use_phonetic_matching` | `false` | Compute Soundex-bonus (§6.2). |
| `name_algorithm` | `SimilarityAlgorithm::Combined` | Algorithm used by `name_score`. |
| `strict_mode` | `false` | Tighten `is_match` to also require `deterministic_match` (§5.2.3). |

### 7.2 Strict — `MatchConfig::strict()`

| Field | Value |
|---|---|
| `match_threshold` | `0.95` (overridden) |
| `strict_mode` | `true` (overridden) |
| every other field | inherits `default()` |

Under strict mode, `is_match` additionally requires `deterministic_match`. `score` and `confidence` are unaffected.

### 7.3 Lenient — `MatchConfig::lenient()`

| Field | Value |
|---|---|
| `match_threshold` | `0.65` (overridden) |
| `use_phonetic_matching` | `true` (overridden) |
| every other field | inherits `default()` |

### 7.4 Tuning guidance

Tuning recipes (false positives → raise `match_threshold`; recurring weekly series collapsing into one event → tighten `start_date_scale_seconds`; multi-room venue confusion → tighten `coordinates_scale_metres`; tour-stop collisions (same name + performers, different cities) → raise `location_weight`; etc.) live in [`AGENTS/matching-algorithm.md`](../AGENTS/matching-algorithm.md). The defaults table above remains normative.

---
