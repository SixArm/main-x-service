## 1. Purpose and Vision

### 1.1 Purpose

The **portfolio entity** is the portfolio / project / product /
program registry of the Main X Index — a federated identity index
serving a worldwide public governmental system with millions of users.
It models the unit of work an organisation funds, staffs, and reports
on as a single recursive record type — a **plan** — carrying an
**optional** descriptive `kind` label:

- **Portfolio**, **Project**, **Product**, **Program**, **Practice**,
  **Process**, **Purpose**, **Pathway**, **Proposal** — the values of
  an **optional** `kind` label used for description, display, and
  grouping. A plan need not carry one; `kind` is not a discriminator
  and does not fix a collection.
- **Containment is general and recursive**: any plan may contain any
  other plan via `parent_ref` (a recursive tree — a portfolio-labelled
  plan may parent project-labelled plans, but any parent/child labelling
  is allowed).

Every plan lives in **one service table and one REST collection**
(`/api/plans`), and matching is **kind-agnostic**. The entity is
delivered as a trio of subprojects that compose into one capability:

| Subproject | Role |
|---|---|
| [project-portfolio-management-service-with-loco](../project-portfolio-management-service-with-loco/) | Registry service **and** project-management tool — loco.rs CRUD + matching over REST on one recursive `plans` collection; PostgreSQL persistence; operational sub-resources (goals, tasks, issues, sprints, phases, effort, automations) and derived views (timeline / burndown / flow metrics) |
| [project-portfolio-management-matcher-rust-crate](../project-portfolio-management-matcher-rust-crate/) | Canonical pairwise matching library — deterministic + probabilistic, kind-agnostic, embedded by the service |
| [project-portfolio-management-front-end-with-svelte](../project-portfolio-management-front-end-with-svelte/) | Operator UI — SvelteKit SPA over the service's REST API |

The entity has **two faces that share one record**:

- **A matchable identity registry.** It gives an organisation one
  canonical record per plan — deduplication: "is
  this migration project the same initiative the other department
  already chartered?" The **thin** `Plan` record (the matcher
  type) is deduplicated and matchable on the attributes that identify a
  plan (name, goal titles, owner-scoped code, sponsoring
  organisation, parent plan, timeframe, keywords, identifiers).
  Matching is **kind-agnostic** (§5.5): any two plans may match
  regardless of their optional `kind` label.
- **A full project-management suite.** A `Plan` also *owns* the
  operational record — goals, tasks, issues, sprints, workflow states,
  effort, automations, and the **project phase** it is being managed
  through (§1.5) — plus the derived views over that record (timeline /
  Gantt, burndown, flow analytics). The ambition is **completeness, not
  a charter-level subset**: a team should be able to run the work here
  rather than mirror it from a tool that does (§1.4). This high-volume
  operational data lives in **separate service tables** and is
  **deliberately excluded** from the matcher payload (§5.6); only the
  thin identity record is matched.

### 1.2 Vision

One canonical record per real-world plan, organised into recursive
containment trees, usable both for dedup and as the live project
workspace:

- **Registry of plan identities.** The entity records *which*
  plans exist and how they are
  identified — not (for matching purposes) the day-to-day task churn.
  Identifiers (external PM-tool ids such as Jira project keys, Asana
  GIDs, Trello board ids, plus URIs / UUIDs) make it the linkage hub
  between PM tools, sponsoring organisations, and the people who lead
  and staff the work.
- **A workspace, not a shadow of one.** The operational surface is a
  first-class deliverable in its own right, not a stub that defers to a
  "real" PM tool: configurable workflows, automation, effort tracking,
  sprint ceremonies (§1.4), the sequential project phases a plan is
  managed through (§1.5), and the Flow Framework metrics that measure
  the result (§1.6). A team adopting this entity should not need a
  second tool to do the work; where one is already in use, the
  external-id identifiers make it a peer to migrate from or coexist
  with, not a master to copy from.
- **Explainable matching.** Every match decision returns a
  per-component score breakdown (name, goals, code, owner org, parent
  plan, timeframe, keywords, relationships, tags) that an auditor
  can inspect — no black boxes.
- **Federated by reference.** A plan's lead, assignees, and
  members are **`EntityRef`s** into the person / worker / authentication
  entities; its sponsoring organisation is an `EntityRef` into the
  organization entity; its parent plan is a `parent_ref`; and
  any goal / task / issue can carry a cross-service link to **any**
  index entity
  ([`agents/share/cross-service-linking.md`](../../agents/share/cross-service-linking.md)).
- **Multinational by design.** `in_language` on every record;
  operator surfaces localize to the locales in
  [`agents/share/locales.md`](../../agents/share/locales.md)
  (roadmap, §15).
- **Audit-grade.** Soft delete, full audit logging, and event
  streaming from the family baseline, suitable for
  government-portfolio information governance.

### 1.3 Non-goals

> **Reversed 2026-08-25.** "Not a replacement for full-feature PM
> suites" was a non-goal here until this revision. It is now an
> **explicit goal** (§1.4): the deep features that sentence delegated to
> Jira / Asana / MS Project / Linear — custom workflows, automation
> rules, time tracking, sprint ceremonies — are capabilities this entity
> owns. The external-id identifiers (§5.5) remain, but their purpose
> changes: they are for **migration and coexistence**, not for
> delegating the hard parts. The bullets below are what remains out of
> scope.

- **Not** a finance / budgeting system — no cost ledgers, no
  invoicing. Budget figures, where modelled at all, are descriptive,
  not transactional.
- **Not** a collaboration / discussion store — posts, comments, and
  membership management are **out of scope** (deferred, §15). Sprint
  notes (§1.4.4) and review verdicts (FR-16a) are structured records
  against a ceremony or a decision, not a general discussion thread.
- **Not** an authentication / authorisation provider. Sign-on for the
  whole index is the [authentication entity](../../authentication/)
  (passwordless magic-link, cookie session + PASETO v4 public token); this
  entity is a token *verifier* and references user identities by `EntityRef`.
  Auth source of truth (supersedes the RS256-JWT model):
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md).
- **Cross-service links are never a match signal** — a plan that
  *links to* a person or org is not thereby the *same* as another plan
  ([`agents/share/cross-service-linking.md` §7](../../agents/share/cross-service-linking.md)).

### 1.4 Deep project-management capabilities

The four capabilities §1.3 once delegated to the source tools, stated
as commitments. This section fixes **what each one is and the rule it
must not break**; the field-level model belongs in §5, the endpoints in
§6 / §9, the tables in §10, and the work in §13.

Two of the four are already partly built, and saying so keeps this
section from reading as greenfield ambition:

| Capability | Today | Gap |
|---|---|---|
| **Custom workflows** | A fixed status vocabulary — task `Todo\|InProgress\|InReview\|Done\|Blocked`, issue `Open\|InProgress\|Resolved\|Closed` — with WIP limits per column | Everything: the vocabulary is a compile-time constant |
| **Automation rules** | Delivered (FR-16c): board-crossing and review triggers, one action per rule, validated at write time, non-cascading, every firing logged | Trigger breadth and multi-action rules |
| **Time tracking** | Nothing. `estimate` / `remaining` on a task are *forecasts*, never recorded effort | The whole capability |
| **Sprint ceremonies** | `sprints` + `sprint_notes` tables; retrospective note categories (`went_well` / `improve` / `action` / `feedback`), convertible into tasks | Planning and review as first-class ceremonies with their own record |

#### 1.4.1 Custom workflows

A deployment — and, where it needs to differ, a single plan — may
declare its **own** task and issue states and the transitions permitted
between them, instead of accepting the built-in vocabulary. A workflow
is configuration, versioned and audited like any other write.

**The rule this must not break: every custom state maps to exactly one
canonical category.** The board, the burndown, the timeline, and every
time-based-analysis figure are computed from what a state *means* —
whether an item is waiting, being worked on, or finished. A free-text
state vocabulary with no mapping would silently break all of them: an
item in a state nobody classified is an item the flow-efficiency
denominator cannot account for. So a workflow declares, per state, one
of `todo` / `active` / `waiting` / `done`, and a state without a
category is **refused at write time** rather than defaulted — the same
posture time-based analysis already takes for an unknown status
(`time-based-analysis.md` §5.1: "unknown statuses are refused, never
coerced").

The built-in vocabulary becomes the default workflow rather than the
only one, so an existing plan keeps working with nothing configured.

#### 1.4.2 Automation rules

Deepened, not introduced. The delivered engine's three invariants are
load-bearing and survive every extension:

- **A failing rule never undoes the operator's action.** The move
  succeeds; the rule is recorded as a `failed` run.
- **Actions are applied without re-entering the engine**, so automations
  cannot cascade into each other.
- **Every firing is logged** — applied, skipped, or failed — because an
  automation nobody can see is indistinguishable from a bug.

The deepening is breadth within those bounds: additional triggers
(a field change, a **phase transition** (§1.5), a date arriving, an SLE
breach) and more than one action per rule, applied in declared order
with each action's outcome logged separately.

#### 1.4.3 Time tracking

Recorded effort against a work item: who (an `EntityRef`), against
which task, how long, on what date, optionally against a billable or
capitalisable category so the existing capex / opex budget lines have a
real denominator. Roll-ups per task, per plan, and per assignee.

**The rule this must not break: effort is a property of the work, not a
score for the worker.** The family has a stated refusal to compute
per-person cycle time, throughput, or flow efficiency
(`time-based-analysis.md` §12.4), and time tracking is the single
easiest way to reintroduce exactly that by the back door. So:

- Per-assignee roll-ups exist for **capacity, cost, and "who should be
  asked about this"**, and — by the owner decision of 2026-08-25 —
  **per-person utilisation is computed** (FR-35), under the five
  obligations in
  [time-based-analysis.md](time-based-analysis.md) §12.4a: declared
  denominator, non-working time excluded, small denominators
  suppressed, never the sole ranking key, effort labelled asserted.
- **What that reversal does not reach:** per-assignee **cycle time,
  throughput and flow efficiency** stay refused (§12.4), and no
  per-person figure is ever an input to Smart Score (FR-16e) or to any
  Flow Framework metric (§1.6).
- Recorded effort is **never** substituted for elapsed time. Flow
  efficiency stays work-time-over-*calendar*-time; computing it from
  timesheet hours over timesheet hours would report 100% and mean
  nothing (`time-based-analysis.md` §6.3).
- Effort is **entered**, not inferred from status transitions. The
  transition log is a by-product of the work and stays trustworthy
  because nobody edits it; a timesheet is an assertion and is labelled
  as one.

#### 1.4.4 Sprint ceremonies

The four ceremonies as first-class records against the existing
`sprints` table, rather than only the retrospective that exists today:

- **Planning** — the committed set at sprint start, captured as a
  commitment snapshot so a later scope change is visible as a change
  rather than as a moved goalpost.
- **Daily** — optional; blockers raised become issues (FR-14) rather
  than a second parallel store.
- **Review** — what was accepted, and by whom.
- **Retrospective** — delivered: categorised notes, with `action` and
  `feedback` notes convertible into tasks so an improvement has an
  owner.

**The rule this must not break: sprint metrics are not the flow
metrics.** Burndown and sprint velocity are sprint-scoped and
count-based; the Flow Framework metrics (§1.6) are item-scoped and
time-based. Both are kept, both stay honest — no ideal line, no
interpolation — and neither is derived from the other.

### 1.5 Project phases — the sequential management lifecycle

A plan is managed through five **ordered** phases:

| # | Phase | The question it answers |
|---|---|---|
| 1 | **Initiating** | Should this exist? Who sponsors it, what is it for, who leads it? |
| 2 | **Planning** | What is the work, in what order, at what cost, with what risks? |
| 3 | **Executing** | Doing it — the phase the board, sprints, and effort belong to |
| 4 | **Controlling** | Is it going as planned, and what is being changed in response? |
| 5 | **Closing** | Formal completion — acceptance, handover, lessons, release of the team |

`phase` is a field on the plan; each change writes a transition row
(from, to, when, by whom, why) so the *duration* of each phase is
measurable rather than merely its current value. Rules:

- **Advancement is one step at a time.** A skip is refused `422`. If a
  plan really is running before it was planned, that is a fact worth
  recording as such, not one to hide by permitting a jump.
- **Moving backwards is allowed and explicit.** Re-planning is normal;
  a plan that returns from Executing to Planning records the return
  with a reason. Only a silent backward move is refused.
- **Every phase reports even at zero**, as the existing funnel already
  does — a phase with nothing in it is a finding, not a row to omit.
- **Phase does not gate operations.** Tasks may be created in
  Initiating and issues raised in Closing. The phase describes where
  the *management* of the plan has got to; refusing operational writes
  on that basis would just teach operators to lie about the phase.

**A recorded concern, delivered as asked.** In PMBOK, Monitoring &
Controlling is a process group that runs *concurrently with* the
others rather than following Executing — it is what you do throughout,
not a stage you reach. This spec models the five phases as strictly
sequential because that is the requested shape, and it reads well for
governmental portfolio reporting where a plan does sit in a reporting
stage. The cost is that "Controlling" as phase 4 means "the delivery is
complete enough that the work is now variance management", not "control
begins here". Should the concurrent reading be wanted later, the clean
change is a boolean overlay rather than a sixth phase.

#### 1.5.1 Three ordered vocabularies, deliberately uncoupled

This entity now carries **three** ordered sequences, and conflating
them is the obvious future defect:

| Vocabulary | Values | Question | Owner |
|---|---|---|---|
| **Lifecycle funnel** | `idea` → `proposal` → `in_delivery` → `gated_complete` → `benefits` → `closed` | Where does this *item of demand* sit, portfolio-wide? | FR-16f |
| **Gate stage** | `g0_concept` … `g5_benefits` | What was the last *approved governance decision*? | Governance (§6) |
| **Project phase** | `initiating` … `closing` | Where has the *management of this plan* got to? | §1.5 |

They are three axes, not three names for one axis: an item can be in
the `in_delivery` funnel phase, hold gate `g2_definition`, and be in
project phase `planning` all at once, and that combination is
informative rather than contradictory. **No cross-vocabulary constraint
is enforced**, and that is deliberate — a rule such as "you may not
enter Executing before g3" sounds prudent and would, in practice, make
a true state unrecordable whenever governance lags delivery, which is
the case most worth being able to see. Divergence is surfaced as a
**readiness check** (FR-16f), where it is a finding an operator reads,
not a write the service refuses.

### 1.6 Flow Framework metrics

The [Flow Framework](flow-framework-metrics/index.md) measures the flow
of business value through a product value stream, in five metrics.
Four of the five this entity already computes under time-based-analysis
vocabulary; naming the correspondence is the point of this section, so
the same number is not built twice under two names:

| Flow metric | Definition | Here |
|---|---|---|
| **Flow Time** | How quickly work moves from start to finish | **Delivered** — cycle time and lead time, as distributions with percentiles (`time-based-analysis.md` §6.1, §7.1) |
| **Flow Velocity** | How much work gets completed over time | **Delivered** — throughput (μ), items finished per unit time (TBA §9) |
| **Flow Efficiency** | Active work as a percentage of elapsed time | **Delivered** — flow efficiency over *calendar* time, the headline 5–15% ratio (TBA §6.3, §7.2) |
| **Flow Load** | Work in progress at any moment | **Delivered** — WIP (κ), reported against the configured WIP limits (TBA §9, §9.3), plus aging WIP (§8) |
| **Flow Distribution** | The mix of work types being prioritised | **New** — see below |

**Flow Distribution is the genuinely missing one**, and it is the one
that makes the other four legible: a rising Flow Velocity means
something entirely different when the mix has shifted to defects. The
Flow Framework's four work-item types map onto records this entity
already holds — **feature** (a task advancing a goal), **defect** (an
issue of kind `Bug`), **risk** (an issue of kind `Risk`, and the risk
register's `compliance` / `security` categories), **debt** (the
technical-debt register, which already rides the risk lifecycle under
category `tech_debt`). Distribution reports the share of each type over
a window, per plan and rolled up across a portfolio tree.

Three rules carry over unchanged, because they are what makes these
figures worth having:

- **Not a person metric.** Flow metrics describe the value stream, not
  the people in it (`time-based-analysis.md` §12.4). There is no
  per-assignee Flow Time, Velocity, Efficiency or Load, and time
  tracking (§1.4.3) does not create one. **This survives the 2026-08-25
  utilisation decision intact** (§12.4a, FR-35): utilisation is a
  capacity measure, not a flow metric, and no per-person figure feeds
  any of the five.
- **Velocity here means items finished, not story points.** TBA
  deliberately declines velocity-as-a-target — it is trivially inflated
  and it is not a time (`time-based-analysis.md` §2.4). Adopting the
  Flow Framework's *Flow Velocity* does not reopen that: it is a
  count-per-period of completed items, reported **alongside** Flow Time
  and Flow Distribution, never as a score on its own.
- **Distribution is reported, never targeted by default.** A deployment
  may declare an intended mix (e.g. "20% debt"), and the gap is then
  shown against it. Absent a declared intent, the service reports the
  mix and says nothing about whether it is right — an unlabelled target
  is how a measurement becomes a quota.
