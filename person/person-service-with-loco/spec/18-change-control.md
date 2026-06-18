## 18. Change Control

Material changes to this spec — domain-model fields, match-quality
thresholds, API-surface shape, compliance scope — MUST land in the
same commit as the corresponding code change. This per-crate spec is
local to the Person Service.

Bullet what changed, not how: every spec edit should be a diff a
reviewer can read in isolation. Avoid re-flowing surrounding paragraphs
in the same PR as a content change — keep stylistic churn out of
behavioural diffs.

### Plan history

- **2026-03-22 — Compile / test / docs remediation (completed).**
  Resolved ~60 compilation errors (missing `.await` on the async
  `PersonRepository` / audit-log calls in the REST + FHIR handlers, a
  `crate::db::models::audit_log::Model` type-reference fix, an
  `i64`→`u64` audit-limit cast, and a missing `sea_orm::sea_query::Expr`
  import); then expanded unit / integration / benchmark coverage and
  filled out the `AGENTS/` reference docs. Outcome is reflected in the
  current §11 (testing) and §14 (status); folded here from a former
  `docs/superpowers/plans/` implementation plan, now removed.

