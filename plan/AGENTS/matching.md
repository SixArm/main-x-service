# Matching Algorithm Reference — Plan Entity

The matching system compares two `Plan` records and produces a
`MatchResult`: score in `[0.0, 1.0]`, `Confidence`, `is_match`, and a
per-component `MatchBreakdown`. The algorithm lives entirely in the
matcher crate; the service embeds it unchanged. Canonical detail:
matcher [spec §5–§18](../plan-matcher-rust-crate/spec/index.md) and
[AGENTS/matching-algorithm.md](../plan-matcher-rust-crate/AGENTS/matching-algorithm.md).

Matching is over the **plan identity** — the project / product /
programme / initiative / portfolio / epic header. The operational
sub-resources (tasks, issues, posts, comments, members) are **not**
matched; only goal titles feed a probabilistic component (see below).
Portfolio-level deduplication is the primary use case.

## Deterministic short-circuits (→ 1.0)

| Rule | Condition |
|---|---|
| R-0 | Any shared value on a deterministic identifier scheme: `Uri`, `Uuid`, `JiraProjectKey`, `AsanaGid`, `TrelloBoardId`, `MsProjectId`, `GitHubProjectId`, `LinearId` (empty values ignored) |
| R-1 | Same non-empty `owner_org_id` AND equal normalised `plan_code` |
| R-2 | Any case-folded `same_as` URL overlap |

`PlanCode` / `LocalId` / `Custom` schemes are **excluded** from
R-0 — owner-scoped codes are not globally unique.

## Probabilistic components

Renormalised weighted average over the components both records carry
(divisor = sum of contributing weights):

| Component | Weight | Algorithm | Skipped when |
|---|---:|---|---|
| Name | 0.30 | Best Jaro-Winkler over `name` + `alternate_names` (folded); Soundex +0.05 bonus on primary names, capped at 0.95 | never (name required) |
| Goals | 0.15 | Jaccard over `fold_set` of goal titles | both sides empty |
| PlanCode | 0.15 | Same owner: 1.0 if normalised codes equal, else 0.0 | differing / missing owner |
| OwnerOrg | 0.10 | Exact `owner_org_id` match: 1.0 / 0.0 | either side unset |
| PlanType | 0.08 | Exact enum: 1.0 / 0.0 | either side unset |
| Timeframe | 0.07 | Date proximity over `[start_date, end_date]` (overlap / closeness decay) | either side lacks dates |
| Keywords | 0.05 | Jaccard over `fold_set` | both sides empty (0.0 if exactly one populated) |
| Relationships | 0.05 | Jaccard over typed `"kind:target"` token set | both sides empty |
| Tags | 0.05 | Jaccard over `fold_set` | both sides empty (0.0 if exactly one populated) |

Weights sum to 1.00.

## Normalisation

| Function | Rule |
|---|---|
| `fold` | trim + NFKC + lowercase; **diacritics preserved** |
| `plan_code` | alphanumeric-only, uppercased — `"PROJ-01"` ≡ `"proj 01"` |
| `fold_set` | fold + sort + dedupe |

## Classification

| Output | Rule |
|---|---|
| `Confidence::High` | score ≥ 0.95 |
| `Confidence::Medium` | score ≥ 0.70 |
| `Confidence::Low` | otherwise |
| `is_match` | score ≥ `MatchConfig::threshold` — default 0.85; presets `strict()` 0.95, `lenient()` 0.70 |

## Entity-level tuning notes

- Name is the dominant signal for portfolio dedup — hence the
  highest weight. Goals and an owner-scoped plan code are the two
  strongest corroborators.
- Goals are matched only by **title** (Jaccard over goal titles), not
  by goal body or status — titles are the stable, comparable surface.
- A shared `owner_org_id` alone is weak corroboration (`OwnerOrg`
  0.10); it only pins a match when combined with an equal plan code
  (R-1).
- Do not change default weights / threshold without updating the
  matcher spec §7, this file, the entity spec §6.2, and CHANGELOGs.

## Where matching runs in the service

**File:** [`src/controllers/plans.rs`](../plan-service-with-loco/src/controllers/plans.rs)
— both endpoints construct `MatchingEngine::new(MatchConfig::default())`:

- `POST /api/v1/plans/match` → `engine.rank(query, candidates)`
  over the request payload (no DB).
- `POST /api/v1/plans/check-duplicates` → loads up to 1 000
  active rows, `match_plans` per candidate, returns
  `is_match` hits sorted by score descending.

## Source files (matcher crate)

- `src/matcher.rs` — `MatchingEngine`, per-component fns,
  deterministic rules, `rank`, `find_matches`
- `src/scoring.rs` — `MatchResult`, `MatchBreakdown`, `Confidence`,
  weighted average
- `src/config.rs` — `MatchConfig` (weights + threshold + presets)
- `src/normalize.rs` — `fold`, `plan_code`, `fold_set`
- `src/phonetic.rs` — Soundex
