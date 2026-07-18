## 18. Change Control

Material changes to this spec — the canonical domain model (§5), the
`WorkItemKind` set and kind gate (§5.2 / §5.5), the matchable/operational
partition (§5.6), shared invariants (§5.8), the API surface (§9),
compliance scope (§12) — MUST land in the same commit as the
corresponding code change, with the corresponding test change
(three-part PR). Until the trio is scaffolded (§13 T-1), a "code
change" may be the scaffold itself; the discipline begins with the
first crate.

Authority boundaries (restated from [`index.md`](index.md)):

- **§5 is canonical.** This entity spec owns the `WorkItem` domain
  model, the four-kind discriminator, the kind gate, and the
  sub-resource partition. The matcher and service crate specs
  **reference** §5; a change to the model is a change here first,
  mirrored into the crates in the same cycle.
- **Crate internals** — the crate spec wins; mirror material changes
  here only when they touch the integration contract.
- **Integration contract** (the thin DTO shape on the wire, JSONB
  persistence of the matcher type, the kind gate, the four-collection
  endpoint inventory the front-end consumes, shared invariants, the
  matchable/operational partition) — this spec wins; a crate change
  that breaks it is a bug or a deliberate, three-spec change.

A change to `project_portfolio_management_matcher::WorkItem` (the thin record, incl.
`WorkItemKind` and `Goal`) is automatically a change to the wire format
and the stored payload: it MUST update the matcher spec, this spec's
§5, the front-end's `types.ts`, and CHANGELOGs in the same change
cycle. Adding or removing a `WorkItemKind` variant is the heaviest such
change — it adds or drops a service table and a REST collection — and
MUST be coordinated across all three specs plus the migration set. The
operational sub-resource shapes (`Task`, `Issue`) have **no** matcher
counterpart and are governed by the service crate spec; a change to
them updates the service spec, this spec's §5.7 / §6.4 / §9.2 / §10.1,
and the front-end types together.

The adopted family designs
([cross-service-linking.md](../../agents/share/cross-service-linking.md),
[bulk-import-export.md](../../agents/share/bulk-import-export.md)) are
the source of truth for their own contracts; this spec restates only
the portfolio-specific bits (§9.5, §9.6, §10.4) and tracks the rest by
reference — a change to those shared docs flows in without re-stating
their canonical tables here.

Bullet what changed, not how: every spec edit should be a diff a
reviewer can read in isolation. Keep stylistic re-flows out of
behavioural diffs.
