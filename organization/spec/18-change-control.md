## 18. Change Control

Material changes to this spec — the DTO contract, shared invariants,
the API-surface shape, compliance scope — MUST land in the same
commit as the corresponding code change, together with the affected
subproject spec(s).

Authority rules (restating the banner in [`index.md`](index.md)):

- **Crate internals** — the subproject's own spec wins. Changing the
  matcher's weights, the service's table layout, or a front-end form
  is a *crate-spec* edit; update this document only if the
  integration contract is touched.
- **Integration contract** — this spec wins. Changing the wire field
  names, the JSONB persistence rule, an endpoint's path/shape, a
  shared invariant (§5.5), or the deterministic-scheme list requires
  an edit **here** plus the affected crate specs, in one PR.
- Disagreements found later become §13 tasks — never a silent
  rewrite of either document.

House rules: bullet what changed, not how; every spec edit should be
a diff a reviewer can read in isolation; keep stylistic re-flows out
of behavioural PRs. Subproject releases additionally bump their own
`CHANGELOG.md` under `[Unreleased]`.
