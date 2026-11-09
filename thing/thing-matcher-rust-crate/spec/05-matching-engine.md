## 5. Matching engine

### 5.1 `MatchingEngine::deterministic_match(&self, &Thing, &Thing) -> bool`

Returns `true` iff any of the following hold:

1. The two things share any `(property_id, value)` pair in their `identifiers` lists. Property IDs are compared as case-sensitive opaque strings; values are compared after trimming (done at construction by `Identifier::new`).
2. The two things share any `sameAs` URL after `normalize_url`.
3. Both things have a `url` and the two URLs are equal after `normalize_url`.

Otherwise returns `false`.

Properties:

- **Reflexive.** `deterministic_match(t, t)` is `true` whenever `t` has at least one identifier, sameAs URL, or canonical url.
- **Symmetric.** `deterministic_match(a, b) == deterministic_match(b, a)`.
- **Cheap.** O(n·m) over the smaller cross-product of identifier and URL lists; no string-similarity work; no allocation beyond the per-URL normalisation buffer.

### 5.2 `MatchingEngine::match_things(&self, &Thing, &Thing) -> MatchResult`

The probabilistic path. Produces a `MatchResult` (§3.6) carrying a renormalised score (§5.10), the `is_match` boolean, the `Confidence` band, and the per-field `MatchBreakdown`.

Under `MatchConfig::strict_mode = true`, `is_match` requires `score >= threshold` **AND** `deterministic_match(a, b)`. Probabilistic `score` and `confidence` are unchanged under strict mode — only `is_match` becomes more conservative.

### 5.3 Batch entry points

```rust
fn match_one_to_many(&self, query: &Thing, candidates: &[Thing]) -> Vec<MatchResult>;
fn rank_one_to_many(&self, query: &Thing, candidates: &[Thing]) -> Vec<(usize, MatchResult)>;
```

- `match_one_to_many` returns one `MatchResult` per candidate, in input order.
- `rank_one_to_many` returns `(original_index, MatchResult)` tuples sorted by descending score; ties break by ascending original index so the result is fully deterministic.

The engine is immutable after construction and `Send + Sync`, so consumers MAY wrap either call in `rayon::par_iter`, `tokio::spawn_blocking`, or similar without changes to this crate.

### 5.4 Constructors

```rust
MatchingEngine::new(MatchConfig)        // explicit
MatchingEngine::default_config()        // = new(MatchConfig::default())
```

The engine owns only a `MatchConfig`, so cloning is cheap. A consumer MAY hold one engine per (strict / default / lenient) preset across the lifetime of its process.

### 5.5 Determinism, safety, IO

- **Deterministic.** Same inputs produce the same outputs, in the same byte order. No clocks, no RNGs, no environment variables.
- **No `unsafe`.** The crate top-level declares `#![forbid(unsafe_code)]`.
- **No IO.** The library never logs, reads files, opens sockets, or queries DNS.
- **Panic-free.** Every fallible input returns either `None` from a scorer or a `MatchingError`. The library MUST NOT panic on any in-range `f64` or any valid UTF-8 string.
- **`Send + Sync`.** Every public type implements both. The engine is safe to share across threads by reference.

### 5.6 String similarity primitives

`Scorer` (in `crate::scorer`) exposes:

- `Scorer::jaro_winkler_similarity(&a, &b) -> f64`
- `Scorer::levenshtein_similarity(&a, &b)  -> f64`
- `Scorer::exact_match(&a, &b)             -> f64`  // `1.0` or `0.0`
- `Scorer::combined_similarity(&a, &b)     -> f64`  // `0.7·JW + 0.3·Lev`
- `Scorer::jaccard_set_similarity(&[T], &[T]) -> f64`  // for `same_as`, `additional_types`
- `Scorer::optional_field_score(&Option<String>, &Option<String>, SimilarityAlgorithm) -> f64`  // both-`None` ⇒ `1.0`, one-`None` ⇒ `0.0`, both-`Some` ⇒ the chosen algorithm. Published for downstream reuse; `MatchingEngine::match_things` does NOT call it — the engine skips a field when either side is absent (§5.10) rather than scoring it `0.0`.

All similarity values are in `[0.0, 1.0]`. Empty-string handling: `jaro_winkler` / `levenshtein` / `combined` return `1.0` when both inputs are empty and `0.0` when exactly one is empty; `exact_match` returns `1.0` only when both inputs are equal (including both empty).

### 5.7 Phonetic matching

When `MatchConfig::use_phonetic_matching = true`, the engine computes Soundex codes (`Normalizer::phonetic_code`) for every name on each side (primary + alternate names) and sets `name_phonetic_score = 1.0` iff any cross-pair shares a non-empty Soundex code; `0.0` otherwise. The result is added as a **bonus** in §5.10 — it never lowers the score.

When `use_phonetic_matching = false`, `name_phonetic_score` is `None` and the bonus is omitted entirely.

### 5.8 Identifier scoring (`identifiers_score`)

| Condition | `identifiers_score` |
|---|---|
| Either side's `identifiers` list is empty | `None` |
| Both sides non-empty AND any `(property_id, value)` pair is shared | `Some(1.0)` |
| Both sides non-empty AND no pair is shared | `Some(0.0)` |

Note the **asymmetry vs. deterministic match**: an empty `identifiers` list on either side suppresses the probabilistic contribution entirely (so the empty side does not unfairly tank the score), but `deterministic_match` would still consider the URL signals. This is intentional.

### 5.9 URL scoring

| Field | Behaviour |
|---|---|
| `url_score` | `None` if either side absent; `1.0` if both `normalize_url`-equal; `0.0` otherwise. |
| `image_score` | Same shape as `url_score`. |
| `main_entity_of_page_score` | Same shape as `url_score`. |
| `same_as_score` | Jaccard over the union of normalised `same_as` URLs. `None` only if BOTH sides are empty; otherwise `Some(intersection / union)`. |
| `additional_types_score` | Same shape as `same_as_score`. |

### 5.9.1 Relationships scoring

Landed in v0.7.0 (see §3.3.1, CHANGELOG.md).

`relationships_score` is typed-set **Jaccard** over the `(relation, thing_id)` pairs: `score = |A ∩ B| / |A ∪ B|`, where each side's set is `{ (r.relation, r.thing_id) for r in relationships }`. The relation kind is part of the key — a `Contains` reference only agrees with a `Contains` reference to the **same** thing id; `ContainedIn` / `SuperPart` / `SubPart` are compared as opaque, distinct kinds (no inversion, no transitive closure). `None` (does not participate) when **either** side has no relationships; otherwise a value in `[0.0, 1.0]`. A supporting signal weighted `relationships_weight` (§3.4, default `0.05`); shared references never single-handedly establish a match.

### 5.9.2 Tags scoring

Landed in v0.7.0 (see §3.1, CHANGELOG.md).

`tags_score` is plain set **Jaccard** over the tag sets: `score = |A ∩ B| / |A ∪ B|`, where each side's set is the operator's `tags` after case-insensitive normalisation (trim + lowercase, de-duplicated). `None` (does not participate) when **either** side has an empty tag set; otherwise a value in `[0.0, 1.0]`. A supporting signal weighted `tags_weight` (§3.4, default `0.05`); shared tags never single-handedly establish a match.

### 5.10 Renormalised weighted sum

`relationships` and `tags` landed in v0.7.0 (§5.9.1, §5.9.2) — `calculate_weighted_score` (`src/matcher.rs`) sums all eleven fields below.

```
weighted_sum  = 0.0
total_weight  = 0.0

for each scored field (name, description, disambig, identifiers,
                       url, same_as, image, main_entity_of_page,
                       additional_types, relationships, tags):
    if breakdown.field_score is Some(s):
        weighted_sum += s * config.field_weight
        total_weight += config.field_weight

if phonetic_enabled AND breakdown.name_phonetic_score == Some(s) AND s > 0.9:
    weighted_sum += s * 0.05
    total_weight += 0.05

score = if total_weight > 0.0 { weighted_sum / total_weight } else { 0.0 }
```

Properties:

- **Missing fields do not penalise.** A `None` field is skipped entirely — neither numerator nor denominator changes.
- **Phonetic is a bonus.** The Soundex contribution adds to both `weighted_sum` and `total_weight` only when its score is above `0.9`, which means it can only push the renormalised score upward (since it adds a value above `0.9` weighted by `0.05`).
- **Empty input.** If every field is `None`, the score is `0.0` and `is_match` is `false`.
- **Bounded.** The score is mathematically guaranteed to lie in `[0.0, 1.0]`.

### 5.11 Score → `is_match` decision

```
above_threshold = score >= config.match_threshold

is_match = if config.strict_mode {
    above_threshold && deterministic_match(a, b)
} else {
    above_threshold
}
```

`confidence = Confidence::from_score(score)` independently of `match_threshold` (see §3.8).

### 5.12 Match evidence (`MatchResult::evidence`)

Every `MatchResult` also carries an `Evidence` value: how much the compared data **proves**, beside how alike it is. The model is fixed family-wide by [`agents/share/match-evidence.md`](../../../agents/share/match-evidence.md) (Fellegi–Sunter log-likelihood weights); this section records only what is specific to this crate. `score`, `is_match` and `confidence` are unchanged by it.

- **Weight.** `evidence.weight_bits` is the sum of `component_bits(s, m, u)` over every component both records hold, with `AGREE_AT = 0.95`, `DISAGREE_AT = 0.60` and linear interpolation between. A component either record lacks adds `0` and is not counted in `fields_compared`. Unlike the similarity breakdown, an empty-vs-non-empty `sameAs` or `additionalType` set, and a blank or fragment-only `image` / `mainEntityOfPage` / `url`, count as **absent**, not as `0.0`.
- **Identity group.** The three signals `deterministic_match` (§5.1) treats as identity — a shared `(property_id, value)` identifier, any shared `sameAs` URL, an equal non-empty canonical `url` — add `IDENTIFIER_BITS = log2(0.99 / 1e-12) ≈ 39.85` **once** per pair however many of them agree, and count as one field. A `url` or `sameAs` set present on both sides that does not agree adds its own disagreement weight. The plain `url` counts as identity here because §5.1 already treats it as identity.
- **Contradictions.** Every `property_id` (trimmed) is a scheme, exactly as §5.1 trusts every scheme (see OQ-C / OQ-H). For each scheme under which both records hold at least one non-blank value and the two value sets share none, `evidence.contradictions` gets `{ field: "identifier:<scheme lowercase>" }` and the weight gets `log2(0.01 / (1 − 1e-12)) ≈ −6.64` bits. A record holding several values under one scheme is not contradicted by a partner holding any one of them. Different schemes neither agree nor contradict.
- **Strength.** `Contradicted` if any contradiction; else `Proven` if `weight_bits >= MatchConfig::proof_bits` (default `30`); else `Consistent` if `is_match`; else `NonMatch`.

Per-component `m` / `u` (informed estimates, `src/evidence.rs`):

| Component | `m` | `u` | Agree bits | Disagree bits |
|---|---|---|---|---|
| `name_score` | 0.95 | 1e-3 | +9.89 | −4.32 |
| `name_phonetic_score` | 0.90 | 0.30 | not summed (derived from the name, §6 of the contract) | — |
| `description_score` | 0.70 | 1e-3 | +9.45 | −1.74 |
| `disambiguating_description_score` | 0.80 | 5e-3 | +7.32 | −2.31 |
| `identifiers_score` | 0.99 | 1e-12 | identity group (+39.85 once) | per contradicted scheme (−6.64) |
| `url_score` | 0.70 | 1e-6 | identity group | −1.74 |
| `same_as_score` | 0.50 | 1e-6 | identity group | −1.00 |
| `image_score` | 0.60 | 1e-3 | +9.23 | −1.32 |
| `main_entity_of_page_score` | 0.70 | 1e-4 | +12.77 | −1.74 |
| `additional_types_score` | 0.90 | 0.10 | +3.17 | −3.17 |
| `relationships_score` | 0.60 | 0.05 | +3.58 | −1.25 |
| `tags_score` | 0.60 | 0.10 | +2.58 | −1.17 |

#### 5.12.1 Correlated groups (public contract)

`evidence::CORRELATED_GROUPS` (contract §6). Within a group the present member with the largest absolute bits (ties to the larger value) counts in full, every other present member at `CORRELATED_DISCOUNT = 0.5`. Agreements and disagreements alike are discounted; the result does not depend on component order. A component in no group stands alone.

| Group | Why the members move together |
|---|---|
| `image`, `main_entity_of_page` | Both are scraped from the same web page. Two different products harvested from one catalogue page share its hero image and its page URL for one reason — the page — not two. |
| `description`, `disambiguating_description` | Both are free text written by one source, the second often abridged from the first. A copied blurb agrees on both for one reason. |

`name_phonetic_score` is in no group because it is not summed at all: a Soundex code is derived from the name (contract §6).

#### 5.12.2 Value frequencies (public contract)

`MatchingEngine::match_things_with_frequencies(a, b, &dyn ValueFrequencies)` adjusts the agreement weight of the components in `evidence::FREQUENCY_COMPONENTS` (contract §7); `match_things` calls it with `NoFrequencies`, so a plain call uses the table above unchanged. `score`, `is_match`, `confidence` and `breakdown` are identical in both.

| Component | Values counted | Key (`frequency_key(component, raw)`) |
|---|---|---|
| `name` | the primary `name` and every `alternateName` | `Normalizer::normalize_name(raw)`; `None` if it normalises to empty |

- Adjusted only on an **exact** agreement: the two records share at least one normalised name. Then the name term is `frequency_bits(0.95, share)` = `log2(0.95 / clamp(share, MIN_SHARE = 1e-9, 0.95))` for the shared key — `0` bits at or above `m`, never negative. With several shared names the strongest term counts; a shared name with no known share uses the table's agreement weight.
- Partial agreement (similar, not equal) and disagreement are never adjusted.
- `FrequencyTable` holds shares in memory: `FrequencyTable::from_counts(rows, total)` from `(component, key, count)` rows (`share = count / total`; `total = 0` gives an empty table) and `insert(component, key, share)`.

#### 5.12.3 Per-term listing

`evidence.components: Vec<ComponentBits>` (`#[serde(default)]`) lists every term — `name`, `description`, `disambiguating_description`, `image`, `main_entity_of_page`, `additional_types`, `relationships`, `tags`, `url`, `same_as` (disagreements only), `identifier` (the single identity term) and one `contradiction:identifier:<scheme>` per contradiction — each with its bits after discount/adjustment and `correlated` / `frequency_adjusted` flags. `weight_bits` is exactly their sum, in list order.

#### 5.12.4 Worked figures

- Two identical records holding only a name: score `1.0`, `+9.89` bits — `Consistent`, not `Proven`.
- An identical record with every non-identity field populated (name, both descriptions, image, `mainEntityOfPage`, a type, a relationship, a tag) and no identifier, `url` or `sameAs`: `+49.73` bits, `Proven` (it was `+58.0` before the groups; the image/page group now gives `17.39` instead of `22.0`, the description group `13.11` instead of `16.77`).
- Same source: two products from one catalogue page sharing image, `mainEntityOfPage` and a type, with different names: `+16.24` bits, `NonMatch` (`+20.86` without the image/page group).
- Generic product: two different products both named "USB Cable" with the same description, type and tag: `+25.10` bits from the table, `+21.78` with a name share of `0.01` — `Consistent`, never `Proven`.

---

