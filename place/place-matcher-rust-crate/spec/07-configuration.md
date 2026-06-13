## 7. Configuration

```rust
#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct MatchConfig {
    pub match_threshold: f64,
    pub name_weight: f64,
    pub coordinates_weight: f64,
    pub coordinates_scale_metres: f64,
    pub address_weight: f64,
    pub category_weight: f64,
    pub country_code_weight: f64,
    pub place_ids_weight: f64,
    pub phone_weight: f64,
    pub email_weight: f64,
    pub use_phonetic_matching: bool,
    pub name_algorithm: SimilarityAlgorithm,
    pub strict_mode: bool,
    pub gmail_dot_folding: bool,
    pub phone_default_country: Option<String>,
}
```

`MatchConfig` carries `#[serde(default)]`; a partial JSON document fills missing fields from `MatchConfig::default()`.

### 7.1 Defaults — `MatchConfig::default()`

| Field | Default | Meaning |
|---|---|---|
| `match_threshold` | `0.80` | Score at or above which `is_match` is `true`. |
| `name_weight` | `0.20` | Weight of the name component (§6.1). |
| `coordinates_weight` | `0.30` | Weight of the coordinates component (§6.3). |
| `coordinates_scale_metres` | `50.0` | Gaussian-decay scale `s` in metres (§6.3). |
| `address_weight` | `0.10` | Weight of the address component (§6.4). |
| `category_weight` | `0.10` | Weight of the category component (§6.5). |
| `country_code_weight` | `0.05` | Weight of the country-code component (§6.6). |
| `place_ids_weight` | `0.15` | Weight of the place-IDs component (§6.7). |
| `phone_weight` | `0.03` | Weight of the phone component (§6.8). |
| `email_weight` | `0.02` | Weight of the email component (§6.9). |
| `use_phonetic_matching` | `false` | Compute Soundex-bonus (§6.2). |
| `name_algorithm` | `SimilarityAlgorithm::Combined` | Algorithm used by `name_score`. |
| `strict_mode` | `false` | Tighten `is_match` to also require `deterministic_match` (§5.2.3). |
| `gmail_dot_folding` | `false` | Apply Gmail-specific localpart canonicalisation (§4.4). |
| `phone_default_country` | `Some("GB")` | Fallback ISO 3166-1 alpha-2 country for E.164 parsing (§4.3.2). |

### 7.2 Strict — `MatchConfig::strict()`

Overrides `match_threshold = 0.95` and `strict_mode = true`; every other field inherits `default()`. Under strict mode, `is_match` additionally requires `deterministic_match`; `score` and `confidence` are unaffected.

### 7.3 Lenient — `MatchConfig::lenient()`

Overrides `match_threshold = 0.65` and `use_phonetic_matching = true`; every other field inherits `default()`.

### 7.4 Tuning guidance

The "symptom → knob" tuning table is maintained in [`AGENTS/matching-algorithm.md`](../AGENTS/matching-algorithm.md) under "Tuning guidance".

---

