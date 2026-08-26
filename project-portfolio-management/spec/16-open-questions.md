## 16. Open Questions

Open questions resolve into §13 tasks or §5–§12 amendments when
decisions are made. Matcher-internal questions migrate to the matcher
spec §16 once that crate is scaffolded.

- **OQ-1 — Validation status code.** Confirm `422` (not `400`) for a
  blank `name`, a malformed `EntityRef` / `parent_ref`, a containment
  cycle, a malformed deterministic identifier, a self-referencing /
  inverse-inconsistent relationship, and a malformed `in_language` —
  matching the family convention (person / place / care-pathway
  services resolved this to `422`). `400` stays for malformed bodies
  (loco JSON rejection). (Lean: `422`; feeds T-3 / T-9.)
- **OQ-2 — Duplicate-check scale strategy.** `check-duplicates` (and
  the create-time `409`, FR-11a) will scan a capped set of stored plan
  rows in memory at first. At portfolio volumes: search-based blocking
  (Tantivy), JSONB GIN pre-filtering on name / goal titles, or both?
  (Feeds the family-parity roadmap.)
- **OQ-3 — Sub-resource indexing & the JSONB boundary.** The thin
  record is pure JSONB; the sub-resources are relational. Confirm the
  index set (`tasks.status`, `tasks.assignee_ref`, `issues.severity`,
  `parent_pid`) and whether `goals` should ever be promoted out of
  `data.goals[]` into its own table if goal-level querying becomes hot
  (today it stays in the payload as the §5.3 bridge).
- **OQ-4 — `owner_org_id` / `parent_ref` identity.**
  `owner_org_id` is an `EntityRef` `organization:<id>`;
  `parent_ref` is an in-entity parent-plan `pid`. Should the service
  soft-validate that the org / parent plan exists (lazy
  verify-on-read for the org, like the link aggregator,
  [cross-service-linking.md §5.1](../../agents/share/cross-service-linking.md);
  a local `plans` lookup for the parent), or treat both as opaque
  optimistic references (no target call on the write path)? (Lean:
  opaque + optimistic for the org; soft-validate `parent_ref`
  locally since it is in this service's own `plans` table — this is
  also where the containment-cycle check runs.)
- **OQ-5 — Goal as payload field vs sub-resource — write authority.**
  Goals are both a payload field and a CRUD sub-resource (§5.3). When a
  goal is edited via the sub-resource endpoint and the plan is
  also `PUT` wholesale, which write wins? (Lean: last-writer-wins on
  `data.goals[]`, both transactional; document the merge so a
  concurrent goal add isn't lost — confirm in T-4.)
- **OQ-6 — Open-enum `Custom` arms vs free string.** `PlanStatus`
  / `GoalStatus` carry a `Custom(String)` arm; `PlanKind` does
  **not** (it is a closed set of labels). `status` and the optional
  `kind` label are never scored, so their arms raise no matching
  question; for `GoalStatus`, is the open arm ever used in matching
  (goal *titles* are scored, not goal status, so the answer is no —
  confirm in T-2)?
- **OQ-7 — Cross-department duplicate threshold.** Two departments
  chartering the "same" initiative under different names, owner orgs, and
  codes will score mainly on name + goals. Is the default threshold
  (0.85) right for portfolio dedup, or should the entity ship a tuned
  preset? (Lean: start at family default; tune from real data.)
- **OQ-8 — Sub-resource volume vs the matcher boundary.** Confirm the
  partition holds under load: a plan with thousands of tasks /
  issues must not bloat the matchable `data` (it does not — they are
  separate tables, §5.6 / §10.1), and `goals[]` must stay bounded.
  Should the spec cap `goals[]` length to keep the match input small?
  (Lean: soft cap with a warning; revisit with data.) **Sharpened
  2026-08-25 by the OKR engine (§5.9.2):** the pressure on `goals[]` is
  now lower, not higher, because the volume an OKR practice generates —
  key results and dated check-ins — lives in its own tables. What rides
  in the payload is still one line per objective. The open part is
  whether an objective *title* set large enough to matter for Jaccard
  scoring is reachable in practice.
- **OQ-9 — Cross-label near-duplicates (kind gate removed).** With the
  kind gate gone (§5.5), two plans labelled `Project` and `Product`
  that describe the "same" initiative now **match directly** on their
  shared signals — the cross-label case this question once worried
  about is handled by kind-agnostic matching itself. Remaining nuance:
  when the labels genuinely mean *different* things (a product vs the
  project that builds it), should an operator preset down-weight or an
  operator disambiguate at review time? (Lean: rely on the other
  signals + the review queue; no label-based penalty — confirm with
  real data.)
