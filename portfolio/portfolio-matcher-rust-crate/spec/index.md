# portfolio-matcher — Specification

> **Single source of truth.** Code conforms to this spec. A behavioural
> change is a three-part PR: spec edit + code edit + test edit. Live
> work queue is §23; open questions are §16.

## 1. Purpose

`portfolio-matcher` is a reusable, dependency-light Rust library for
**pairwise work-item record matching**. A *work item* is a named unit of
intended work — a **Portfolio** (the umbrella container), or a
**Project**, **Product**, or **Program** that sits under a portfolio —
each with goals and a timeframe, tracked in a portfolio /
project-management registry. The canonical matchable type is
`WorkItem`, carrying a required discriminator `kind: WorkItemKind`
(`Portfolio` | `Project` | `Product` | `Program`); a Portfolio is the
umbrella kind of work item. Given two `WorkItem` records the matcher
returns a `MatchResult`: score in `[0.0, 1.0]`, `Confidence`,
`is_match`, and a per-component `MatchBreakdown`. It is the canonical
algorithm embedded in `portfolio-service`'s matching layer for
deduplication **within each work-item collection**.

The four kinds map to four distinct service collections / tables
(`portfolios`, `projects`, `products`, `programs`); they are **not**
types of one collapsed entity. The matcher reuses **one** comparison
core but **gates on kind** (§5, R-GATE): two work items of different
kind never match. Matching is always *within-kind*.

## 2. Scope

In scope: the attributes that distinguish one work item from another —
name, goals, owner-scoped code, owning organisation, parent portfolio,
timeframe, keywords, tags, relationships, and tool/registry
identifiers. Out of scope: the full work-item content (task breakdown,
resourcing, Gantt scheduling, status history, issues), person-level
assignment data, and anything requiring IO, a runtime, or network
access. The operational sub-resources (tasks / issues, and goals beyond
their titles) belong to the service, not the matcher.

## 3. Glossary

- **Work item** — a named unit of intended work, of `kind` Portfolio /
  Project / Product / Program, with goals and a timeframe.
- **Kind** — the required `WorkItemKind` discriminator; a hard match
  gate (§5, R-GATE) and the collection/table the record lives in.
- **Portfolio** — the umbrella `kind` of work item; Project / Product /
  Program records carry a `portfolio_ref` to their parent portfolio.
- **Deterministic identifier** — globally unique (URI, UUID, Jira
  project key, Asana GID, Trello board id, MS Project id, GitHub
  project id, Linear id). A match pins the score to `1.0`.
- **Owner-scoped code** — `code`/`Code`/`LocalId`; only unique within
  the issuing organisation.
- **Goal** — a discrete intended outcome of the work item; its
  **title** is the matchable surface.

## 4. Research basis

Work items are largely identified by the tool that tracks them (Jira,
Asana, Trello, MS Project, GitHub Projects, Linear) and by their owning
organisation, name, and goals. The same initiative is frequently
re-entered across tools or teams, so matching combines deterministic
linkage on the tool/registry identifiers with fuzzy comparison of the
name and overlap of the goal titles, owner, parent portfolio, timeframe,
keywords, tags, and relationships. Because the registry partitions work
items into distinct collections by `kind`, the matcher first refuses any
cross-kind comparison (R-GATE): a project and a product are different
record types and are never the same record.

## 5. Algorithm overview

```
Input: WorkItem A, WorkItem B, MatchConfig
  ├─ R-GATE A.kind != B.kind?                   ─yes─> 0.0 (no match)
  ├─ R-0 deterministic identifier match?        ─yes─> 1.0
  ├─ R-1 same owner + code?                     ─yes─> 1.0
  ├─ R-2 same_as URL overlap?                   ─yes─> 1.0
  │
  ├─ name_score          (always)   Jaro-Winkler + Soundex bonus
  ├─ goals_score         (≥1 set)   Jaccard over folded goal titles
  ├─ code_score          (same owner)  1.0/0.0
  ├─ owner_org_score     (both set) case-folded exact (1.0/0.0)
  ├─ portfolio_score     (both set, child kinds) same parent portfolio_ref exact (1.0/0.0)
  ├─ timeframe_score     (dates set) date proximity (Gaussian decay)
  ├─ keywords_score      (≥1 set)   Jaccard
  ├─ relationships_score (≥1 set)   typed-set Jaccard over (relation, work_item_id)
  ├─ tags_score          (both set) set Jaccard over normalised tags
  └─ renormalised weighted average over present components
```

**R-GATE is the headline rule.** It runs *before* every other rule,
deterministic or probabilistic: if `A.kind != B.kind`, the result is an
immediate `0.0` no-match (different collections — a project is never a
product). It is not a weighted component; it is a gate. All subsequent
rules and components assume `A.kind == B.kind`. See §12 for the gate's
formal statement.

## 6. Domain model

The canonical domain model lives in the **entity-level spec**
([`../../spec/index.md`](../../spec/index.md) §5); this section restates
only the matcher-relevant surface. The crate's `WorkItem` type **is**
that model (§20) — the matcher type, the API DTO, and the persisted
JSONB payload are one shape, no adapter.

`WorkItem`: `kind` (`WorkItemKind`, **required**), `name` (required),
`alternate_names`, `code` (`Option<String>`, owner-scoped),
`owner_org_id` (`Option<String>`, EntityRef organization),
`owner_org_name` (`Option<String>`), `lead_ref` (`Option<String>`,
EntityRef person/worker), `portfolio_ref` (`Option<String>`, parent
portfolio `pid` — set on Project / Product / Program, absent on
Portfolio), `status` (`Option<WorkItemStatus>`), `goals` (`Vec<Goal>`),
`start_date` (`Option<Date>`), `target_date` (`Option<Date>`),
`keywords`, `tags` (`Vec<String>`, default empty), `identifiers`
(`WorkItemIdentifier { scheme, value }`), `same_as`, `in_language`,
`relationships` (`WorkItemRelationship { relation, work_item_id }`).

`kind: WorkItemKind` is **required** and is the match gate (§5, §12).
`WorkItemKind`: `Portfolio`, `Project`, `Product`, `Program`. The set is
**closed** — it is **not** `#[non_exhaustive]` and carries **no**
`Custom` variant, because it maps to a fixed set of tables/collections.

`Goal { title: String, description: Option<String>, target_date:
Option<Date>, status: Option<GoalStatus> }` where `GoalStatus` is an
enum: `NotStarted`, `InProgress`, `Achieved`,
`Missed`, `Custom(String)`. Only the goal **titles** feed matching
(§10); `description`, per-goal `target_date`, and `status` are
informational-only — serialized for callers but never read by the
matcher.

`WorkItemStatus`: `Proposed`, `Active`, `OnHold`, `Completed`,
`Cancelled`, `Custom(String)`. `status` is informational-only — not a
matching signal (two records of the same initiative routinely sit at
different statuses).

`portfolio_ref: Option<String>` is the parent portfolio's `pid` for
Project / Product / Program records (the umbrella link); it is absent
and ignored for the Portfolio kind. It is an **exact-match supporting
signal** for the child kinds (§11b), never a fuzzy comparison.

`tags: Vec<String>` holds operator-applied free-text labels for
grouping / workflow (e.g. `vip`, `review`, `q3`); each is whitespace-
trimmed, non-empty, and the set is de-duplicated case-insensitively.
Distinct from `keywords` (descriptive / discovery terms about *what the
record is*): tags are user-applied operational labels. A supporting
signal, not an identifying field on its own (§13.2).

`relationships: Vec<WorkItemRelationship>` holds typed work-item-to-
work-item references — `WorkItemRelationship { relation: RelationKind,
work_item_id: String }` where `RelationKind` is an
enum mirroring the service: `ParentOf` / `ChildOf` (hierarchy inverses),
`DependsOn` / `BlockedBy` (dependency inverses), `Supersedes` /
`SupersededBy` (versioning inverses), `SimilarTo` (symmetric),
`RelatedTo` (symmetric), plus `Custom(String)`. `work_item_id` is an
opaque registry id (whitespace-trimmed, non-empty); the matcher does
**not** resolve, invert, or transitively close the references — it
compares the two records' relationship **sets** (§13.1). A supporting
signal, not an identifying field on its own.

`IdentifierScheme`: deterministic (globally unique) — `Uri`, `Uuid`,
`JiraProjectKey`, `AsanaGid`, `TrelloBoardId`, `MsProjectId`,
`GitHubProjectId`, `LinearId`; owner-scoped — `Code`, `LocalId`; plus
`Custom(String)`.

`owner_org_name` is **informational-only**: it is serialized for callers
but never read for the owner gate. The code gate (§11, R-1) and the
owner-org component (§11a) key solely on `owner_org_id`, so two records
can only share an owner scope via that opaque id, not via a fuzzy
organisation-name comparison. `lead_ref` is likewise informational-only.

## 7. Configuration

`MatchConfig` weights (default, sum 1.0): name 0.30, goals 0.15,
code 0.15, owner_org 0.10, **portfolio 0.08**, timeframe 0.07, keywords
0.05, `relationships_weight` 0.05 (§13.1), `tags_weight` 0.05 (§13.2).
The weighted average is renormalised over the components actually
present (§17), so the supporting weights never break the
renormalisation. Threshold 0.85. Presets: `strict()` 0.95, `lenient()`
0.70.

The `portfolio` weight (0.08) **replaces** the `plan_type` weight the
ancestor `plan-matcher` carried: `kind` is now a hard gate (§5, §12),
not a weighted component, and the freed weight funds the parent-portfolio
corroboration signal (§11b).

Changing any weight (including `relationships_weight` or `tags_weight`)
is a config-section + `CHANGELOG.md` edit in the same PR (§25).

## 8. Normalisation

`fold` (trim + NFKC + lowercase, diacritic-preserving); `code`
(alphanumeric-only, uppercased — so `"PROJ-01"` ≡ `"proj 01"`);
`fold_set` (fold + sort + dedupe). Goal titles and keywords are compared
through `fold_set`.

## 9. Name similarity

Best Jaro-Winkler over `name` + `alternate_names` (folded), with a
Soundex +0.05 bonus on the primary names capped at 0.95. `name` is
required, so this component is **always** present.

## 10. Goals

Jaccard over the `fold_set` of goal **titles**. A strong descriptive
signal — the same initiative tends to repeat its headline outcomes.
Skipped when both sides have no goals; `0.0` when exactly one side
carries goals.

## 11. Code

Within the same non-empty `owner_org_id`: 1.0 if normalised codes equal,
else 0.0. Across owners (or missing owner): `None` (a local code like
`PROJ-01` is not globally unique).

### 11a. Owner org

`owner_org_id` case-folded exact match → 1.0 else 0.0. `None` when
either side is unset. Keys solely on the opaque id, never on
`owner_org_name`.

### 11b. Portfolio

Parent-portfolio corroboration for the **child kinds** (`Project` /
`Product` / `Program`): when both sides carry a non-empty
`portfolio_ref`, 1.0 if the (case-folded) parent portfolio `pid`s are
equal else 0.0. `None` (does not participate) when either side is unset,
which is always the case for the `Portfolio` kind (a portfolio has no
parent portfolio). An exact-match **supporting** signal weighted
`portfolio` (§7, default `0.08`); two children sharing a parent
corroborate but do not by themselves establish a match. The matcher
never fuzzy-matches `portfolio_ref`.

## 12. Kind gate & timeframe

`kind` is **not** a weighted component. It is the **R-GATE** (§5): the
very first rule, evaluated before R-0/R-1/R-2 and every probabilistic
component. If `A.kind != B.kind` the matcher returns `0.0`
(`is_match = false`, `Confidence::Low`, an all-`None` breakdown flagged
as a kind mismatch) — two work items of different kind are distinct
record types in distinct collections and never match. If
`A.kind == B.kind` the gate is transparent and matching proceeds. This
replaces the ancestor's `plan_type` exact-enum component, which is gone.

`timeframe_score`: **date proximity** over `start_date` / `target_date`.
The two records' available dates are compared pairwise (start↔start,
target↔target) by a **Gaussian decay** on the day gap — `exp(-(Δdays /
σ)² / 2)` — averaged over the date pairs both sides carry, with a
configurable `σ` (default 90 days). `None` when neither side carries a
date that the other side also carries (no comparable pair). Gaussian
decay is chosen over hard window-overlap so that near-miss dates degrade
smoothly rather than snapping to 0.

## 13. Keywords

Jaccard over `fold_set(keywords)`. Skipped when both empty; `0.0` when
exactly one side is populated.

### 13.1 Relationships — `relationships_score`

Typed-set **Jaccard** over the `(relation, work_item_id)` pairs: `score =
|A ∩ B| / |A ∪ B|`, where each side's set is `{ (r.relation,
r.work_item_id) for r in relationships }`. The relation kind is part of
the key, so a `Supersedes` reference only agrees with a `Supersedes`
reference to the **same** work-item id; `ParentOf` / `ChildOf` /
`DependsOn` / `BlockedBy` / `SupersededBy` / `SimilarTo` / `RelatedTo`
are compared as opaque, distinct kinds (no inversion or transitive
closure). `None` (does not participate) when **either** side has no
relationships; otherwise a value in `[0.0, 1.0]`. A **supporting**
signal weighted `relationships_weight` (§7, default `0.05`); shared
references never single-handedly establish a match.

### 13.2 Tags — `tags_score`

Plain set **Jaccard** over the case-insensitively normalised tag sets:
`tags_score = |A ∩ B| / |A ∪ B|`, where each side's set is the
`fold`-normalised, de-duplicated `tags`. `None` (does not participate)
when **either** side has an empty tag set; otherwise a value in `[0.0,
1.0]`. Identical in shape to `keywords_score` (§13) — a **supporting**
signal weighted `tags_weight` (§7, default `0.05`); shared tags never
single-handedly establish a match.

## 14. (reserved)

## 15. Deterministic identifier short-circuits

R-0: any shared value on a deterministic scheme → 1.0 (only when the
kinds already agree — R-GATE precedes R-0, §5). Empty values ignored.
Deterministic schemes: `Uri`, `Uuid`, `JiraProjectKey`, `AsanaGid`,
`TrelloBoardId`, `MsProjectId`, `GitHubProjectId`, `LinearId`. `Code` /
`LocalId` / `Custom` are excluded (owner-scoped or free-form, not
globally unique).

## 16. Owner+code, same_as, and open questions

R-1: shared non-empty `owner_org_id` + equal normalised `code` → 1.0.
R-2: any case-folded `same_as` URL overlap → 1.0. Both presuppose the
kinds agree (R-GATE precedes them, §5).

Open questions: should a goal-title exact overlap alone be a strong pin
(currently probabilistic — many work items share a headline goal)?
Should a `portfolio_ref` mismatch between two children *penalise* rather
than just not corroborate? Should the timeframe `σ` differ by `kind` (a
project's months vs. a portfolio's years)? Should a shared `same_as`
URL across *different* kinds be allowed to escape R-GATE (currently it
cannot — the gate is absolute)?

## 17. Renormalisation

Weighted average over `Some` components only; divisor is the sum of
contributing weights.

## 18. Confidence classification

`High` ≥ 0.95, `Medium` ≥ 0.70, else `Low`. Separate from
`MatchConfig::threshold` (`is_match`). A kind mismatch (R-GATE) yields
`Low` at score `0.0`.

## 19. Quality goals

Total functions (no `unwrap`/`expect`/`panic`); no `unsafe`;
deterministic; explainable; diacritic-correct.

## 20. Consumption

`portfolio-service` embeds this crate directly: the crate's `WorkItem`
type **is** the API DTO, the persisted JSONB payload, and the match
input (no adapter) — the same posture as care-pathway. The four
collections (`portfolios` / `projects` / `products` / `programs`) all
store and compare `WorkItem`; matching is within a collection only
(enforced by R-GATE — the service never matches a project against a
product). A bridge test in the service pins the contract.

## 21. Compatibility

Semantic versioning. Re-exports from `lib.rs` are the contract:
`WorkItem`, `WorkItemIdentifier`, `IdentifierScheme`, `WorkItemKind`,
`WorkItemStatus`, `Goal`, `GoalStatus`, `WorkItemRelationship`,
`RelationKind`, `MatchingEngine`, `MatchConfig`, `MatchResult`,
`MatchBreakdown`, `Confidence`, `Error`, `Result`.

`Error`/`Result` are **reserved for future fallible APIs**: every
current entry point (`match_work_items` and all component fns) is total
and returns `MatchResult` directly, so nothing produces an `Error`
today. They remain part of the SemVer surface so a future fallible path
(e.g. validated construction) can be added without a breaking re-export.

## 22. Anti-patterns

Never match across kinds — R-GATE is absolute (a project is never a
product). Do not short-circuit on owner-scoped or free-form schemes
(`Code` / `LocalId` / `Custom`). Do not score a `code` across owners. Do
not match on `status` (it drifts between duplicate records). Do not
strip diacritics. Do not add IO, async, or panics to library code.

## 23. Tasks (live work queue)

> **Status: implemented (v0.1.0).** The crate is built, `cargo test`
> green (55 unit + 10 integration + 7 doctests), `clippy --all-targets
> --all-features -- -D warnings` clean, `cargo fmt` clean, zero
> `#[allow]`. The boxes below are checked accordingly.

- [x] Implement the domain model in code: `WorkItem`, `WorkItemKind`
      (closed — **not** `#[non_exhaustive]`, no `Custom`), `Goal` /
      `GoalStatus`, `WorkItemStatus`, `WorkItemIdentifier` /
      `IdentifierScheme`, `WorkItemRelationship` / `RelationKind`, with
      serde derives and `WorkItem::new(kind, name)`. (The supporting
      enums are plain — **not** `#[non_exhaustive]` — matching the
      `case-matcher` sibling house style so the service can match them
      without wildcard arms.)
- [x] Implement the **R-GATE** (§12): `A.kind != B.kind` short-circuits
      to `0.0` before every other rule, with an all-`None` breakdown
      flagged `kind_gate_blocked`.
- [x] Implement deterministic short-circuits: R-0 (deterministic scheme
      overlap), R-1 (same owner + code), R-2 (`same_as` overlap), and
      `IdentifierScheme::is_deterministic` — all gated behind R-GATE.
- [x] Implement the probabilistic components: name (Jaro-Winkler +
      Soundex), goals (Jaccard over folded titles, §10), code (§11),
      owner_org (§11a), portfolio (§11b), timeframe (Gaussian decay,
      §12), keywords (§13), with the weighted average renormalised over
      present components (§17).
- [x] Implement `relationships` (§13.1): the typed-set Jaccard component,
      the `relationships_score` field on `MatchBreakdown`, and
      `relationships_weight` (default `0.05`) on `MatchConfig`.
- [x] Implement `tags` (§13.2): the set-Jaccard component (`None` when
      either side empty), the `tags_score` field on `MatchBreakdown`, and
      `tags_weight` (default `0.05`) on `MatchConfig`.
- [x] Re-export the public surface from `lib.rs` (§21). *(The
      `portfolio-service` bridge test lands with the service crate's
      `tests/matching.rs`.)*
- [ ] Optional: per-`kind` timeframe `σ`; ordered goal-sequence
      similarity; `lead_ref` corroboration.
- [ ] Split this single `spec/index.md` into the numbered §-per-file
      layout used by the sibling matcher crates.

## 24. Testing strategy

Unit tests embedded per module; an integration suite
(`tests/public_api.rs`) over the re-exported surface; rustdoc examples
run as doctests. Gate (mirrors CI): `cargo test`, `cargo clippy
--all-targets --all-features -- -D warnings`, `cargo fmt --check`.
Library code carries **no** `#[allow(clippy::…)]` attributes — it is
clippy-clean without suppressions (repo-wide invariant). The R-GATE
cross-kind no-match has dedicated coverage (every kind pair → `0.0`).

## 25. Change control

Update this spec in the same PR as any behavioural change; bump
`CHANGELOG.md` under `[Unreleased]`.
