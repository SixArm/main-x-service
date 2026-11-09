# Matching Algorithm Reference

The service scores every pair with the canonical `event-matcher` crate
(spec §6.2, §13 T-16): `ProbabilisticMatcher` projects both events
through `src/matching/adapter.rs` and calls
`MatchingEngine::match_events_with_frequencies`. The **score**
(`[0.0, 1.0]`, renormalised over the components both events hold),
**`is_match`** (score ≥ `threshold_score`, default `0.85`), **quality**
and **breakdown** are the matcher's; the weights and component
algorithms are in the matcher spec §5–§7.

Quality labels, from the matcher's confidence band: `certain` (≥ 0.90),
`probable` (≥ 0.75), `possible` (below).

## Evidence and auto-merge

Every result carries the matcher's evidence
([match-evidence.md](../../../agents/share/match-evidence.md)): bits,
fields compared, contradictions, per-term components and a strength.
A batch item is `AutoMerged` only when score ≥ `auto_merge_threshold`
**and** strength is `proven` (`matching::permits_auto_merge`).
`consistent` (a match on score, short of proof) and `contradicted`
(two values under one identifier scheme) stay `Pending`.

The adapter scopes the service's own identifier types by system
(`Other("<Type>@<system>")`), so two systems' references are separate
schemes rather than a contradiction.

The name's agreement weight is frequency-adjusted from counted shares
(`src/matching/frequencies.rs`; none below 1 000 records).

## Standalone component algorithms

`src/matching/algorithms.rs` and `phonetic.rs` hold the service's older
component similarity functions. They were combined by the service's own
scorer until T-16 (which counted absent components as `0`); no request
path calls them now, and they are kept for their benchmarks.


### Name (`name_matching`)

`match_titles(a, b)` — case-insensitive Jaro-Winkler + normalized
Levenshtein; takes the max. Adds a Soundex floor of `0.85` when the
two titles share a Soundex code.

`match_name_with_alternates(primary_a, alt_a, primary_b, alt_b)` —
best pairwise score across both lists.

### Time (`time_matching`)

`match_start_dates(a, b)` — exponential decay with half-life of
1 hour. Same-second match = `1.0`; 1 h apart ≈ `0.5`; 1 day apart
≈ `0.03`.

`match_end_dates(a, b)` — like `match_start_dates`; both `None` → `0.5`.

> **Removed (T-13, 2026-09-06): `match_window_overlap`.** A
> `match_window_overlap(a_start, a_end, b_start, b_end)` helper (Jaccard
> ratio of the two `[start, end]` intervals) was implemented and unit
> tested here but never called from `scoring::ProbabilisticScorer` (since removed, T-16),
> which scores `start_date`/`end_date` independently instead. Wiring it
> in would mean unilaterally resolving `event-matcher`'s own still-open
> **OQ-C** (`event-matcher-rust-crate/spec/10-open-questions.md`:
> whether the matcher should score window overlap instead of, or in
> addition to, independent endpoint proximity, and how a
> `start_date`-only record should participate) — a canonical-crate
> design question this service crate defers to rather than answering
> unilaterally with a half-built, uncalled function of its own. Removed
> rather than left dead; revisit once OQ-C resolves upstream.

### Location (`location_matching`)

`match_location` dispatches by `Location` variant:

- `Place ↔ Place`: short-circuit to `1.0` when both have the same
  external `id`; otherwise 0.4 × name + 0.4 × address + 0.2 × geo.
- `PostalAddress ↔ PostalAddress`: postal_code (0.30) + city (0.20)
  + state (0.20) + line1 (0.30).
- `Place ↔ PostalAddress`: compares the place's address to the
  postal address.
- `Virtual ↔ Virtual`: case-insensitive URL equality.
- `Text ↔ Text`: title similarity.
- Other cross-variant pairings: `0.0`.

`match_addresses` / `match_locations` take the best pair.

Geo proximity uses Haversine distance with sigmoid decay.

### Party (`party_matching`)

`match_party`:

- Different `kind` (Person vs Organization) → `0.0`.
- Both have the same external `id` → `1.0`.
- Otherwise `max(name_similarity, email_exact_match)`.

`match_parties` returns the best pair.

### Identifier (`identifier_matching`)

- Different `identifier_type` or `system` → `0.0`.
- Identical normalized value → `1.0`.
- Identical with formatting (dashes/spaces) stripped → `0.98`.
- Otherwise → `0.0`.

### Reference (`reference_matching`)

External `id` match → `1.0`; otherwise name similarity.

### Phonetic (`phonetic.rs`)

Soundex (4-character code: first letter + three digits). Used by
`name_matching` as a similarity floor.

| Letters | Code |
|---|---|
| B, F, P, V | 1 |
| C, G, J, K, Q, S, X, Z | 2 |
| D, T | 3 |
| L | 4 |
| M, N | 5 |
| R | 6 |
| A, E, I, O, U, H, W, Y | ignored |

## Language tags (`in_language`) — known divergence

The service validates `in_language` entries as 2-letter ISO 639-1
codes; the embedded `event-matcher` crate documents its single
`in_language` field as a full IETF BCP 47 tag. The bridge
(`src/matching/adapter.rs`) projects only the first non-empty entry,
and the matcher treats the field as data-only (never scored), so the
divergence is currently inert. Documented in service spec §6.2;
tracked as entity task ET-8.

## Source files

- `src/matching/mod.rs` — `EventMatcher` trait, `ProbabilisticMatcher`, `MatchResult`, `MatchScoreBreakdown`
- `src/matching/adapter.rs` — service `Event` → `event_matcher::Event`
- `src/matching/evidence.rs` — `MatchEvidence` (schema'd mirror of the matcher's evidence), `permits_auto_merge`
- `src/matching/frequencies.rs` — `FrequencySource`, `FrequencyCache`, `MIN_FREQUENCY_POPULATION`
- `src/matching/algorithms.rs` — standalone `name_matching`, `time_matching`, `location_matching`, `party_matching`, `identifier_matching`, `reference_matching`
- `src/matching/phonetic.rs` — Soundex
