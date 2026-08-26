# PRINCE2

**PRINCE2** (PRojects IN Controlled Environments) is a structured,
process-based project management method that separates **governance**
from **delivery**. Developed by the UK government, now owned by
[PeopleCert](https://www.peoplecert.org/), and one of the most widely
held project management certifications in the world — which matters
here because a UK public-sector portfolio is very likely to be run this
way, and to expect its tooling to speak the vocabulary.

The current edition is **PRINCE2 7** (2023). Its predecessor's *themes*
were renamed **practices**, and it added digital-and-data and
**sustainability** management approaches.

This file is a **map, not a transcription**: what PRINCE2 asks for, what
this entity already provides, and what it would have to build. The
pattern follows [total-project-control](../total-project-control/index.md)
and [flow-framework-metrics](../flow-framework-metrics/index.md).

## The five integrated elements

People · Principles · Practices · Processes · Project context. The last
one is the tailoring dimension, and it is the reason a rigid
implementation would be *wrong*: "tailor to suit the project" is itself
a principle.

## 1. The seven principles

A project is only PRINCE2 if it follows all seven.

| Principle | Where this entity already stands |
|---|---|
| **Continued business justification** | `benefits`, `budget_lines`, business-case targets (§5.9.6), and DIPP (§5.9.7) — *is the value still to come worth the money still to spend?* is the same question, arithmetically |
| **Learn from experience** | Retrospective ceremonies with categorised notes convertible into tasks (§5.9.3). **Gap:** no cross-project lessons log — lessons die with the plan |
| **Defined roles and responsibilities** | ABAC attributes and `EntityRef` leads / assignees / owners. **Gap:** no PRINCE2 role model (see §Organizing below) |
| **Manage by stages** | The gate stage vocabulary `g0_concept` … `g5_benefits` with gate reviews. **This is the closest existing fit, and the trap — see §4** |
| **Manage by exception** | **The controls register (FR-38) is this principle, built.** See §5 |
| **Focus on products** | **Weakest area.** This entity models *tasks* (activities), not *products* with quality criteria. See §Quality |
| **Tailor to suit the project** | Custom workflows (FR-26), configurable weights, declared intended mix, deployment-tunable policy |

## 2. The seven practices

| Practice | Status here |
|---|---|
| **Business case** | Substantially present: benefits, budget lines, ROI, Smart Score, TPC |
| **Organizing** | Partial: refs and ABAC, but no four-tier structure |
| **Plans** | Present: the recursive `Plan` tree, timeline, milestones, sprints |
| **Quality** | **Absent** — no product descriptions, no quality criteria, no quality register |
| **Risk** | Present: `risks` with probability × impact, categories, escalation to issues |
| **Issues** | **Specified and unbuilt** (FR-14 — §14.2). PRINCE2 splits issues into *request for change*, *off-specification*, and *problem/concern*, which is a finer vocabulary than the planned `kind` |
| **Progress** | Present and strong: TBA, burndown, flow metrics, lifecycle funnel, control coverage |

## 3. The seven processes

| Process | Nearest existing surface |
|---|---|
| Starting up a project | `ideas` → `proposals` intake pipeline |
| Directing a project | Gate reviews (`gate_reviews`, board-level approve / hold / reject) |
| Initiating a project | Proposal → plan promotion; business-case targets |
| Controlling a stage | The board, WIP limits, automations, the controls register |
| Managing product delivery | Tasks and their transition log — *activity* delivery, not *product* delivery |
| Managing a stage boundary | Gate review + readiness checklist (FR-16f) |
| Closing a project | Plan status `Completed`, phase `closing`, benefits review at `g5_benefits` |

## 4. The vocabulary trap — read this before implementing

This entity already carries **three** ordered vocabularies, and §1.5.1
says they are deliberately uncoupled: the lifecycle funnel
(`idea` … `closed`), the gate stage (`g0` … `g5`), and the project phase
(`initiating` … `closing`).

**PRINCE2 stages are a fourth, and they are not any of the three.**

- A PRINCE2 **management stage** is a *funding and authorisation
  increment* — the unit the board authorises, with its own stage plan
  and its own tolerances. A project has as many as it needs, named by
  the project, and they are **not a fixed enumeration**.
- The **gate stage** is a fixed ladder of governance decisions.
- The **project phase** is a fixed process-group label.

A PRINCE2 stage boundary usually *coincides* with a gate review, which
makes them look like the same thing. They are not: a project can have
four management stages between `g2` and `g3`, and PRINCE2 requires at
least two stages (initiation plus one) regardless of how many gates a
portfolio office defines.

**Do not model PRINCE2 stages by reusing `stage` or `phase`.** They need
their own record — a per-plan, operator-named sequence with a plan and
tolerances attached. Reusing an existing column would silently redefine
what the other three mean, which is exactly what §1.5.1 exists to
prevent.

## 5. Management by exception — already built, under another name

This is the mapping worth knowing, because it means the hardest PRINCE2
practice is largely done.

**Tolerances are delegated authority**: a higher level sets bounds, a
lower level works freely inside them and escalates only when a bound is
*forecast* to be breached. PRINCE2 7 defines **seven** tolerance types —
time, cost, scope, quality, risk, benefits, and **sustainability**.

That is precisely the shape of the **controls register** (§5.9.8 /
FR-38): a standard, a measurement, a comparison, and an action, with the
**timing** deciding what a breach may do. The correspondence:

| PRINCE2 | Controls register |
|---|---|
| Tolerance | `ControlStandard` — metric, target, comparator, tolerance band |
| Forecast breach | A `ControlReading` with verdict `fail` |
| Exception report | A failing reading, and the `ControlAction` it provokes |
| Escalation to the board | `ControlAction` of kind `escalate` |
| Highlight report | The coverage view (FR-39) |
| Concession / accepting a deviation | `ControlAction` of kind `accept`, which clears the *unanswered* count |

Two differences that would need closing:

- **PRINCE2 escalates on a *forecast* breach, not an observed one.** A
  reading records what *is*; a tolerance asks what *will be*. The
  forecasting machinery exists (`GET …/forecast`, Monte-Carlo over
  throughput), so the gap is wiring, not invention.
- **Tolerances are set per stage and inherited downward.** Controls are
  currently per plan with no inheritance.

**Sustainability tolerance has no home at all** — no sustainability
metric exists anywhere in this entity, and adding one is a data
question, not a reporting one.

## 6. Management products → records

| PRINCE2 product | Here |
|---|---|
| Business case | `benefits` + `budget_lines` + `business_case_targets` |
| Project brief / PID | The `Plan` payload plus its goals |
| Stage plan | **Absent** (see §4) |
| Work package | `tasks`, loosely — a work package is authorised and has tolerances; a task has neither |
| Risk register | `risks` ✅ |
| Issue register | **Absent** (FR-14 unbuilt) |
| Quality register / product descriptions | **Absent** |
| Highlight report | Insights + coverage views |
| Exception report | A failing control reading |
| Lessons log | Sprint retrospective notes, per-plan only |
| Benefits management approach | `benefits` + value realization (§6.4c) |
| Daily log / checkpoint report | Standup digest |

## 7. What adopting PRINCE2 would actually require

In dependency order, honestly sized:

1. **Management stages** — a per-plan named sequence with its own plan
   and tolerances (§4). The prerequisite for everything else.
2. **Tolerances per stage**, inherited, with forecast-based breach
   detection wired to the existing forecaster (§5).
3. **The issue register** (FR-14), with the PRINCE2 three-way split.
4. **Products with quality criteria** — the largest gap and a genuine
   model change: this entity tracks activities, and PRINCE2 is
   product-focused by principle. A `Product` sub-resource with
   descriptions, quality criteria, and a quality register.
5. **A cross-project lessons log**, so "learn from experience" survives
   the plan it was learned in.
6. **The four-tier role model** (corporate / board / manager / team),
   expressible as ABAC attributes plus a per-plan role assignment.

## 8. Out of scope, deliberately

- **Certification content.** Foundation / Practitioner / PRINCE2 Agile
  are training products; this is a registry and a PM suite. Where a
  certification pathway matters to a person's record, that is the
  [course](../../../course/) and
  [worker](../../../worker/) entities' business, not this one.
- **Claiming PRINCE2 conformance.** Supporting the vocabulary is not
  being certified against the method, and the distinction is the same
  one [conformance levels](../14-implementation-status.md) draws
  everywhere else: a level states what has been verified, not what has
  been written.
- **Prescribing PRINCE2.** Tailoring is a principle, and a portfolio
  running Scrum, SAFe or nothing at all is a first-class user of this
  entity. PRINCE2 support is a *vocabulary a deployment may adopt*,
  never a shape imposed on every plan.

## References

- PeopleCert — <https://www.peoplecert.org/>
- PRINCE2 official — <https://www.prince2.com/uk/what-is-prince2>
- PRINCE2 wiki — <https://prince2.wiki>
- Wikipedia — <https://en.wikipedia.org/wiki/PRINCE2>
- Knowledge Train overview — <https://www.knowledgetrain.co.uk/project-management/prince2>
