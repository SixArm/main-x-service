# Module 1 — Content modelling & authoring

## Operator-defined content types

An admin declares a **ContentType** as data: a list of typed fields
with validation, whether the type is routable, and which template
contract it uses ([domain-model](domain-model.md)). Adding an
"Article" or a "Job posting" is an API call, not a migration.

**Schema versioning is explicit.** Any field change bumps
`schema_version`. Existing revisions were validated against the
version they were written under and keep validating against it;
they are re-validated only when re-saved under the current version.
This is the honest position: a live CMS cannot retro-validate a
million stored documents on every schema edit, and pretending
otherwise would mean either lying about validity or blocking schema
evolution. A **compatibility check** classifies each change as
`additive` (new optional field), `tightening` (new required field,
narrowed validation — flags existing entries as `needs_migration`
in insights), or `breaking` (field removed or kind changed —
requires an explicit `confirm_breaking` flag and is audited with a
reason).

Field kinds: `text`, `rich_text`, `number`, `boolean`, `date`,
`datetime`, `choice` (declared options), `media` (asset
reference), `reference` (another entry, optionally type-scoped),
`entity_ref` (a family `EntityRef` URN, optionally type-scoped —
the [scope](scope.md) boundary), `url`, `geo`, `json` (escape
hatch, capped, never rendered as markup).

## Structured authoring, not HTML

Content bodies are **block documents**: an ordered list of typed
blocks with structured inline marks
([domain-model](domain-model.md)). This is the load-bearing
authoring decision ([design](design.md) CMS-D5), for three
reasons:

1. **Safety.** A stored-HTML CMS is a stored-XSS engine. Blocks
   carry no markup, so there is nothing to smuggle. Where HTML is
   accepted at all (import, the `embed` block), it is sanitized
   against an allow-list **at write time** and re-escaped at
   delivery — never stored raw and trusted later.
2. **Portability.** The same document renders to a web page, an
   app screen, and a kiosk panel — the point of headless delivery.
3. **Queryability.** Blocks and references are extractable, which
   is what makes "where is this asset used" and "which links are
   broken" answerable at all.

Validation on save: block kinds allow-listed per region/type, nest
depth capped, text length and array cardinality capped (the family
input-size invariants), unknown block kinds refused with `422`
naming the offending path (e.g. `blocks[3].kind`).

## References are extracted, not implied

On every save the pure core walks the block document and field
values and writes **Reference** rows for every entry, asset, and
`EntityRef` mentioned. Consequences:

- **"Where used"** is an index lookup, not a full scan.
- **Deleting a referenced asset or entry is refused** (`409` listing
  the referrers) — the CMS failure mode where a page silently loses
  its hero image does not exist here.
- **Broken references** (target archived, unpublished, or missing)
  are a derived insight ([insights](insights.md)), not a
  404 discovered by a reader.

## Revisions

Every save writes an append-only **Revision** (monotonic `number`
per variant) with the author, an optional note, and the full body —
full snapshots, not deltas: storage is cheap, and a delta chain
that cannot be replayed is a history you do not actually have.

- **Diff** between any two revisions is derived (block-level:
  added / removed / changed, plus field-level scalar diffs).
- **Restore** writes a **new** revision copying an old body, with
  `restored_from_pid` recorded. History is never rewritten or
  truncated — the same posture as the family's tamper-evident audit
  trail ([audit](audit.md)).
- **Concurrency**: a save states the `base_revision_pid` it edited.
  If the variant has advanced, the save is refused `409` with the
  competing revision — last-write-wins silently losing an editor's
  work is not an acceptable default. Optional advisory **locks**
  (`locked_by_ref` + `locked_until`, auto-expiring) reduce the
  collision rate but never replace the check
  ([workflow](workflow.md)).

## Preview

An unpublished revision is readable only by an authorized editor,
or through a **short-lived scoped preview token** naming exactly one
revision ([auth](auth.md)). There is no "secret URL" that is
guessable, permanent, or transferable to another revision.
