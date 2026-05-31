# Matching Algorithm Reference

The matching system compares two events and produces a confidence
score in `[0.0, 1.0]` plus a per-component breakdown.

## Strategies

- **Probabilistic** — weighted fuzzy sum of component scores.
- **Deterministic** — rule-based; counts satisfied rules as a
  fraction of available rules.

Both short-circuit to `1.0` when the two events share an exact value
on a **strong identifier type**: `BookingNumber`,
`ConfirmationCode`, `TicketNumber`, `EncounterId`, or
`TransactionId`.

## Probabilistic weights

| Component | Weight |
|---|---|
| Name (title + alternates) | 0.20 |
| Start date | 0.20 |
| End date | 0.10 |
| Location | 0.15 |
| Organizer | 0.10 |
| Performer | 0.10 |
| Attendee | 0.05 |
| Identifier | 0.10 |

Weights sum to 1.0.

## Deterministic rules

| Rule | Condition | Points |
|---|---|---|
| 0 (short-circuit) | Strong-identifier exact match | → 1.0 |
| 1 | `name_score ≥ 0.90` AND `start_score ≥ 0.95` | 1 |
| 2 | Both sides have locations AND `location_score ≥ 0.80` | 1 |
| 3 | Both sides have organizers AND `organizer_score ≥ 0.90` | 1 |

Final score = `achieved / available`. `is_match` threshold = `0.75`.

## Match quality classification

| Quality | Score range (probabilistic) |
|---|---|
| Definite | `≥ 0.95` |
| Probable | `≥ threshold` (default `0.85`) |
| Possible | `≥ 0.50` |
| Unlikely | `< 0.50` |

## Component algorithms

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

`match_window_overlap(a_start, a_end, b_start, b_end)` — Jaccard
ratio of the two `[start, end)` intervals when both ends are known;
falls back to `match_start_dates` otherwise.

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

## Source files

- `src/matching/mod.rs` — `EventMatcher` trait, `ProbabilisticMatcher`, `DeterministicMatcher`, `MatchResult`, `MatchScoreBreakdown`
- `src/matching/algorithms.rs` — `name_matching`, `time_matching`, `location_matching`, `party_matching`, `identifier_matching`, `reference_matching`
- `src/matching/scoring.rs` — `ProbabilisticScorer`, `DeterministicScorer`, `MatchQuality`
- `src/matching/phonetic.rs` — Soundex
