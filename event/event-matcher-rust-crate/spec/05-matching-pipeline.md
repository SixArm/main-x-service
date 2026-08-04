## 5. Matching pipeline

The crate exposes two strategies under one engine.

### 5.1 Deterministic match

```rust
fn deterministic_match(&self, e1: &Event, e2: &Event) -> bool;
```

Returns `true` iff **any one** of the following holds:

1. The two `event_ids` lists share at least one element under `EventId` equality (i.e. an identical `(scheme, value)` pair).
2. Both events carry a `name` that normalises (via `Normalizer::normalize_name`) to the same string **AND** both carry a `start_date` that parses (via `Normalizer::parse_iso8601_unix_seconds`, §4.7) to the same Unix instant. Textually different layouts of the same instant agree: `"2024-09-10T09:00:00Z"` matches `"2024-09-10T11:00:00+02:00"`.

Otherwise returns `false`. Boolean result; no score. A missing or unparseable `start_date` on either side fails rule 2 (it never falls back to textual comparison).

### 5.2 Probabilistic match

```rust
fn match_events(&self, e1: &Event, e2: &Event) -> MatchResult;
```

Produces a `MatchResult { score, is_match, confidence, breakdown }`:

- `breakdown: MatchBreakdown` — per-field `Option<f64>` contributions (§6).
- `score: f64` — weight-renormalised sum across components that scored (§5.2.2).
- `is_match: bool` — `score >= MatchConfig::match_threshold` under default mode; under strict mode (§5.2.3) the boolean additionally requires `deterministic_match` to return `true`.
- `confidence: Confidence` — fixed-band derivative of `score` (§3.9).

#### 5.2.1 Per-field scoring

Each field is scored as described in §6. A `None` from any component means "did not participate"; the field is skipped by the renormaliser. The ten weighted components implemented today are name, start date, end date, location, category, country code, event IDs, organizer, performers, and URL. Two further supporting components, relationships (§6.11) and tags (§6.12), are specified but not yet implemented (§3.1).

#### 5.2.2 Weighted-sum reducer

```text
weighted_sum  = sum of (score_i * weight_i)  over components that scored
total_weight  = sum of (weight_i)            over components that scored
if name_phonetic_score is Some and > 0.9:
    weighted_sum += name_phonetic_score * 0.05
    total_weight += 0.05
score = if total_weight > 0 then weighted_sum / total_weight else 0.0
```

The renormalisation rule MUST hold: missing fields neither contribute to nor penalise the overall score.

Worked numeric example. Suppose `name_score = Some(0.95)` (weight `0.20`), `start_date_score = Some(0.90)` (weight `0.25`), `category_score = Some(1.0)` (weight `0.08`), and every other component is `None`. Then `weighted_sum = 0.95 * 0.20 + 0.90 * 0.25 + 1.0 * 0.08 = 0.19 + 0.225 + 0.08 = 0.495`. `total_weight = 0.20 + 0.25 + 0.08 = 0.53`. `score = 0.495 / 0.53 ≈ 0.9340`. With default threshold `0.80`, `is_match = true`; `Confidence::from_score(0.9340) = High`.

#### 5.2.3 Strict mode

When `MatchConfig::strict_mode = true`, the engine computes the same `score` and `confidence`, but tightens `is_match` to `score >= match_threshold && deterministic_match`. A fuzzy near-match that clears the threshold but has no shared event ID and no normalised-name + same-start-instant agreement is rejected as `is_match = false`.

### 5.3 Batch entry points

```rust
fn match_one_to_many(&self, query: &Event, candidates: &[Event]) -> Vec<MatchResult>;
fn rank_one_to_many(&self, query: &Event, candidates: &[Event]) -> Vec<(usize, MatchResult)>;
```

- `match_one_to_many` returns results parallel to the input slice.
- `rank_one_to_many` returns the same results sorted by descending `score`, with ties broken by ascending original index. Ordering MUST be deterministic across calls with the same inputs.

### 5.4 Behaviour when a field is missing

If a field is `None` on either side — or, for `start_date` / `end_date`, fails to parse as ISO 8601 on either side — the corresponding `MatchBreakdown` entry is `None` and the field is skipped by the weighted-sum reducer. This is the renormalisation rule: missing data is treated as "no information", not as "disagreement".

The single exception is `name_phonetic_score`: when `MatchConfig::use_phonetic_matching = false`, it is always `None` regardless of input data.

---
