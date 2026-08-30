---
name: sixarm-services-skill
description: Explains the Main X Index (main-x-service) in plain terms — what it is, its core concepts (entity registry, matching, search, merge, cross-service linking), its terminology, and worked examples. Use when someone asks what this repo does, how a concept like "deterministic matching" or "the review queue" works, wants a glossary term explained, or wants a concrete example (a request/response, a matching score, a merge) rather than an implementation walkthrough.
---

# SixArm Services — concepts and examples

This skill explains the **Main X Index** (this repository,
`main-x-service`) to someone who wants to understand *what it is and
how it works*, not how to change its code. For implementation work,
use the `sixarm-services-maintainer-skill` instead.

## What this is, in one paragraph

The Main X Index is a **federated identity index**: a family of small
services, one per kind of real-world entity (a person, a workplace, a
piece of equipment, an event, a course, an organization, a clinical
care pathway, a governmental case, a project or plan), each doing the
same four things for its entity — **store records, find likely
duplicates, merge confirmed duplicates, and let other services and
front-ends read and search them**. A handful of cross-cutting pieces
(a single sign-on provider, a cross-service link graph) tie the
entities together without merging them into one big database.

Read [`agents/share/overview.md`](../agents/share/overview.md) for the
full crate-by-crate map; this skill is the plain-language layer above
it.

## Core concepts

**Entity registry.** One service per entity kind (person, worker,
place, thing, event, course, organization, care pathway, case,
portfolio/plan). Each is a small CRUD API over one PostgreSQL table,
with the same shape everywhere: create, read, update, soft-delete,
search, match, merge, audit.

**Matching.** Comparing two records of the same entity and producing a
confidence score from 0.00 (certainly different) to 1.00 (certainly
the same), plus a breakdown of which fields drove the score. Two
strategies, used together:

- *Probabilistic* — a weighted blend of fuzzy comparisons (name
  spelling, date closeness, address similarity, …). Good at catching
  typos and near-misses.
- *Deterministic* — a short-circuit rule: if two records share an exact
  identifier that can only mean one real-world thing (a tax ID, a
  passport number, a global location number), the score is pinned to
  1.00 regardless of anything else. Good at catching records that
  *look* different but are provably the same.

See [`agents/share/match.md`](../agents/share/match.md).

**Duplicate detection and the review queue.** When a new record is
created, or on request, the service searches for records that might
already represent the same real-world thing. A high-confidence hit can
block creation outright (409 Conflict) or auto-merge; a medium
confidence hit is queued for a human to confirm or reject rather than
decided automatically. That queue is a real, persisted list — not a
one-off report — so a reviewer can come back to it.

**Merge.** Once two records are confirmed as duplicates, one is chosen
as the survivor and the other's data (identifiers, alternate names,
contacts, …) is folded in. The loser is soft-deleted, not erased, and
the merge itself is recorded with a full snapshot of what moved where —
so a merge can always be explained, and in principle reasoned about
later, even though it isn't undone automatically.

**Full-text search.** Beyond exact lookups, most registries index
their records for fuzzy and phonetic search (so "Jon Smith" can find
"John Smyth"), separate from the matching engine above — search finds
*candidates*, matching *scores* them.

**Audit trail.** Every create, update, delete, and merge is logged —
who did it, what changed, when — as its own durable record, distinct
from the entity data itself. Several registries also emit a stream of
these events for other systems to consume in real time.

**Cross-service linking.** A person and a worker record can represent
the same human; a worker can be employed by an organization; a case can
be about a person. These relationships live in an edge table and a
read-only aggregator service, deliberately *not* folded into the
matching engines above — "these two records are the same" and "these
two records are related" are different questions with different
answers. See
[`agents/share/cross-service-linking.md`](../agents/share/cross-service-linking.md).

**FHIR.** Several registries also speak [HL7
FHIR](https://hl7.org/fhir/) — the standard healthcare/registry
interchange format — as a second representation of the same data, for
clients that expect a `Patient`, `Organization`, or similar resource
rather than this repo's own JSON shape.

## Glossary

| Term | Meaning |
|---|---|
| Entity | One kind of real-world thing this repo tracks an identity for (person, place, organization, …) |
| Registry / entity service | The one Rust service that owns one entity's records |
| Matcher | The algorithm (and the crate) that scores how alike two records of one entity are |
| Deterministic short-circuit | A rule that pins a match score to 1.00 on an exact, unambiguous identifier match |
| Review queue | The persisted list of candidate-duplicate pairs waiting for a human decision |
| Merge | Folding a confirmed duplicate's data into a survivor record and retiring the duplicate |
| Soft delete | Marking a record inactive rather than removing its row — it still exists for audit and history |
| `EntityRef` | The `entity_type:uuid` string that names one record from any service, used to link across services |
| ABAC | Attribute-based access control — who may do what, decided from attributes of the caller and the record, not a fixed role list |
| PASETO | The signed-token format this repo uses for service-to-service auth (deliberately not JWT — see [`agents/share/jwt.md`](../agents/share/jwt.md)) |
| Outbox / durable event bus | The transactional pattern that guarantees an emitted event and its database change either both commit or neither does |
| Loco | The Rust web framework (on top of Axum) most of these services are built with |

## Worked examples

**A duplicate on create.** Two operators independently register
"Jonathan Smith, born 1985-03-14" and "Jon Smith, born 1985-03-14" as
separate person records, days apart. The second `POST` runs matching
against the first automatically: name similarity is high, birth date
is exact, and the combined score clears the review threshold but not
the auto-merge threshold — so the second create is queued for a human
review rather than silently rejected or silently accepted as a new
person.

**A deterministic match.** Two organization records both carry the
same [LEI](https://www.gleif.org/en/about-lei/introducing-the-legal-entity-identifier-lei)
(Legal Entity Identifier), a globally unique registered code. Even if
their names are spelled differently ("Acme Corp" vs "ACME
Corporation"), the shared LEI alone pins the match score to 1.00 — the
deterministic rule overrides the fuzzy name comparison rather than
averaging with it.

**A merge.** A reviewer confirms the two "Smith" records above are the
same person. The merge keeps the earlier-created record as the
survivor, adds "Jonathan Smith" as a former/alias name on it, copies
over any identifiers or contacts the duplicate had that the survivor
lacked, soft-deletes the duplicate, and writes one merge record
capturing exactly what was transferred — so the decision is traceable
later, not just applied.

**A cross-service question.** "Does this person work at this
organization?" is answered by walking a `works_at` edge in the link
graph, not by matching a `person` record against an `organization`
record — matching only ever asks "are these two records of the *same
entity kind* the same real thing," never "how are these two different
kinds of record related."

## Where to go deeper

- [`agents/share/overview.md`](../agents/share/overview.md) — the full
  service, matcher, and front-end inventory
- [`agents/share/architecture.md`](../agents/share/architecture.md) —
  how a request flows through one service
- [`agents/share/match-search-merge.md`](../agents/share/match-search-merge.md) —
  the three workflows together
- Per-entity `spec/index.md` (or `spec/01-*.md` … for the six oldest
  crates) — the canonical behaviour for one specific service
