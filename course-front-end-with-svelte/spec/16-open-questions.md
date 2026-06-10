## 16. Open Questions

- **OQ-1**: SVAR DataGrid free tier is GPL-3.0. If this front-end ships in a commercial / proprietary deployment, what license tier do we need? Pro? Enterprise? Decision required before any production deploy.
- **OQ-2**: Should the create route call `check-duplicates` for an inline preview before the actual `create` POST, or rely solely on the 409 round-trip? Round-trip is simpler; preview is friendlier. Operator feedback needed.
- **OQ-3**: When the service returns `403`/`401` (post-auth), how should the UI redirect? Tied to whatever auth flow the service chooses (JWT vs session vs OAuth).
- **OQ-4** *(resolved 2026-06-05)*: Drift policy was set on
  2026-06-02 — keep API client + types in each front-end project,
  revisit when the third sibling ships. Six entity front-ends are
  now live (person / worker / place / thing / event / course) and
  the drift has stayed bounded: this session caught a handful of
  flat shape mismatches (e.g. `ScoredCandidate` adding `name` +
  `course_code`) which fixed cleanly per-project without needing
  a shared package. Decision stands: no shared `mxi-svelte-core`
  package; carry copy-adapted code per project.

