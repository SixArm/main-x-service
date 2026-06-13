## 18. Change Control

Material changes to this spec — the DTO contract (§5), shared
invariants (§5.5), the API surface (§9), compliance scope (§12) —
MUST land in the same commit as the corresponding code change, with
the corresponding test change (three-part PR).

Authority boundaries (restated from [`index.md`](index.md)):

- **Crate internals** — the crate spec wins; mirror material changes
  here only when they touch the integration contract.
- **Integration contract** (DTO shape on the wire, JSONB persistence
  of the matcher type, endpoint inventory the front-end consumes,
  shared invariants) — this spec wins; a crate change that breaks it
  is a bug or a deliberate, three-spec change.

A change to `care_pathway_matcher::CarePathway` is automatically a
change to the wire format and the stored payload: it MUST update the
matcher spec, this spec's §5, the front-end's `types.ts`, and
CHANGELOGs in the same change cycle.

Bullet what changed, not how: every spec edit should be a diff a
reviewer can read in isolation. Keep stylistic re-flows out of
behavioural diffs.
