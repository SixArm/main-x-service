## 17. References

The portfolio entity's standards and tooling landscape, plus the
family's standing references. The registry-of-identities-with-a-PM-tool
posture relative to this landscape is §8.7.

### 17.1 Standards and vocabularies

- [schema.org/Project](https://schema.org/Project) — nearest
  schema.org type for a project / plan;
  [schema.org/Product](https://schema.org/Product) for the product
  label; `same_as` follows schema.org
  [`sameAs`](https://schema.org/sameAs) semantics.
- [Dublin Core](https://www.dublincore.org/) — `title` / `subject` /
  `identifier` term semantics that `name` / `keywords` / `identifiers`
  echo.
- ISO 639-1 language codes ([`agents/share/locales.md`](../../agents/share/locales.md))
  — the `in_language` value set.
- Date / time: ISO 8601 calendar dates for `start_date` /
  `target_date` / goal `target_date` / task `due_date`.

### 17.2 Project-management tools (interop targets)

The deterministic identifier schemes (§5.2, R-0) are the external ids
these tools expose, so a project synced from a source tool deduplicates
against its registry twin in the `plans` collection (§8.7):

- [Jira](https://www.atlassian.com/software/jira) — `JiraProjectKey`
  (e.g. `MIG`); REST + webhook surface for sync.
- [Asana](https://asana.com/) — `AsanaGid` (the global id on every
  Asana object).
- [Trello](https://trello.com/) — `TrelloBoardId`.
- [Microsoft Project](https://www.microsoft.com/microsoft-365/project/project-management-software)
  — `MsProjectId`.
- [GitHub Projects](https://docs.github.com/issues/planning-and-tracking-with-projects)
  — `GitHubProjectId`.
- [Linear](https://linear.app/) — `LinearId`.

### 17.3 Methodology landscape (informative)

Planning concepts the sub-resources implement — out of matching scope,
useful when judging whether two records describe the same plan:

- **Goals / OKRs** — **a full OKR engine** (§5.9.2). `goals[]` in the
  payload is the *objective*, the identity-bearing half whose titles
  feed matching; the measurable half — key results with a metric,
  baseline, target and direction, plus dated check-ins with an optional
  confidence — lives in its own tables and never reaches the matcher.
  Progress, objective score and plan score are **derived on read**, so
  they cannot drift from the evidence. Alignment rolls up through the
  existing `parent_ref` containment tree rather than a second OKR
  hierarchy that could disagree with it.
- **Work breakdown** — tasks under goals, tasks under tasks
  (`parent_task_id`); a shallow hierarchy, not a full WBS tool.
- **Issue / risk tracking** — `Issue { kind, severity, status }`
  covers bug / risk / blocker / question / improvement, and feeds the
  Flow Distribution work-type classification (§5.9.5).
- **Portfolio / programme management** — recursive containment via
  `parent_ref` (any plan may contain any other plan) models the
  portfolio → programme → project hierarchy, with the optional `kind`
  label (`Portfolio` / `Program` / `Project` / `Product`) describing
  each level; the `ParentOf` / `ChildOf` relationships model the same
  hierarchy among plans, and `DependsOn` / `BlockedBy` model
  cross-initiative dependencies.
- **Burndown / Gantt** — the two derived views (§6.4); standard
  progress projections, computed not stored.

### 17.4 Family references

- Subproject specs (to be scaffolded, T-1):
  [service](../project-portfolio-management-service-with-loco/spec/index.md),
  [matcher](../project-portfolio-management-matcher-rust-crate/spec/index.md),
  [front-end](../project-portfolio-management-front-end-with-svelte/spec/index.md).
- Entity AGENTS reference set: [`agents/index.md`](../agents/index.md).
- Adopted family designs:
  [cross-service-linking.md](../../agents/share/cross-service-linking.md),
  [bulk-import-export.md](../../agents/share/bulk-import-export.md),
  [event-bus.md](../../agents/share/event-bus.md),
  [match-search-merge.md](../../agents/share/match-search-merge.md),
  [auditability.md](../../agents/share/auditability.md),
  [privacy.md](../../agents/share/privacy.md).
- Shared docs index: [`agents/share/index.md`](../../agents/share/index.md);
  sibling entity-level spec exemplar:
  [care-pathway/spec](../../care-pathway/spec/index.md).
- loco.rs: [loco.rs](https://loco.rs/) — the service framework.
