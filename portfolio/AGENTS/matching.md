# Matching Algorithm Reference — Portfolio Entity

The matching system compares two `WorkItem` records and produces a
`MatchResult`: score in `[0.0, 1.0]`, `Confidence`, `is_match`, and a
per-component `MatchBreakdown`. The algorithm lives entirely in the
matcher crate; the service embeds it unchanged. Canonical detail:
matcher [spec §5–§18](../portfolio-matcher-rust-crate/spec/index.md) and
[AGENTS/matching-algorithm.md](../portfolio-matcher-rust-crate/AGENTS/matching-algorithm.md).

Matching is over the **work-item identity** — the Portfolio / Project /
Product / Program header. The operational sub-resources (tasks, issues)
are **not** matched; only goal titles feed a probabilistic component (see
below). Within-collection deduplication (e.g. two duplicate projects, two
duplicate portfolios) is the primary use case.

## The kind gate (→ 0.0)

| Rule | Condition |
|---|---|
| R-GATE | `A.kind != B.kind` → **0.0** (no match) — different kinds are distinct record types in distinct collections |

R-GATE runs **first**, before any short-circuit or component. It is the
headline rule that distinguishes the portfolio matcher from a
single-entity matcher: a project and a product are never the same record,
so the matcher refuses to score them. `kind` is therefore a **gate, not a
weighted component** — there is no `kind`/`type` score.

## Deterministic short-circuits (→ 1.0)

Evaluated only after R-GATE passes (same `kind`):

| Rule | Condition |
|---|---|
| R-0 | Any shared value on a deterministic identifier scheme: `Uri`, `Uuid`, `JiraProjectKey`, `AsanaGid`, `TrelloBoardId`, `MsProjectId`, `GitHubProjectId`, `LinearId` (empty values ignored) |
| R-1 | Same non-empty `owner_org_id` AND equal normalised `code` |
| R-2 | Any case-folded `same_as` URL overlap |

`Code` / `LocalId` / `Custom` schemes are **excluded** from R-0 —
owner-scoped codes are not globally unique.

## Probabilistic components

Renormalised weighted average over the components both records carry
(divisor = sum of contributing weights):

| Component | Weight | Algorithm | Skipped when |
|---|---:|---|---|
| Name | 0.30 | Best Jaro-Winkler over `name` + `alternate_names` (folded); Soundex +0.05 bonus on primary names, capped at 0.95 | never (name required) |
| Goals | 0.15 | Jaccard over `fold_set` of goal titles | both sides empty |
| Code | 0.15 | Same owner: 1.0 if normalised codes equal, else 0.0 | differing / missing owner |
| OwnerOrg | 0.10 | Exact `owner_org_id` match: 1.0 / 0.0 | either side unset |
| Portfolio | 0.08 | Child kinds only: same parent `portfolio_ref` exact 1.0 / 0.0 | either side unset (always for Portfolio kind) |
| Timeframe | 0.07 | Date proximity over `[start_date, target_date]` (Gaussian decay on day gap, σ default 90) | either side lacks dates |
| Keywords | 0.05 | Jaccard over `fold_set` | both sides empty (0.0 if exactly one populated) |
| Relationships | 0.05 | Jaccard over typed `"relation:work_item_id"` token set | **either** side empty (strict — supporting signal) |
| Tags | 0.05 | Jaccard over `fold_set` | **either** side empty (strict — supporting signal) |

Weights sum to 1.00. `Portfolio` (0.08) replaces plan's `PlanType`
component — kind is now a gate (R-GATE), and the parent-portfolio link is
the new weighted corroborator for child kinds.

## Normalisation

| Function | Rule |
|---|---|
| `fold` | trim + NFKC + lowercase; **diacritics preserved** |
| `code` | alphanumeric-only, uppercased — `"PROJ-01"` ≡ `"proj 01"` |
| `fold_set` | fold + sort + dedupe |

## Classification

| Output | Rule |
|---|---|
| `Confidence::High` | score ≥ 0.95 |
| `Confidence::Medium` | score ≥ 0.70 |
| `Confidence::Low` | otherwise |
| `is_match` | score ≥ `MatchConfig::threshold` — default 0.85; presets `strict()` 0.95, `lenient()` 0.70 |

## Entity-level tuning notes

- R-GATE is non-negotiable: matching is partitioned by `kind`, mirroring
  the four distinct collections. Never relax it into a weighted
  component.
- Name is the dominant signal for within-collection dedup — hence the
  highest weight. Goals and an owner-scoped `code` are the two strongest
  corroborators.
- Goals are matched only by **title** (Jaccard over goal titles), not by
  goal body or status — titles are the stable, comparable surface.
- A shared `owner_org_id` alone is weak corroboration (`OwnerOrg` 0.10);
  it only pins a match when combined with an equal `code` (R-1).
- `Portfolio` (0.08) corroborates two children sharing the same parent;
  it contributes nothing for Portfolio-kind records (no `portfolio_ref`).
- Do not change default weights / threshold without updating the matcher
  spec §7, this file, the entity spec §6.2, and CHANGELOGs.

## Where matching runs in the service

**File:** [`src/controllers/`](../portfolio-service-with-loco/src/controllers/)
— each collection's two matching endpoints construct
`MatchingEngine::new(MatchConfig::default())`:

- `POST /api/{collection}/match` → `engine.rank(query, candidates)`
  over the request payload (no DB).
- `POST /api/{collection}/check-duplicates` → loads up to 1 000 active
  rows **from that collection only**, `match_work_items` per candidate,
  returns `is_match` hits sorted by score descending. (R-GATE makes
  cross-collection matching impossible even if rows leaked in.)

## Source files (matcher crate)

- `src/matcher.rs` — `MatchingEngine`, the R-GATE check, per-component
  fns, deterministic rules, `rank`, `find_matches`
- `src/scoring.rs` — `MatchResult`, `MatchBreakdown`, `Confidence`,
  weighted average
- `src/config.rs` — `MatchConfig` (weights + threshold + presets)
- `src/normalize.rs` — `fold`, `code`, `fold_set`
- `src/phonetic.rs` — Soundex
