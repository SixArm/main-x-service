## 18. Change Control

Material changes to this spec — domain-model fields, match-quality
thresholds, API-surface shape, compliance scope — MUST land in the
same commit as the corresponding code change. This per-crate spec is local to the
Worker Service.

Bullet what changed, not how: every spec edit should be a diff a
reviewer can read in isolation.

### Plan history

- **2026-03-22 — Compile / test / docs remediation (completed).**
  Resolved the outstanding compilation errors (missing `.await` on
  async repository / handler calls, type-reference and import fixes),
  then expanded unit / integration / benchmark coverage and filled out
  the `agents/` reference docs. Outcome is reflected in the current §11
  (testing) and §14 (status); folded here from a former
  `docs/superpowers/plans/` implementation plan, now removed.

