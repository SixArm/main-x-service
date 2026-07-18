## 16. Open Questions

Open questions resolve into §13 tasks or §5–§12 amendments when
decisions are made. Matcher-internal questions migrate to the matcher
spec §16 once that crate is scaffolded.

- **OQ-1 — Validation status code.** Confirm `422` (not `400`) for a
  blank `name`, a `kind`/collection mismatch, a malformed `EntityRef`
  / `portfolio_ref`, a malformed deterministic identifier, a
  self-referencing / inverse-inconsistent relationship, and a malformed
  `in_language` — matching the family convention (person / place /
  care-pathway services resolved this to `422`). `400` stays for
  malformed bodies (loco JSON rejection). (Lean: `422`; feeds T-3 /
  T-9.)
- **OQ-2 — Duplicate-check scale strategy.** `check-duplicates` (and
  the create-time `409`, FR-11a) will scan a capped set of stored rows
  **in the same collection** in memory at first. At portfolio volumes:
  search-based blocking (Tantivy), JSONB GIN pre-filtering on name /
  goal titles, or both? (Feeds the family-parity roadmap.)
- **OQ-3 — Sub-resource indexing & the JSONB boundary.** The thin
  record is pure JSONB; the sub-resources are relational. Confirm the
  index set (`tasks.status`, `tasks.assignee_ref`, `issues.severity`,
  `(parent_kind, parent_pid)`, child-kind `portfolio_pid`) and whether
  `goals` should ever be promoted out of `data.goals[]` into its own
  table if goal-level querying becomes hot (today it stays in the
  payload as the §5.3 bridge).
- **OQ-4 — `owner_org_id` / `portfolio_ref` identity.**
  `owner_org_id` is an `EntityRef` `organization:<id>`;
  `portfolio_ref` is an in-entity portfolio `pid`. Should the service
  soft-validate that the org / parent portfolio exists (lazy
  verify-on-read for the org, like the link aggregator,
  [cross-service-linking.md §5.1](../../agents/share/cross-service-linking.md);
  a local `portfolios` lookup for the parent), or treat both as opaque
  optimistic references (no target call on the write path)? (Lean:
  opaque + optimistic for the org; soft-validate `portfolio_ref`
  locally since it is in this service's own `portfolios` table.)
- **OQ-5 — Goal as payload field vs sub-resource — write authority.**
  Goals are both a payload field and a CRUD sub-resource (§5.3). When a
  goal is edited via the sub-resource endpoint and the work item is
  also `PUT` wholesale, which write wins? (Lean: last-writer-wins on
  `data.goals[]`, both transactional; document the merge so a
  concurrent goal add isn't lost — confirm in T-4.)
- **OQ-6 — Open-enum `Custom` arms vs free string.** `WorkItemStatus`
  / `GoalStatus` carry a `Custom(String)` arm; `WorkItemKind` does
  **not** (it is closed). `status` is never scored, so its `Custom`
  arm raises no matching question; for `GoalStatus`, is the open arm
  ever used in matching (goal *titles* are scored, not goal status, so
  the answer is no — confirm in T-2)?
- **OQ-7 — Cross-department duplicate threshold.** Two departments
  chartering the "same" project under different names, owner orgs, and
  codes will score mainly on name + goals (within the `projects`
  collection). Is the default threshold (0.85) right for portfolio
  dedup, or should the entity ship a tuned preset? (Lean: start at
  family default; tune from real data.)
- **OQ-8 — Sub-resource volume vs the matcher boundary.** Confirm the
  partition holds under load: a work item with thousands of tasks /
  issues must not bloat the matchable `data` (it does not — they are
  separate tables, §5.6 / §10.1), and `goals[]` must stay bounded
  (charter-level, not task-level). Should the spec cap `goals[]` length
  to keep the match input small? (Lean: soft cap with a warning;
  revisit with data.)
- **OQ-9 — Kind gate vs cross-kind near-duplicates.** The kind gate
  (§5.5) makes a project and a product never match, even when they
  describe the "same" initiative split across kinds. Is a *cross-kind*
  near-duplicate surfaced any other way (a relationship suggestion, the
  link aggregator), or deliberately ignored? (Lean: out of matcher
  scope; a future cross-service / cross-kind suggestion is a
  relationship hint, never a same-identity match — confirm with the
  roadmap aggregator.)
