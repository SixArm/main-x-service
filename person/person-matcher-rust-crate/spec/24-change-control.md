## 24. Change Control

**Authority.** This file is **the** specification. Behavioural changes MUST update this file in the same PR as the code. Spec-only PRs are acceptable for documenting existing behaviour or recording a decision; editorial fixes (typos, formatting) MAY be batched. Section numbering is stable — prefer appending. `CHANGELOG.md` records *what changed*; this spec records *what is*.

**SDD workflow.** SDD artefacts live in this one document: **Specification** → §1 / §2 / §3 / §6 / §7; **Plan** → §8–§20; **Forward look** → §21 / §22; **Tasks** → §23 plus `AGENTS/delivered-tasks.md` / `AGENTS/delivered-tasks-detail.md`; **Provenance** → `CHANGELOG.md`. No separate `plan.md` / `tasks.md`.

**Lifecycle of a change.** (1) Identify affected sections; if the spec is silent, draft an addition first. (2) Update the spec with normative text (RFC 2119 MUST / SHOULD / MAY). (3) Update or add tests. (4) Implement in `src/`. (5) Record in `CHANGELOG.md` under "Unreleased". (6) Open a PR referencing the affected sections.

**Resolving disagreements.** If the spec disagrees with the code, the spec wins (file a §23 task; never silently rewrite the spec). If two sections disagree, the more specific wins (file an editorial fix). If a contributor disagrees with a design point, propose a change to §22 rather than acting unilaterally.

---

