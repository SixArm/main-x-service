# Matching Algorithm Reference — Portfolio Entity

The matching system compares two `Plan` records and produces a
`MatchResult`: score in `[0.0, 1.0]`, `Confidence`, `is_match`, and a
per-component `MatchBreakdown`. The algorithm lives entirely in the
matcher crate; the service embeds it unchanged. Canonical detail:
matcher [spec §5–§18](../project-portfolio-management-matcher-rust-crate/spec/index.md) and
[AGENTS/matching-algorithm.md](../project-portfolio-management-matcher-rust-crate/AGENTS/matching-algorithm.md).

Matching is over the **plan identity** — the plan header. The optional
`kind` label (Portfolio / Project / Product / Program / Practice /
Process / Purpose / Pathway / Proposal) is descriptive only and does
**not** affect matching. The operational sub-resources (tasks, issues)
are **not** matched; only goal titles feed a probabilistic component
(see below). Deduplication (e.g. two duplicate plans) is the primary use
case.

## Kind is not a gate

Matching is **kind-agnostic**: any two plans may match regardless of
their `kind` labels. There is no kind gate — a plan labelled `Project`
can match one labelled `Product` if the other signals agree. `kind` is a
purely descriptive/display/grouping label; it is neither a gate nor a
weighted component, so there is no `kind`/`type` score.
(`MatchBreakdown.kind_gate_blocked` remains only as a vestigial,
always-`false` field for wire compatibility.)

## Deterministic short-circuits (→ 1.0)

Evaluated for any pair (kind is never consulted):

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
| Parent | 0.08 | Same parent `parent_ref` exact 1.0 / 0.0 | either side unset |
| Timeframe | 0.07 | Date proximity over `[start_date, target_date]` (Gaussian decay on day gap, σ default 90) | either side lacks dates |
| Keywords | 0.05 | Jaccard over `fold_set` | both sides empty (0.0 if exactly one populated) |
| Relationships | 0.05 | Jaccard over typed `"relation:plan_id"` token set | **either** side empty (strict — supporting signal) |
| Tags | 0.05 | Jaccard over `fold_set` | **either** side empty (strict — supporting signal) |

Weights sum to 1.00. The `Parent` (0.08) component corroborates two
plans that share the same parent plan; it contributes nothing when either
side has no `parent_ref`.

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

- Matching is kind-agnostic: `kind` is a descriptive label, never a gate
  or a weighted component. Any two plans may match. Never re-introduce a
  kind gate.
- Name is the dominant signal for dedup — hence the highest weight. Goals
  and an owner-scoped `code` are the two strongest corroborators.
- Goals are matched only by **title** (Jaccard over goal titles), not by
  goal body or status — titles are the stable, comparable surface.
- A shared `owner_org_id` alone is weak corroboration (`OwnerOrg` 0.10);
  it only pins a match when combined with an equal `code` (R-1).
- `Parent` (0.08) corroborates two plans sharing the same parent plan; it
  contributes nothing when either side has no `parent_ref`.
- Do not change default weights / threshold without updating the matcher
  spec §7, this file, the entity spec §6.2, and CHANGELOGs.

## Where matching runs in the service

**File:** [`src/controllers/`](../project-portfolio-management-service-with-loco/src/controllers/)
— the plan matching endpoints construct
`MatchingEngine::new(MatchConfig::default())`:

- `POST /api/plans/match` → `engine.rank(query, candidates)`
  over the request payload (no DB).
- `POST /api/plans/check-duplicates` → loads up to 1 000 active plan
  rows, `match_plans` per candidate, returns `is_match` hits sorted by
  score descending.

## Source files (matcher crate)

- `src/matcher.rs` — `MatchingEngine`, per-component fns, deterministic
  rules, `rank`, `find_matches`
- `src/scoring.rs` — `MatchResult`, `MatchBreakdown`, `Confidence`,
  weighted average
- `src/config.rs` — `MatchConfig` (weights + threshold + presets)
- `src/normalize.rs` — `fold`, `code`, `fold_set`
- `src/phonetic.rs` — Soundex
