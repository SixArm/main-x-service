## 5. Domain Model

This section is the **canonical home** of the portfolio domain model.
The matcher and service crate specs reference it rather than redefining
it. Two load-bearing ideas:

- **Four matchable kinds, one type.** A single matchable type,
  `WorkItem`, carries a required `kind: WorkItemKind` discriminator
  (`Portfolio` | `Project` | `Product` | `Program`). Portfolio is the
  **umbrella** kind; Project / Product / Program are **child** kinds
  that sit under a portfolio via `portfolio_ref`. The four kinds are
  **distinct record types** — each in its own service table and REST
  collection — and matching is **within a kind only** (the kind gate,
  §5.5). `WorkItemKind` is a **closed** set (no `Custom` arm) precisely
  because it maps to fixed tables / collections.
- **The partition.** The **thin matchable `WorkItem` record** (the
  matcher crate's `WorkItem` type) is the API DTO, the persisted JSONB
  payload, and the matching input — one shape end to end, **no separate
  service model and no adapter to drift** (exactly the care-pathway
  posture); the **operational sub-resources** (tasks, issues; and
  goals, which are *also* in the payload) are high-volume child data,
  held in **separate service tables** keyed by `(parent_kind,
  parent_pid)`, and are **never** part of the matcher payload (§5.6).

### 5.1 Canonical `WorkItem` (matcher crate) — the thin matchable record

Defined in `project-portfolio-management-matcher-rust-crate/src/work_item.rs`; normative
reference: matcher
[spec §6](../project-portfolio-management-matcher-rust-crate/spec/index.md).

| Field | Type | Notes |
|---|---|---|
| `kind` | WorkItemKind | **Required.** `Portfolio` \| `Project` \| `Product` \| `Program` — the collection / table it lives in; a hard match gate (§5.5) |
| `name` | String | Required (service rejects blank) |
| `alternate_names` | Vec\<String\> | Aliases, former titles, codenames |
| `code` | Option\<String\> | Owner-scoped code, e.g. `PROJ-2026` |
| `owner_org_id` | Option\<String\> | `EntityRef` `organization:<id>` — sponsoring / owning org (scopes `code`) |
| `owner_org_name` | Option\<String\> | Owning organisation display name (informational-only) |
| `lead_ref` | Option\<String\> | `EntityRef` `person:<id>` \| `worker:<id>` — the lead |
| `portfolio_ref` | Option\<String\> | Parent portfolio `pid` for a child kind (the umbrella link); an exact supporting match signal for child kinds (§5.5); absent / ignored for the `Portfolio` kind |
| `status` | Option\<WorkItemStatus\> | See enum below — **informational-only**, not a match signal |
| `goals` | Vec\<Goal\> | Work-item objectives — **part of the payload**; goal *titles* feed matching (§5.4) |
| `start_date` | Option\<Date\> | Planned / actual start |
| `target_date` | Option\<Date\> | Planned completion / due date |
| `keywords` | Vec\<String\> | Descriptive / discovery terms (what the work item *is*) |
| `tags` | Vec\<String\> | Operator-applied labels for grouping / workflow — see below |
| `identifiers` | Vec\<WorkItemIdentifier\> | `{ scheme: IdentifierScheme, value: String }` |
| `same_as` | Vec\<String\> | Canonical URLs (schema.org `sameAs`) |
| `in_language` | Option\<String\> | ISO 639-1 code — see [`agents/share/locales.md`](../../agents/share/locales.md) |
| `relationships` | Vec\<WorkItemRelationship\> | Typed work-item-to-work-item links — `{ relation: RelationKind, work_item_id: String }` |

**`EntityRef` fields.** `owner_org_id`, `lead_ref` (and the
sub-resources' `assignee_ref`) hold an **`EntityRef` URN** —
`<entity_type>:<id>` — per
[`agents/share/cross-service-linking.md` §3](../../agents/share/cross-service-linking.md).
They are references, not matching strings: `owner_org_id` is an exact
match signal (§5.5), `lead_ref` is **not** scored (federation
boundary). They are stored as plain strings; the service does not call
the target service on the write path. `portfolio_ref` is a parent
portfolio's `pid` (an in-entity reference into the `portfolios`
collection), scored as an exact supporting signal for child kinds
(§5.5).

### 5.2 Supporting enums

- `WorkItemKind`: `Portfolio`, `Project`, `Product`, `Program`. The
  set is **closed** — **no** `Custom` arm and **not**
  `#[non_exhaustive]` — because each variant maps to a fixed table /
  collection. The discriminator is required on every `WorkItem`.
- `WorkItemStatus`: `Proposed`, `Active`, `OnHold`, `Completed`,
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
goals describe *what the work item is trying to achieve*, which is
identity-bearing, so `goals[]` rides in the JSONB `data` and reaches
the matcher. The matcher scores the **folded set of goal titles** by
Jaccard (§5.4 / §6.3). The service additionally exposes goals as a CRUD
sub-resource (§6.4) so they can be managed without rewriting the whole
work item; goal writes update the same `data.goals[]` array (§10.2), so
the payload and the sub-resource never diverge.

### 5.4 Relationships, keywords, tags — the supporting signals

**Relationships** — typed work-item-to-work-item links:
`relationships: Vec<WorkItemRelationship>`, each `{ relation,
work_item_id }` **referencing another `WorkItem` in the registry**.
`relation` is a `RelationKind`:

- **`ParentOf`** / **`ChildOf`** (**inverses** — programme/portfolio
  hierarchy: A `ParentOf` B ⇔ B `ChildOf` A);
- **`DependsOn`** / **`BlockedBy`** (**inverses** — A `DependsOn` B ⇔
  B `BlockedBy` A);
- **`Supersedes`** / **`SupersededBy`** (**inverses** — charter
  versioning: A `Supersedes` B ⇔ B `SupersededBy` A);
- **`SimilarTo`** (**symmetric**), a comparable work item;
- **`RelatedTo`** (**symmetric**), a loosely associated work item.

Relationships are a **supporting** match signal — a typed-set Jaccard
over the `(relation, work_item_id)` pairs — never an identifying field
on their own. They are distinct from the `portfolio_ref` parent link
(§5.1): `portfolio_ref` is the umbrella hierarchy and is its own exact
signal, while `relationships[]` are arbitrary within-entity links
(including a `ParentOf` / `ChildOf` programme hierarchy among child
kinds).

**Keywords** are descriptive / discovery terms about *what the work
item is*. **Tags** (`tags: Vec<String>`) are short free-text operator
labels for grouping, filtering, triage, or workflow (e.g.
`priority-1`, `q3-review`, `archived-2026`, `fast-track`). **Any
`WorkItem` can carry tags.** Each tag is a short, trimmed, non-empty
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

### 5.5 Match input — what makes two work items "the same"

**The kind gate first.** Matching compares two thin `WorkItem` records
of the **same kind**. If `A.kind != B.kind` the matcher
short-circuits to **0.0** (no match) — a project and a product, or a
portfolio and a program, are distinct record types in distinct
collections and are never the same identity. This **kind gate
(R-GATE)** runs before every other rule (§6.3). Within a kind, the
defining signals (§6.3 / FR-8 has the weight table) are:

- **Name** (+ alternate names) — the heaviest weight.
- **Goal titles** — the second signal; what the work item sets out to
  do.
- **Owner-scoped `code`** — exact, but **only within the same
  `owner_org_id`**; never matched across owners.
- **`owner_org_id`** — exact `EntityRef` match (same sponsor); skipped
  if either side is unset.
- **`portfolio_ref`** — exact parent-portfolio match for child kinds
  (same umbrella); skipped if either side is unset or for the
  `Portfolio` kind.
- **timeframe** (`start_date` / `target_date` proximity), **keywords**,
  **relationships**, **tags** — supporting.

`status` is **informational-only** and never scored (it replaces no
weighted component; the old plan-family `plan_type` weight is gone —
kind is a gate, not a weight). `lead_ref` and the sub-resource
`EntityRef`s are **not** scored: "same lead" is not sameness evidence,
and cross-service references are never a match signal
([`agents/share/cross-service-linking.md` §7](../../agents/share/cross-service-linking.md)).

### 5.6 Persistence model (JSONB) and the partition

The thin record is stored verbatim in one row of its kind's table
(`portfolios` / `projects` / `products` / `programs`):

| Column | Type | Purpose |
|---|---|---|
| `id` | serial PK | Internal row id |
| `pid` | UUID unique | Public id (route param) |
| `name` | string | Denormalised from the payload for cheap listing |
| `data` | JSONB | The full thin `WorkItem` payload (incl. `kind`, `goals[]`) |
| `portfolio_pid` | UUID null | Denormalised `data.portfolio_ref` — **child kinds only**; for cheap roll-up of a portfolio's children |
| `active` | boolean (default true) | Registry flag |
| `deleted_at` | timestamptz null | Soft delete |

`Model::to_work_item()` deserialises `data` into the matcher type;
`Model::create()` / `update_data()` serialise it in. The `name` column
MUST equal `data.name`, and (for child kinds) the `portfolio_pid`
column MUST equal `data.portfolio_ref` (the model layer writes them
together).

Because the matcher's `WorkItem` **is** the persisted payload and the
matching input (no adapter), every scored field — including `kind`,
`goals[]`, `relationships[]`, `keywords`, `tags`, `portfolio_ref` —
round-trips verbatim through `data` and reaches the matcher unchanged.
There is **no lossy-drop list**; the only fields outside the JSONB
payload are the registry-plumbing columns (`id`, `pid`, `active`,
`deleted_at`) and the two denormalised projections (`name`,
`portfolio_pid`).

**The partition.** The operational sub-resources — **tasks, issues**
— are **not** in `data` and **never** enter the matcher. They live in
their own tables keyed by `(parent_kind, parent_pid)` (§10.1), because
they are high-volume (a work item may have thousands of tasks) and are
not identity-bearing. `goals[]` is the sole crossover: it is in the
payload **and** exposed as a sub-resource, with goal writes mutating
`data.goals[]` (§10.2) so the two views stay consistent.

### 5.7 Front-end TypeScript types

The front-end mirrors the wire shape in `src/lib/api/types.ts`
(`WorkItem`, `WorkItemKind`, `Goal`, `WorkItemStatus`, `GoalStatus`,
`IdentifierScheme`, `WorkItemIdentifier`, `WorkItemRelationship`,
`RelationKind`, `WorkItemRef`, `ScoredRef`, plus the sub-resource types
`Task`, `Issue` and the derived `Timeline` / `Burndown` shapes). The
matcher type is upstream for the thin record: if a `WorkItem` field
changes in the matcher crate, the service inherits it automatically
(re-serialisation) and the front-end types MUST be fixed in the same
change cycle. The sub-resource types are owned by the service crate
spec (they have no matcher counterpart).

### 5.8 Shared invariants

All subprojects MUST uphold:

- `kind` is present on every `WorkItem` and is one of the four closed
  variants; the record lives in the matching collection / table.
- Two work items of different `kind` **never** match (the kind gate,
  §5.5 / §6.3) — end to end, in the matcher and in every service
  endpoint (you never compare across collections).
- `name` is non-empty; the stored `name` column equals `data.name`.
- For child kinds, when `portfolio_ref` is set the denormalised
  `portfolio_pid` column equals it; the `Portfolio` kind carries no
  `portfolio_ref`.
- The JSONB payload round-trips losslessly:
  `serde_json::from_value(to_value(w)) == w`.
- Owner-scoped codes (`code`, `Code`, `LocalId`) are never treated as
  globally unique — no cross-owner short-circuit, end to end. They
  short-circuit (R-1, §6.6) **only** within an equal, non-empty
  `owner_org_id`.
- `EntityRef` fields hold a valid `<entity_type>:<id>` URN or are
  absent; the service does not call the target service on the write
  path (optimistic, per
  [`agents/share/cross-service-linking.md` §5](../../agents/share/cross-service-linking.md)).
- A `WorkItemRelationship` references an **existing** `WorkItem`; **no
  work item relates to itself**. `ParentOf`/`ChildOf`,
  `DependsOn`/`BlockedBy`, and `Supersedes`/`SupersededBy` stay
  **acyclic** (no work item is its own ancestor / dependency /
  predecessor, directly or transitively) and **inverse-consistent**
  (A `ParentOf` B ⇔ B `ChildOf` A; likewise the other two pairs);
  `SimilarTo` and `RelatedTo` are **symmetric**.
- Each `tags` entry is short, trimmed, and non-empty; the list is
  de-duplicated case-insensitively and defaults to empty.
- Operational sub-resources (tasks, issues) are **never** serialised
  into `data` and **never** reach the matcher; `goals[]` is the only
  payload-and-sub-resource field, and goal writes mutate
  `data.goals[]`.
- A sub-resource always belongs to exactly one live (non-soft-deleted)
  work item; soft-deleting a work item hides its sub-resources from
  read paths.
- Cross-service links (`entity_links`) are never stored in
  `relationships` and never fed to any matcher
  ([cross-service-linking.md §7](../../agents/share/cross-service-linking.md)).
- Match scores are in `[0.00, 1.00]` and always travel with a
  per-component breakdown and `Confidence` band.
- Soft delete (`deleted_at`) is the only delete: the service never
  row-deletes, and the front-end never offers hard delete.
