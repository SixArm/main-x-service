## 5. Domain Model

This section is the **canonical home** of the portfolio domain model.
The matcher and service crate specs reference it rather than redefining
it. Two load-bearing ideas:

- **One recursive type, an optional label.** A single matchable type,
  `Plan`, carries an **optional** `kind: Option<PlanKind>` label
  (`Portfolio` | `Project` | `Product` | `Program` | `Practice` |
  `Process` | `Purpose` | `Pathway` | `Proposal`) used for description /
  display / grouping. Any plan may **contain** any other plan via
  `parent_ref` (a recursive tree). `kind` is **not** a discriminator: it
  does not map to a table / collection and does not gate matching —
  every plan lives in one `plans` table and matching is
  **kind-agnostic** (§5.5).
- **The partition.** The **thin matchable `Plan` record** (the matcher
  crate's `Plan` type) is the API DTO, the persisted JSONB payload, and
  the matching input — one shape end to end, **no separate service model
  and no adapter to drift** (exactly the care-pathway posture); the
  **operational sub-resources** (tasks, issues; and goals, which are
  *also* in the payload) are high-volume child data, held in **separate
  service tables** keyed by `parent_pid`, and are **never** part of the
  matcher payload (§5.6).

### 5.1 Canonical `Plan` (matcher crate) — the thin matchable record

Defined in `project-portfolio-management-matcher-rust-crate/src/plan.rs`; normative
reference: matcher
[spec §6](../project-portfolio-management-matcher-rust-crate/spec/index.md).

| Field | Type | Notes |
|---|---|---|
| `kind` | Option\<PlanKind\> | **Optional** descriptive label `Portfolio` \| `Project` \| `Product` \| `Program` \| `Practice` \| `Process` \| `Purpose` \| `Pathway` \| `Proposal`; not a discriminator, not a match gate (§5.5); defaults to `None` |
| `name` | String | Required (service rejects blank) |
| `alternate_names` | Vec\<String\> | Aliases, former titles, codenames |
| `code` | Option\<String\> | Owner-scoped code, e.g. `PROJ-2026` |
| `owner_org_id` | Option\<String\> | `EntityRef` `organization:<id>` — sponsoring / owning org (scopes `code`) |
| `owner_org_name` | Option\<String\> | Owning organisation display name (informational-only) |
| `lead_ref` | Option\<String\> | `EntityRef` `person:<id>` \| `worker:<id>` — the lead |
| `parent_ref` | Option\<String\> | Parent plan `pid` (the containment link); an exact supporting match signal (§5.5); absent for a root plan |
| `status` | Option\<PlanStatus\> | See enum below — **informational-only**, not a match signal |
| `goals` | Vec\<Goal\> | Plan objectives — **part of the payload**; goal *titles* feed matching (§5.4) |
| `start_date` | Option\<Date\> | Planned / actual start |
| `target_date` | Option\<Date\> | Planned completion / due date |
| `keywords` | Vec\<String\> | Descriptive / discovery terms (what the plan *is*) |
| `tags` | Vec\<String\> | Operator-applied labels for grouping / workflow — see below |
| `identifiers` | Vec\<PlanIdentifier\> | `{ scheme: IdentifierScheme, value: String }` |
| `same_as` | Vec\<String\> | Canonical URLs (schema.org `sameAs`) |
| `in_language` | Option\<String\> | ISO 639-1 code — see [`agents/share/locales.md`](../../agents/share/locales.md) |
| `relationships` | Vec\<PlanRelationship\> | Typed plan-to-plan links — `{ relation: RelationKind, plan_id: String }` |

**`EntityRef` fields.** `owner_org_id`, `lead_ref` (and the
sub-resources' `assignee_ref`) hold an **`EntityRef` URN** —
`<entity_type>:<id>` — per
[`agents/share/cross-service-linking.md` §3](../../agents/share/cross-service-linking.md).
They are references, not matching strings: `owner_org_id` is an exact
match signal (§5.5), `lead_ref` is **not** scored (federation
boundary). They are stored as plain strings; the service does not call
the target service on the write path. `parent_ref` is a parent
plan's `pid` (an in-entity reference into the `plans`
collection), scored as an exact supporting signal (§5.5).

### 5.2 Supporting enums

- `PlanKind`: `Portfolio`, `Project`, `Product`, `Program`, `Practice`,
  `Process`, `Purpose`, `Pathway`, `Proposal`. An **optional** label on
  every `Plan` (the field is `Option<PlanKind>`, defaulting to `None`);
  it is descriptive only — it does not map to a table / collection and
  does not gate matching.
- `PlanStatus`: `Proposed`, `Active`, `OnHold`, `Completed`,
  `Cancelled`, `Custom(String)`. Informational-only — never scored.
- `GoalStatus`: `NotStarted`, `InProgress`, `Achieved`, `Missed`,
  `Custom(String)`.
- `IdentifierScheme`:
  - **deterministic** (a shared value pins the match to 1.0 — R-0,
    §5.5): `Uri`, `Uuid`, `JiraProjectKey`, `AsanaGid`,
    `TrelloBoardId`, `MsProjectId`, `GitHubProjectId`, `LinearId`;
  - **owner-scoped** (never globally unique, excluded from R-0):
    `Code`, `LocalId`;
  - plus `Custom(String)`.
- `RelationKind`: `ParentOf` / `ChildOf` (**inverses** — programme /
  portfolio hierarchy), `DependsOn` / `BlockedBy` (**inverses**),
  `Supersedes` / `SupersededBy` (**inverses**), `SimilarTo`
  (**symmetric**), `RelatedTo` (**symmetric**); plus `Custom(String)`.

### 5.3 `Goal` (in the payload)

`Goal { title: String, description: Option<String>, target_date:
Option<Date>, status: Option<GoalStatus> }`.

A `Goal` is the **one** sub-resource that is also a payload field:
goals describe *what the plan is trying to achieve*, which is
identity-bearing, so `goals[]` rides in the JSONB `data` and reaches
the matcher. The matcher scores the **folded set of goal titles** by
Jaccard (§5.4 / §6.3). The service additionally exposes goals as a CRUD
sub-resource (§6.4) so they can be managed without rewriting the whole
plan; goal writes update the same `data.goals[]` array (§10.2), so
the payload and the sub-resource never diverge.

### 5.4 Relationships, keywords, tags — the supporting signals

**Relationships** — typed plan-to-plan links:
`relationships: Vec<PlanRelationship>`, each `{ relation,
plan_id }` **referencing another `Plan` in the registry**.
`relation` is a `RelationKind`:

- **`ParentOf`** / **`ChildOf`** (**inverses** — programme/portfolio
  hierarchy: A `ParentOf` B ⇔ B `ChildOf` A);
- **`DependsOn`** / **`BlockedBy`** (**inverses** — A `DependsOn` B ⇔ B
  `BlockedBy` A);
- **`Supersedes`** / **`SupersededBy`** (**inverses** — charter
  versioning: A `Supersedes` B ⇔ B `SupersededBy` A);
- **`SimilarTo`** (**symmetric**), a comparable plan;
- **`RelatedTo`** (**symmetric**), a loosely associated plan.

Relationships are a **supporting** match signal — a typed-set Jaccard
over the `(relation, plan_id)` pairs — never an identifying field
on their own. They are distinct from the `parent_ref` containment link
(§5.1): `parent_ref` is the recursive containment tree and is its own
exact signal, while `relationships[]` are arbitrary within-entity links
(including a `ParentOf` / `ChildOf` hierarchy among plans).

**Keywords** are descriptive / discovery terms about *what the plan
is*. **Tags** (`tags: Vec<String>`) are short free-text operator
labels for grouping, filtering, triage, or workflow (e.g.
`priority-1`, `q3-review`, `archived-2026`, `fast-track`). **Any
`Plan` can carry tags.** Each tag is a short, trimmed, non-empty
string; the list is unordered, de-duplicated **case-insensitively**,
and defaults to empty. Tags are distinct from keywords — keywords are
domain vocabulary about what the record *is*; tags are **user-applied
operational labels**. The two coexist; neither replaces the other.

Both `keywords` and `tags` **are** supporting match signals: they
round-trip through the JSONB payload (§5.6) and reach the matcher
unchanged, scored as plain set Jaccard over the case-insensitively
normalised sets (`score = |A ∩ B| / |A ∪ B|`), each weighted (§6.3, FR-8).
Like `relationships`, they are **supporting** signals, never
identifying on their own; each contributes `None` (does not
participate) when either side's set is empty.

### 5.5 Match input — what makes two plans "the same"

**Kind-agnostic.** Matching compares two thin `Plan` records
regardless of their optional `kind` label — two plans may match whether
or not their labels agree (there is no kind gate). The defining
signals (§6.3 / FR-8 has the weight table) are:

- **Name** (+ alternate names) — the heaviest weight.
- **Goal titles** — the second signal; what the plan sets out to do.
- **Owner-scoped `code`** — exact, but **only within the same
  `owner_org_id`**; never matched across owners.
- **`owner_org_id`** — exact `EntityRef` match (same sponsor); skipped
  if either side is unset.
- **`parent_ref`** — exact parent-plan match (same container); skipped
  if either side is unset.
- **timeframe** (`start_date` / `target_date` proximity), **keywords**,
  **relationships**, **tags** — supporting.

`status` is **informational-only** and never scored; `kind` is a
descriptive label and carries no weight — it neither gates nor scores
(the old plan-family `plan_type` weight is gone). `lead_ref` and the
sub-resource `EntityRef`s are **not** scored: "same lead" is not
sameness evidence, and cross-service references are never a match signal
([`agents/share/cross-service-linking.md` §7](../../agents/share/cross-service-linking.md)).

### 5.6 Persistence model (JSONB) and the partition

The thin record is stored verbatim in one row of the `plans` table:

| Column | Type | Purpose |
|---|---|---|
| `id` | serial PK | Internal row id |
| `pid` | UUID unique | Public id (route param) |
| `name` | string | Denormalised from the payload for cheap listing |
| `data` | JSONB | The full thin `Plan` payload (incl. optional `kind`, `goals[]`) |
| `parent_pid` | UUID null | Denormalised `data.parent_ref` for cheap roll-up of a plan's children |
| `active` | boolean (default true) | Registry flag |
| `deleted_at` | timestamptz null | Soft delete |

`Model::to_plan()` deserialises `data` into the matcher type;
`Model::create()` / `update_data()` serialise it in. The `name` column
MUST equal `data.name`, and the `parent_pid` column MUST equal
`data.parent_ref` (the model layer writes them together).

Because the matcher's `Plan` **is** the persisted payload and the
matching input (no adapter), every field — including the optional
`kind`, `goals[]`, `relationships[]`, `keywords`, `tags`, `parent_ref`
— round-trips verbatim through `data` and reaches the matcher
unchanged. There is **no lossy-drop list**; the only fields outside the
JSONB payload are the registry-plumbing columns (`id`, `pid`, `active`,
`deleted_at`) and the two denormalised projections (`name`,
`parent_pid`).

**The partition.** The operational sub-resources — **tasks, issues**
— are **not** in `data` and **never** enter the matcher. They live in
their own tables keyed by `parent_pid` (§10.1), because
they are high-volume (a plan may have thousands of tasks) and are
not identity-bearing. `goals[]` is the sole crossover: it is in the
payload **and** exposed as a sub-resource, with goal writes mutating
`data.goals[]` (§10.2) so the two views stay consistent.

### 5.7 Front-end TypeScript types

The front-end mirrors the wire shape in `src/lib/api/types.ts`
(`Plan`, `PlanKind`, `Goal`, `PlanStatus`, `GoalStatus`,
`IdentifierScheme`, `PlanIdentifier`, `PlanRelationship`,
`RelationKind`, `PlanRef`, `ScoredRef`, plus the sub-resource types
`Task`, `Issue` and the derived `Timeline` / `Burndown` shapes). The
matcher type is upstream for the thin record: if a `Plan` field
changes in the matcher crate, the service inherits it automatically
(re-serialisation) and the front-end types MUST be fixed in the same
change cycle. The sub-resource types are owned by the service crate
spec (they have no matcher counterpart).

### 5.8 Shared invariants

All subprojects MUST uphold:

- `kind`, when present on a `Plan`, is one of the four label values; it
  is **optional** (`None` allowed) and does not fix a collection — every
  plan lives in the one `plans` table.
- Matching is **kind-agnostic**: two plans may match regardless of their
  optional `kind` labels (§5.5 / §6.3) — end to end, in the matcher and
  in every service endpoint.
- `name` is non-empty; the stored `name` column equals `data.name`.
- When `parent_ref` is set the denormalised `parent_pid` column equals
  it; a `parent_ref` may point at any other plan but never forms a
  containment cycle (points a plan at itself or a descendant → `422`,
  §6.1).
- The JSONB payload round-trips losslessly:
  `serde_json::from_value(to_value(w)) == w`.
- Owner-scoped codes (`code`, `Code`, `LocalId`) are never treated as
  globally unique — no cross-owner short-circuit, end to end. They
  short-circuit (R-1, §6.6) **only** within an equal, non-empty
  `owner_org_id`.
- `EntityRef` fields hold a valid `<entity_type>:<id>` URN or are
  absent; the service does not call the target service on the write path
  (optimistic, per [`agents/share/cross-service-linking.md`
  §5](../../agents/share/cross-service-linking.md)).
- A `PlanRelationship` references an **existing** `Plan`; **no plan
  relates to itself**. `ParentOf`/`ChildOf`, `DependsOn`/`BlockedBy`,
  and `Supersedes`/`SupersededBy` stay **acyclic** (no plan is its own
  ancestor / dependency / predecessor, directly or transitively) and
  **inverse-consistent** (A `ParentOf` B ⇔ B `ChildOf` A; likewise the
  other two pairs); `SimilarTo` and `RelatedTo` are **symmetric**.
- Each `tags` entry is short, trimmed, and non-empty; the list is
  de-duplicated case-insensitively and defaults to empty.
- Operational sub-resources (tasks, issues) are **never** serialised
  into `data` and **never** reach the matcher; `goals[]` is the only
  payload-and-sub-resource field, and goal writes mutate `data.goals[]`.
- A sub-resource always belongs to exactly one live (non-soft-deleted)
  plan; soft-deleting a plan hides its sub-resources from read paths.
- Cross-service links (`entity_links`) are never stored in
  `relationships` and never fed to any matcher
  ([cross-service-linking.md
  §7](../../agents/share/cross-service-linking.md)).
- Match scores are in `[0.00, 1.00]` and always travel with a
  per-component breakdown and `Confidence` band.
- Soft delete (`deleted_at`) is the only delete: the service never
  row-deletes, and the front-end never offers hard delete.
