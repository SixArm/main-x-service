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

Produces a `MatchResult { score, is_match, confidence, breakdown, evidence }`:

- `breakdown: MatchBreakdown` — per-field `Option<f64>` contributions (§6).
- `score: f64` — weight-renormalised sum across components that scored (§5.2.2).
- `is_match: bool` — `score >= MatchConfig::match_threshold` under default mode; under strict mode (§5.2.3) the boolean additionally requires `deterministic_match` to return `true`.
- `confidence: Confidence` — fixed-band derivative of `score` (§3.9).
- `evidence: Evidence` — Fellegi–Sunter log-likelihood evidence beside the score (§5.5). It never changes `score`, `is_match`, or `confidence`.

#### 5.2.1 Per-field scoring

Each field is scored as described in §6. A `None` from any component means "did not participate"; the field is skipped by the renormaliser. The twelve weighted components implemented today are name, start date, end date, location, category, country code, event IDs, organizer, performers, URL, relationships (§6.11), and tags (§6.12).

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

### 5.5 Match evidence

`MatchResult::evidence` follows the family contract in [`agents/share/match-evidence.md`](../../../agents/share/match-evidence.md): `score` says how alike the present data is; `evidence` says how much it **proves**. Implementation: `src/evidence.rs`.

```rust
pub struct Evidence {
    pub weight_bits: f64,            // sum of log-likelihood weights, in bits
    pub fields_compared: u32,        // components both records held (identifiers count as one)
    pub contradictions: Vec<Contradiction>, // { field: "identifier:<scheme>" }
    pub strength: MatchStrength,     // Contradicted | Proven | Consistent | NonMatch
    #[serde(default)]
    pub components: Vec<ComponentBits>, // every term of the sum (contract §8)
}
```

`match_events(a, b)` is `match_events_with_frequencies(a, b, &NoFrequencies)`. Passing a `ValueFrequencies` source changes only `evidence`; `score`, `is_match`, `confidence` and `breakdown` are identical either way.

- **Components.** Each `Some` breakdown component adds `component_bits(s, m, u)`: the agreement weight `log2(m/u)` at `s >= 0.95`, the disagreement weight `log2((1−m)/(1−u))` at `s <= 0.60`, linear between. A `None` component adds `0` and is not counted. **Location (OQ-J):** the evidence weighs its own location blend, in which a nested address sharing no comparable field is left out and a location pair sharing no comparable sub-field is `None`. The neutral `0.5` of §6.4 is therefore missing data here, as in the place and worker matchers, never a disagreement; `breakdown.location_score` and `score` keep the `0.5` unchanged.
- **Per-component `m`/`u`** (informed estimates, not measurements; each constant's doc comment says where its `u` comes from):

  | Component | `m` | `u` | Agree bits | Disagree bits |
  |---|---|---|---|---|
  | `name_score` | 0.95 | 0.005 | +7.57 | −4.31 |
  | `start_date_score` | 0.90 | 0.001 | +9.81 | −3.32 |
  | `end_date_score` | 0.85 | 0.01 | +6.41 | −2.72 |
  | `location_score` | 0.85 | 0.01 | +6.41 | −2.72 |
  | `category_score` | 0.90 | 0.10 | +3.17 | −3.17 |
  | `country_code_score` | 0.98 | 0.20 | +2.29 | −5.32 |
  | `organizer_score` | 0.85 | 0.01 | +6.41 | −2.72 |
  | `performers_score` | 0.80 | 0.01 | +6.32 | −2.31 |
  | `url_score` | 0.90 | 0.0001 | +13.14 | −3.32 |
  | `relationships_score` | 0.70 | 0.05 | +3.81 | −1.66 |
  | `tags_score` | 0.60 | 0.10 | +2.58 | −1.17 |

  `name_phonetic_score` is not weighed (it is derived from the same names as `name_score`). `event_ids_score` is not weighed as a component; the identifier rule replaces it.
- **Correlated groups** (contract §6; `CORRELATED_GROUPS`, public). Within a group the present member with the largest absolute bits counts in full (ties to the larger value, so input order never matters) and every other present member at `CORRELATED_DISCOUNT = 0.5`; agreements and disagreements alike. Components in no group stand alone.

  | Group | Why the members are not independent |
  |---|---|
  | `start_date`, `end_date` | One event's two endpoints: an end is usually the start plus a standard duration, and a reschedule moves both. |
  | `location`, `organizer`, `country_code` | A series run by one organiser at one venue agrees on all three for one reason, and the venue fixes the country. |
  | `url`, `name` | Event pages are slugged from the title, and a recurring series reuses both its title and its page. |

- **Value frequencies** (contract §7; `FREQUENCY_COMPONENTS = ["name"]`, public). When both events hold the same normalised name (primary or alternate; `frequency_key("name", raw)` = `Normalizer::normalize_name(raw)`, `None` if empty) and the caller's `ValueFrequencies::share("name", key)` knows it, the name's agreement weight is `frequency_bits(m, share) = log2(m / clamp(share, MIN_SHARE = 1e-9, m))` instead of `log2(m/u)`. Several shared names: the most common is used. A share at or above `m` gives `0` bits, never negative. Partial agreement (a fuzzy name with no shared normalised value) and disagreement are never adjusted. A recurring title such as "Annual General Meeting" at a share of `0.2` gives `2.25` bits where the table gives `7.57`; `0.01` gives `6.57`; a rare `1e-6` gives `19.86`. `FrequencyTable` (`new`, `from_counts(counts, total)`, `insert`) stores keys through `frequency_key`, so raw and normalised values both work.
- **Explainability** (contract §8). `components` holds one `ComponentBits { component, bits, correlated, frequency_adjusted }` per term: the weighed components by name, `identifier` for a shared event id, and `contradiction:identifier:<scheme>` per contradiction. `weight_bits` is defined as the sum of those terms in list order, so the two are equal exactly.
- **Identifiers.** Every `EventIdScheme`, `Other(_)` included, is identity, because §5.1 rule 1 short-circuits on any shared `(scheme, value)`. A non-blank shared pair adds `IDENTIFIER_BITS = log2(0.99/1e-12) ≈ 39.85` once. Per scheme, when both records hold at least one non-blank value and the sets are disjoint, a `Contradiction { field: "identifier:<scheme>" }` is recorded (`wikidata`, `eventbrite`, …, `googlecalendar`, `icalendaruid`, `other:<name>`) and `log2(0.01/(1−1e-12)) ≈ −6.64` bits added. Different schemes neither share nor contradict. The §5.1 rule 2 (name + start instant) is **not** an identifier.
- **Strength**, checked in order: any contradiction → `Contradicted`; `weight_bits >= MatchConfig::proof_bits` (default `30`, §7) → `Proven`; `is_match` → `Consistent`; else `NonMatch`.
- **Reference points.** Identical name + start date only: `17.38` bits, `Consistent` (it is `is_match` and satisfies §5.1 rule 2, yet is not proof; the two are in different groups, so nothing is discounted). All eleven weighed components identical, no identifier: `56.58` bits after the group discount (`67.92` summed independently), `Proven`; `47.23` without the URL, still `Proven`. Two occurrences of one series a week apart (same name, organiser, venue, URL, category and country; both dates disagree): `26.17` bits, not `Proven` — summed independently the same terms would have been `32.95`.

---
