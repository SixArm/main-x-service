# Matching Algorithm Reference — Case Entity

The matching system compares two `Case` records and produces a
`MatchResult`: score in `[0.0, 1.0]`, `Confidence`, `is_match`, and a
per-component `MatchBreakdown`. The algorithm lives entirely in the
matcher crate; the service embeds it unchanged. Canonical detail:
matcher [spec §5–§18](../case-matcher-rust-crate/spec/index.md) and
[agents/matching-algorithm.md](../case-matcher-rust-crate/agents/matching-algorithm.md).

## Deterministic short-circuits (→ 1.0)

| Rule | Condition |
|---|---|
| R-0 | Any shared value on a deterministic identifier scheme: `Docket`, `ExternalCaseId`, `Uri`, `Uuid` (empty values ignored) |
| R-1 | Same non-empty `agency_id` AND equal normalised `case_number` |
| R-2 | Any case-folded `same_as` URL overlap |

`AgencyCaseNumber` / `LocalId` / `Custom` schemes are **excluded** from
R-0 — agency-scoped codes are not globally unique, and `case_number`
never short-circuits across agencies.

## Probabilistic components

Renormalised weighted average over the components both records carry
(divisor = sum of contributing weights):

| Component | Weight | Algorithm | Skipped when |
|---|---:|---|---|
| Title | 0.30 | Best Jaro-Winkler over `title` + `alternate_titles` (folded); Soundex +0.05 bonus on primary titles, capped at 0.95 | never (title required) |
| Subjects | 0.25 | Jaccard over the folded subject-id set | both sides empty |
| Case number | 0.15 | Same agency: 1.0 if normalised codes equal, else 0.0 | differing / missing agency |
| Case type | 0.10 | Exact enum: 1.0 / 0.0 | either side unset |
| Status | 0.05 | Exact enum: 1.0 / 0.0 | either side unset |
| Keywords | 0.15 | Jaccard over `fold_set` | both sides empty |

`priority`, `opened_date`, `agency_name`, and `in_language` do **not**
contribute to the score.

## Normalisation

| Function | Rule |
|---|---|
| `fold` | trim + NFKC + lowercase; **diacritics preserved** |
| `case_number` | alphanumeric-only, uppercased — `"BEN-2026-00417"` ≡ `"ben 2026 00417"` |
| `fold_set` | fold + sort + dedupe |

## Classification

| Output | Rule |
|---|---|
| `Confidence::High` | score ≥ 0.95 |
| `Confidence::Medium` | score ≥ 0.70 |
| `Confidence::Low` | otherwise |
| `is_match` | score ≥ `MatchConfig::threshold` — default 0.85; presets `strict()` 0.95, `lenient()` 0.70 |

## Entity-level tuning notes

- Subjects are the strongest discriminator after the title — two cases
  about the same people for the same agency are almost certainly the
  same matter — hence the second-highest weight. A shared subject set
  alone is corroboration, not a pin (matcher spec §16 open question).
- Status carries a deliberately low weight (0.05): the same case moves
  through statuses over time, so a status mismatch should barely
  penalise an otherwise-strong match.
- Do not change default weights / threshold without updating the matcher
  spec §7, this file, the entity spec §6.2, and CHANGELOGs.

## Where matching runs in the service

**File:** [`src/controllers/cases.rs`](../case-service-with-loco/src/controllers/cases.rs)
— both endpoints construct `MatchingEngine::new(MatchConfig::default())`:

- `POST /api/cases/match` → `engine.rank(query, candidates)` over the
  request payload (no DB).
- `POST /api/cases/check-duplicates` → loads up to
  `CHECK_DUPLICATES_SCAN_CAP` (= 1 000) active rows, `match_cases` per
  candidate, returns `is_match` hits sorted by score descending (a
  `tracing::warn!` fires at the cap).

## Source files (matcher crate)

- `src/matcher.rs` — `MatchingEngine`, per-component fns, deterministic
  rules, `rank`, `find_matches`
- `src/scoring.rs` — `MatchResult`, `MatchBreakdown`, `Confidence`,
  weighted average
- `src/config.rs` — `MatchConfig` (weights + threshold + presets)
- `src/normalize.rs` — `fold`, `case_number`, `fold_set`
- `src/phonetic.rs` — Soundex
