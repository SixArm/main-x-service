# project-portfolio-management-matcher — Specification

> **Single source of truth.** Code conforms to this spec. A behavioural
> change is a three-part PR: spec edit + code edit + test edit. Live
> work queue is §23; open questions are §16.

## 1. Purpose

`project-portfolio-management-matcher` is a reusable, dependency-light
Rust library for **pairwise plan record matching**. A *plan* is a named
unit of intended work — with goals and a timeframe — that may contain
other plans, forming a recursive tree, tracked in a portfolio /
project-management registry. The canonical matchable type is `Plan`,
carrying an optional descriptive `kind: Option<PlanKind>` (`Portfolio` |
`Project` | `Product` | `Program` | `Practice` | `Process` | `Purpose` |
`Pathway` | `Proposal`) used only as a display / grouping label. Given
two `Plan` records the matcher returns a `MatchResult`: score in `[0.0,
1.0]`, `Confidence`, `is_match`, and a per-component `MatchBreakdown`.
It is the canonical algorithm embedded in
`project-portfolio-management-service`'s matching layer for
deduplication **across the single plan collection**.

The four former kinds (Portfolio / Project / Product / Program) were
**unified into one recursive plan tree** stored in one collection /
table (`plans`); any plan may contain any other via `parent_ref`.
`kind` survives only as **optional descriptive metadata** (a display /
grouping label, since extended with `Practice` / `Process` / `Purpose` /
`Pathway` / `Proposal`). The
matcher uses **one** comparison core with **no kind gate**: any two
plans may match regardless of their (optional) `kind`.

## 2. Scope

In scope: the attributes that distinguish one plan from another —
name, goals, owner-scoped code, owning organisation, parent plan,
timeframe, keywords, tags, relationships, and tool/registry
identifiers. Out of scope: the full plan content (task breakdown,
resourcing, Gantt scheduling, status history, issues), person-level
assignment data, and anything requiring IO, a runtime, or network
access. The operational sub-resources (tasks / issues, and goals beyond
their titles) belong to the service, not the matcher.

## 3. Glossary

- **Plan** — a named unit of intended work, with goals and a
  timeframe, that may contain other plans (`parent_ref`).
- **Kind** — the optional `PlanKind` label (`Portfolio` / `Project` /
  `Product` / `Program` / `Practice` / `Process` / `Purpose` /
  `Pathway` / `Proposal`); descriptive metadata only. It does **not**
  gate matching and does **not** fix a collection.
- **Parent ref** — `parent_ref`, the containment link: any plan may
  reference any other as its parent (a recursive tree).
- **Deterministic identifier** — globally unique (URI, UUID, Jira
  project key, Asana GID, Trello board id, MS Project id, GitHub
  project id, Linear id). A match pins the score to `1.0`.
- **Owner-scoped code** — `code`/`Code`/`LocalId`; only unique within
  the issuing organisation.
- **Goal** — a discrete intended outcome of the plan; its
  **title** is the matchable surface.

## 4. Research basis

Plans are largely identified by the tool that tracks them (Jira,
Asana, Trello, MS Project, GitHub Projects, Linear) and by their owning
organisation, name, and goals. The same initiative is frequently
re-entered across tools or teams, so matching combines deterministic
linkage on the tool/registry identifiers with fuzzy comparison of the
name and overlap of the goal titles, owner, parent plan, timeframe,
keywords, tags, and relationships. The registry holds all plans in
one recursive collection, so the matcher compares any two records
regardless of their (optional) `kind` — there is no cross-kind refusal.

## 5. Algorithm overview

```
Input: Plan A, Plan B, MatchConfig
  ├─ R-0 deterministic identifier match?        ─yes─> 1.0
  ├─ R-1 same owner + code?                     ─yes─> 1.0
  ├─ R-2 same_as URL overlap?                   ─yes─> 1.0
  │
  ├─ name_score          (always)   Jaro-Winkler + Soundex bonus
  ├─ goals_score         (≥1 set)   Jaccard over folded goal titles
  ├─ code_score          (same owner)  1.0/0.0
  ├─ owner_org_score     (both set) case-folded exact (1.0/0.0)
  ├─ parent_score        (both set) same parent_ref exact (1.0/0.0)
  ├─ timeframe_score     (dates set) date proximity (Gaussian decay)
  ├─ keywords_score      (≥1 set)   Jaccard
  ├─ relationships_score (≥1 set)   typed-set Jaccard over (relation, plan_id)
  ├─ tags_score          (both set) set Jaccard over normalised tags
  └─ renormalised weighted average over present components
```

**There is no kind gate.** The four kinds were unified into one
recursive plan tree, so any two plans may match regardless of
their (optional, descriptive) `kind`. The `MatchBreakdown` retains a
vestigial `kind_gate_blocked` field, now always `false`. See §12.

## 6. Domain model

The canonical domain model lives in the **entity-level spec**
([`../../spec/index.md`](../../spec/index.md) §5); this section restates
only the matcher-relevant surface. The crate's `Plan` type **is**
that model (§20) — the matcher type, the API DTO, and the persisted
JSONB payload are one shape, no adapter.

`Plan`: `kind` (`Option<PlanKind>`, optional descriptive label),
`name` (required),
`alternate_names`, `code` (`Option<String>`, owner-scoped),
`owner_org_id` (`Option<String>`, EntityRef organization),
`owner_org_name` (`Option<String>`), `lead_ref` (`Option<String>`,
EntityRef person/worker), `parent_ref` (`Option<String>`, parent
plan `pid` — the containment link; any plan may reference any other),
`status` (`Option<PlanStatus>`), `goals` (`Vec<Goal>`),
`start_date` (`Option<Date>`), `target_date` (`Option<Date>`),
`keywords`, `tags` (`Vec<String>`, default empty), `identifiers`
(`PlanIdentifier { scheme, value }`), `same_as`, `in_language`,
`relationships` (`PlanRelationship { relation, plan_id }`).

`kind: Option<PlanKind>` is **optional descriptive metadata** — a
display / grouping label that does **not** gate matching (§5, §12).
`PlanKind`: `Portfolio`, `Project`, `Product`, `Program`, `Practice`,
`Process`, `Purpose`, `Pathway`, `Proposal`. The set is **closed** — it
is **not** `#[non_exhaustive]` and carries **no** `Custom` variant.

`Goal { title: String, description: Option<String>, target_date:
Option<Date>, status: Option<GoalStatus> }` where `GoalStatus` is an
enum: `NotStarted`, `InProgress`, `Achieved`,
`Missed`, `Custom(String)`. Only the goal **titles** feed matching
(§10); `description`, per-goal `target_date`, and `status` are
informational-only — serialized for callers but never read by the
matcher.

`PlanStatus`: `Proposed`, `Active`, `OnHold`, `Completed`,
`Cancelled`, `Custom(String)`. `status` is informational-only — not a
matching signal (two records of the same initiative routinely sit at
different statuses).

`parent_ref: Option<String>` is the containing plan's `pid` — the
general containment link, by which any plan may reference any other as
its parent (a recursive tree). It is an **exact-match supporting
signal** for every plan that carries one (§11b), never a fuzzy comparison.

`tags: Vec<String>` holds operator-applied free-text labels for
grouping / workflow (e.g. `vip`, `review`, `q3`); each is whitespace-
trimmed, non-empty, and the set is de-duplicated case-insensitively.
Distinct from `keywords` (descriptive / discovery terms about *what the
record is*): tags are user-applied operational labels. A supporting
signal, not an identifying field on its own (§13.2).

`relationships: Vec<PlanRelationship>` holds typed plan-to-
plan references — `PlanRelationship { relation: RelationKind,
plan_id: String }` where `RelationKind` is an
enum mirroring the service: `ParentOf` / `ChildOf` (hierarchy inverses),
`DependsOn` / `BlockedBy` (dependency inverses), `Supersedes` /
`SupersededBy` (versioning inverses), `SimilarTo` (symmetric),
`RelatedTo` (symmetric), plus `Custom(String)`. `plan_id` is an
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
code 0.15, owner_org 0.10, **parent 0.08**, timeframe 0.07, keywords
0.05, `relationships_weight` 0.05 (§13.1), `tags_weight` 0.05 (§13.2).
The weighted average is renormalised over the components actually
present (§17), so the supporting weights never break the
renormalisation. Threshold 0.85. Presets: `strict()` 0.95, `lenient()`
0.70.

The `parent` weight (0.08) **replaces** the `plan_type` weight the
ancestor `plan-matcher` carried: `kind` is now optional descriptive
metadata (§5, §12), not a weighted component and not a gate, and the
freed weight funds the parent-plan corroboration signal (§11b).

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

### 11b. Parent

Parent-plan corroboration for **any** plan carrying a containment link:
when both sides carry a non-empty `parent_ref`, 1.0 if the (case-folded)
parent plan `pid`s are equal else 0.0. `None` (does not participate) when
either side is unset (e.g. a root plan with no parent). An exact-match
**supporting** signal weighted `parent` (§7, default `0.08`); two plans
sharing a parent corroborate but do not by themselves establish a match.
The matcher never fuzzy-matches `parent_ref`.

## 12. Kind (no gate) & timeframe

`kind` is **not** a weighted component and **not** a gate. Since the four
kinds were unified into one recursive plan tree, `kind` is only optional
descriptive metadata (a display / grouping label) and never affects a
match: any two plans may match regardless of their (optional) `kind`, and
`kind` is never compared. The `MatchBreakdown` retains a vestigial
`kind_gate_blocked` field, now always `false` (§5). This replaces the
ancestor's `plan_type` exact-enum component, which is gone.

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

Typed-set **Jaccard** over the `(relation, plan_id)` pairs: `score =
|A ∩ B| / |A ∪ B|`, where each side's set is `{ (r.relation,
r.plan_id) for r in relationships }`. The relation kind is part of
the key, so a `Supersedes` reference only agrees with a `Supersedes`
reference to the **same** plan id; `ParentOf` / `ChildOf` /
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

R-0: any shared value on a deterministic scheme → 1.0, regardless of the
two records' (optional) `kind`. Empty values ignored.
Deterministic schemes: `Uri`, `Uuid`, `JiraProjectKey`, `AsanaGid`,
`TrelloBoardId`, `MsProjectId`, `GitHubProjectId`, `LinearId`. `Code` /
`LocalId` / `Custom` are excluded (owner-scoped or free-form, not
globally unique).

## 16. Owner+code, same_as, and open questions

R-1: shared non-empty `owner_org_id` + equal normalised `code` → 1.0.
R-2: any case-folded `same_as` URL overlap → 1.0. Both apply regardless
of the two records' (optional) `kind`.

Open questions: should a goal-title exact overlap alone be a strong pin
(currently probabilistic — many plans share a headline goal)?
Should a `parent_ref` mismatch between two plans *penalise* rather
than just not corroborate? Should the timeframe `σ` differ by `kind` (a
project's months vs. a portfolio's years)?

## 17. Renormalisation

Weighted average over `Some` components only; divisor is the sum of
contributing weights.

## 18. Confidence classification

`High` ≥ 0.95, `Medium` ≥ 0.70, else `Low`. Separate from
`MatchConfig::threshold` (`is_match`).

## 19. Quality goals

Total functions (no `unwrap`/`expect`/`panic`); no `unsafe`;
deterministic; explainable; diacritic-correct.

## 20. Consumption

`project-portfolio-management-service` embeds this crate directly: the crate's `Plan`
type **is** the API DTO, the persisted JSONB payload, and the match
input (no adapter) — the same posture as care-pathway. All plans live
in one collection / table (`plans`) and are stored and compared as
`Plan`; matching spans the whole collection regardless of a record's
(optional) `kind` (there is no kind gate). A bridge test in the service
pins the contract.

## 21. Compatibility

Semantic versioning. Re-exports from `lib.rs` are the contract:
`Plan`, `PlanIdentifier`, `IdentifierScheme`, `PlanKind`,
`PlanStatus`, `Goal`, `GoalStatus`, `PlanRelationship`,
`RelationKind`, `MatchingEngine`, `MatchConfig`, `MatchResult`,
`MatchBreakdown`, `Confidence`, `Error`, `Result`.

`Error`/`Result` are **reserved for future fallible APIs**: every
current entry point (`match_plans` and all component fns) is total
and returns `MatchResult` directly, so nothing produces an `Error`
today. They remain part of the SemVer surface so a future fallible path
(e.g. validated construction) can be added without a breaking re-export.

## 22. Anti-patterns

Never reintroduce a kind gate — `kind` is optional descriptive metadata,
and two plans with different kinds may still be the same identity. Do not
compare `kind` as a matching signal. Do not short-circuit on owner-scoped or free-form schemes
(`Code` / `LocalId` / `Custom`). Do not score a `code` across owners. Do
not match on `status` (it drifts between duplicate records). Do not
strip diacritics. Do not add IO, async, or panics to library code.

## 23. Tasks (live work queue)

> **Status: implemented (v0.1.0).** The crate is built, `cargo test`
> green (57 unit + 10 integration (`tests/public_api.rs`) + 6 property
> (`tests/property_tests.rs`, SEC-M6) + 7 doctests), `clippy
> --all-targets --all-features -- -D warnings` clean, `cargo fmt`
> clean, zero `#[allow]`. A standalone `cargo-fuzz` harness (`fuzz/`,
> SEC-I2, two targets) runs separately on nightly — not part of the
> stable `cargo test` count above. The boxes below are checked
> accordingly.

- [x] **2026-07-22 — Extend `PlanKind` with `Practice`, `Process`,
  `Purpose`, `Pathway`, `Proposal`.** Additive variants on the closed
  set (§6.2); `kind` stays optional descriptive metadata and is still
  never a match gate, so scoring is unchanged. Pinned by the
  kind-never-gates-matching integration test (every ordered pair of
  the nine labels) and the property-test kind strategy.

- [x] Implement the domain model in code: `Plan`, `PlanKind`
      (closed — **not** `#[non_exhaustive]`, no `Custom`), `Goal` /
      `GoalStatus`, `PlanStatus`, `PlanIdentifier` /
      `IdentifierScheme`, `PlanRelationship` / `RelationKind`, with
      serde derives and `Plan::new(name)` (`kind` defaults to `None`).
      (The supporting enums are plain — **not** `#[non_exhaustive]` —
      matching the `case-matcher` sibling house style so the service can
      match them without wildcard arms.)
- [x] Unify the four former kinds into one recursive plan tree and
      **remove the kind gate** (§12): any two plans may match regardless
      of their (optional) `kind`, which is never compared. The
      `MatchBreakdown.kind_gate_blocked` field is retained but vestigial
      (always `false`).
- [x] Implement deterministic short-circuits: R-0 (deterministic scheme
      overlap), R-1 (same owner + code), R-2 (`same_as` overlap), and
      `IdentifierScheme::is_deterministic` — all independent of `kind`.
- [x] Implement the probabilistic components: name (Jaro-Winkler +
      Soundex), goals (Jaccard over folded titles, §10), code (§11),
      owner_org (§11a), parent (§11b), timeframe (Gaussian decay,
      §12), keywords (§13), with the weighted average renormalised over
      present components (§17).
- [x] Implement `relationships` (§13.1): the typed-set Jaccard component,
      the `relationships_score` field on `MatchBreakdown`, and
      `relationships_weight` (default `0.05`) on `MatchConfig`.
- [x] Implement `tags` (§13.2): the set-Jaccard component (`None` when
      either side empty), the `tags_score` field on `MatchBreakdown`, and
      `tags_weight` (default `0.05`) on `MatchConfig`.
- [x] Re-export the public surface from `lib.rs` (§21). *(The
      `project-portfolio-management-service` bridge test lands with the service crate's
      `tests/matching.rs`.)*
- [x] **SEC-M2** — a bare root `same_as` URL (`"/"`) no longer forces a
      deterministic match; `R-2` skips it.
- [x] **SEC-M4** — bound the year in `normalize::iso_date_to_days` to
      `0..=9999` so a crafted long-year date cannot overflow
      `days_from_civil`.
- [x] **SEC-M6** — `tests/property_tests.rs`: `proptest` properties
      proving never-panic, bounded/finite score, same-kind symmetry,
      no-kind-gate, and self-match reflexivity over arbitrary input.
- [x] **SEC-I2** — `fuzz/`: a standalone `cargo-fuzz` crate with
      `match_plans` and `normalize` libFuzzer targets (nightly-only,
      not part of the stable `cargo test` gate).
- [ ] Optional: per-`kind` timeframe `σ`; ordered goal-sequence
      similarity; `lead_ref` corroboration.
- [ ] Split this single `spec/index.md` into the numbered §-per-file
      layout used by the sibling matcher crates.

## 24. Testing strategy

Unit tests embedded per module; an integration suite
(`tests/public_api.rs`) over the re-exported surface; a `proptest`
property suite (`tests/property_tests.rs`, SEC-M6) proving never-panic,
bounded-score, symmetry, no-kind-gate, and reflexivity invariants over
arbitrary input; rustdoc examples run as doctests. A standalone,
nightly-only `cargo-fuzz` harness (`fuzz/`, SEC-I2, two libFuzzer
targets — see [`AGENTS/testing.md`](../AGENTS/testing.md)) complements
`proptest` with coverage-guided search; it is not part of the stable
gate below. Gate (mirrors CI): `cargo test`, `cargo clippy
--all-targets --all-features -- -D warnings`, `cargo fmt --check`.
Library code carries **no** `#[allow(clippy::…)]` attributes — it is
clippy-clean without suppressions (repo-wide invariant). Dedicated
coverage pins that `kind` does not gate matching (two plans with
different kinds still match on their other signals; `kind_gate_blocked`
is always `false`).

## 25. Change control

Update this spec in the same PR as any behavioural change; bump
`CHANGELOG.md` under `[Unreleased]`.
