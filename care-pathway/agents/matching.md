# Matching Algorithm Reference — Care Pathway Entity

The matching system compares two `CarePathway` records and produces a
`MatchResult`: score in `[0.0, 1.0]`, `Confidence`, `is_match`, and a
per-component `MatchBreakdown`. The algorithm lives entirely in the
matcher crate; the service embeds it unchanged. Canonical detail:
matcher [spec §5–§18](../care-pathway-matcher-rust-crate/spec/index.md)
and
[agents/matching-algorithm.md](../care-pathway-matcher-rust-crate/agents/matching-algorithm.md).

## Deterministic short-circuits (→ 1.0)

| Rule | Condition |
|---|---|
| R-0 | Any shared value on a deterministic identifier scheme: `Doi`, `Wikidata`, `GuidelineId`, `Uri`, `Uuid` (empty values ignored) |
| R-1 | Same non-empty `provider_id` AND equal normalised `pathway_code` |
| R-2 | Any case-folded `same_as` URL overlap |

`PathwayCode` / `LocalId` / `Custom` schemes are **excluded** from
R-0 — provider-scoped codes are not globally unique.

## Probabilistic components

Renormalised weighted average over the components both records carry
(divisor = sum of contributing weights):

| Component | Weight | Algorithm | Skipped when |
|---|---:|---|---|
| Name | 0.30 | Best Jaro-Winkler over `name` + `alternate_names` (folded); Soundex +0.05 bonus on primary names, capped at 0.95 | never (name required) |
| Condition codes | 0.25 | Jaccard over lower-cased `"system:code"` tokens | both sides empty |
| Pathway code | 0.15 | Same provider: 1.0 if normalised codes equal, else 0.0 | differing / missing provider |
| Care setting | 0.10 | Exact enum: 1.0 / 0.0 | either side unset |
| Interventions | 0.10 | Jaccard over `fold_set` | both sides empty (0.0 if exactly one populated) |
| Keywords | 0.10 | Jaccard over `fold_set` | both sides empty (0.0 if exactly one populated) |

## Normalisation

| Function | Rule |
|---|---|
| `fold` | trim + NFKC + lowercase; **diacritics preserved** |
| `pathway_code` | alphanumeric-only, uppercased — `"STROKE-01"` ≡ `"stroke 01"` |
| `fold_set` | fold + sort + dedupe |

## Classification

| Output | Rule |
|---|---|
| `Confidence::High` | score ≥ 0.95 |
| `Confidence::Medium` | score ≥ 0.70 |
| `Confidence::Low` | otherwise |
| `is_match` | score ≥ `MatchConfig::threshold` — default 0.85; presets `strict()` 0.95, `lenient()` 0.70 |

## Entity-level tuning notes

- Condition codes are the defining attribute of a pathway — hence
  the second-highest weight. Many pathways share a condition, so a
  shared code alone is corroboration, not a pin (matcher spec §16
  open question).
- Do not change default weights / threshold without updating the
  matcher spec §7, this file, the entity spec §6.2, and CHANGELOGs.

## Where matching runs in the service

**File:** [`src/controllers/care_pathways.rs`](../care-pathway-service-with-loco/src/controllers/care_pathways.rs)
— both endpoints construct `MatchingEngine::new(MatchConfig::default())`:

- `POST /api/care-pathways/match` → `engine.rank(query, candidates)`
  over the request payload (no DB).
- `POST /api/care-pathways/check-duplicates` → loads up to 1 000
  active rows, `match_care_pathways` per candidate, returns
  `is_match` hits sorted by score descending.

## Source files (matcher crate)

- `src/matcher.rs` — `MatchingEngine`, per-component fns,
  deterministic rules, `rank`, `find_matches`
- `src/scoring.rs` — `MatchResult`, `MatchBreakdown`, `Confidence`,
  weighted average
- `src/config.rs` — `MatchConfig` (weights + threshold + presets)
- `src/normalize.rs` — `fold`, `pathway_code`, `fold_set`
- `src/phonetic.rs` — Soundex
