## 24. Change Control

**24.1 Authority.** This file is the specification. Any behaviour-affecting change MUST update this file in the same PR. Section numbering is stable. `CHANGELOG.md` records what changed; this spec records what is.

**24.2 SDD workflow.** All canonical artefacts (spec / plan / tasks) live here — no separate `plan.md` or `tasks.md`. Full discipline in [`AGENTS/spec-driven-development.md`](../AGENTS/spec-driven-development.md). Sections cluster as: spec §1–§7; plan §8–§20; forward look §21–§22; tasks §23 (live) + [`AGENTS/delivered-tasks.md`](../AGENTS/delivered-tasks.md) + [`AGENTS/delivered-tasks-2.md`](../AGENTS/delivered-tasks-2.md); provenance in `CHANGELOG.md`.

**24.3 Lifecycle.** Identify section → update spec (RFC 2119) → update / add tests → implement in `src/` → record in `CHANGELOG.md` → open PR referencing the section(s).

**24.4 Disagreements.** Spec wins over code (file a §23 task to align). More-specific section wins over less-specific. Design disagreements → §22, not unilateral action.

