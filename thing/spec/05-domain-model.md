## 5. Domain Model

Field-by-field references:
[service `AGENTS/models.md`](../thing-service-with-loco/AGENTS/models.md)
(registry shape) and
[matcher spec §3](../thing-matcher-rust-crate/spec/03-data-model.md)
(comparison shape). This section owns the **contract between the two**.

### 5.1 Canonical `Thing` (registry shape)

The service models schema.org/Thing one-to-one: `name` (required),
`alternate_names`, `description`, `disambiguating_description`,
`additional_type`, `url`, `identifiers` (`Vec<ThingIdentifier>`,
the `PropertyValue` shape), `images`, `main_entity_of_page`, `owner`,
`same_as`, `subject_of`, `potential_action` — plus registry-internal
`id` (UUID), `is_deleted` / `deleted_at`, `created_at`, `updated_at`.

`IdentifierType` variants: deterministic `Doi`, `Isbn`, `Issn`,
`Gtin`, `Mpn`, `SerialNumber`, `Uuid`; non-deterministic `Sku`,
`Uri`, `Custom(String)`.

**Relationships** — typed thing-to-thing links:
`relationships: Vec<ThingRelationship>`, each `{ relation, thing_id }`
**referencing another `Thing` in the registry**. `relation` is a
`RelationKind` enum, initially **`Contains`**, **`ContainedIn`**,
**`SuperPart`**, and **`SubPart`**:

- `Contains` / `ContainedIn` are **inverses** — A `Contains` B (A is the
  container) ⇔ B `ContainedIn` A.
- `SuperPart` / `SubPart` are **inverses** (schema.org `hasPart` /
  `isPartOf`) — A `SuperPart` B means A is the whole / super-part and B a
  sub-part; B `SubPart` A is a sub-part of A.

These generalise simple containment / part-of hierarchy into a single
typed-link list; the enum is extensible (e.g. `SimilarTo`, `Replaces` /
`ReplacedBy` later). Relationships are a **supporting** match signal, not
identifying on their own.

**Tags** — `tags: Vec<String>`, a list of short free-text labels an
operator can attach to a record for grouping, filtering, triage, or
workflow (e.g. `"vip"`, `"review"`, `"archived-2026"`, `"fast-track"`).
**Any `Thing` can carry tags.** Each tag is a short, trimmed, non-empty
string; the list is unordered, de-duplicated case-insensitively, and
defaults to empty. The `Thing` registry has no `keywords` field, so
**tags are the labelling mechanism** — user-applied **operational
labels** for grouping and workflow, distinct from the descriptive /
discovery properties (`name`, `alternate_names`, `description`,
`additional_type`, `subject_of`) that say *what the record is*.

Tags are a **supporting match signal**: the matcher scores them by set
Jaccard over the case-insensitively normalised tag sets (weighted
`tags_weight`, default `0.05`) and they **are** projected into the matcher
`Thing` (see §5.3). They are a supporting signal only — shared tags never
single-handedly establish a match, so do not treat tags as identifying.

This canonical model is **upstream**: the service model
([`AGENTS/models.md`](../thing-service-with-loco/AGENTS/models.md)),
the matcher DTO ([matcher spec §3](../thing-matcher-rust-crate/spec/03-data-model.md),
which carries `tags` per §5.3), and the front-end types
([`src/lib/api/types.ts`](../thing-front-end-with-svelte/src/lib/api/types.ts))
follow in the same change cycle.

The front-end mirrors this shape in
[`src/lib/api/types.ts`](../thing-front-end-with-svelte/src/lib/api/types.ts)
(its own copy — drift policy 2026-06-02).

### 5.2 Matcher `Thing` (comparison shape)

The matcher's `Thing` is `#[non_exhaustive]`, builder-constructed,
and slightly wider where comparison needs it: `additional_types` and
`subject_of` are lists, `image` is singular, identifiers are opaque
`(property_id: String, value: String)` pairs, it carries
`relationships: Vec<RelationshipRef>` (typed `(relation, thing_id)`
refs, scored by typed-set Jaccard), it carries `tags: Vec<String>`
(scored by case-insensitive set Jaccard), and it carries a
`local_id` (data-only, never scored). See
[matcher spec §3.1](../thing-matcher-rust-crate/spec/03-data-model.md).

### 5.3 The DTO contract: service → matcher adapter

The service embeds `thing-matcher` (Cargo.toml: `thing-matcher =
"0.6.1"`) and re-exports it from `src/matching/mod.rs` as
`matcher_lib`. The bridge is
[`src/matching/adapter.rs`](../thing-service-with-loco/src/matching/adapter.rs):

```
to_matcher_thing(&service::Thing) -> thing_matcher::Thing
```

Projection rules (normative — pinned by the bridge tests):

| Service field | Matcher field | Rule |
|---|---|---|
| `name`, `description`, `disambiguating_description`, `url`, `main_entity_of_page`, `owner` | same | 1:1 |
| `alternate_names: Vec<String>` | `alternate_names` | 1:1 |
| `additional_type` (singular) | `additional_types` | first entry |
| `subject_of` (singular) | `subject_of` | first entry |
| `images: Vec<String>` | `image` | first entry (matcher takes one) |
| `same_as: Vec<String>` | `same_as` | 1:1 |
| `identifiers[]` | `identifiers[]` | `property_id` mapped to schema.org canonical tokens (`doi`, `isbn`, `issn`, `gtin`, `sku`, `mpn`, `serialNumber`, `uri`, `uuid`); `Custom(s)` passes the label through verbatim; identifier `name` / `url` metadata dropped |
| `relationships[]` | `relationships[]` | typed `(relation, thing_id)` refs passed through 1:1; scored by typed-set Jaccard (matcher §6.6), weighted `relationships_weight`. NOT dropped — it is a supporting match signal |
| `tags: Vec<String>` | `tags[]` | passed through 1:1; scored by set Jaccard over the case-insensitively normalised tag sets (matcher §5.9.2 / §6.8), weighted `tags_weight`. NOT dropped — it is a supporting match signal |
| `id`, `is_deleted`, timestamps, `potential_action` | — | dropped (no matcher counterpart) |

Both sides of this contract are pinned by
[`tests/duplicate_detection.rs`](../thing-service-with-loco/tests/duplicate_detection.rs)
(15 bridge tests). A change to either the adapter's routing rules or
the matcher's scoring MUST update this section, the adapter, and the
bridge tests in one PR.

#### Confidence-vocabulary bridge (normative)

The service and the embedded matcher classify the **same** `[0.0, 1.0]`
score with **different vocabularies and different cut points**:

- Service — `MatchConfidence::from_score`
  (`src/matching/scoring.rs`): Certain ≥ 0.95, Probable ≥ 0.80,
  Possible ≥ 0.60, else Unlikely.
- Matcher — `thing_matcher::Confidence::from_score`: High ≥ 0.90,
  Medium ≥ 0.75, else Low.

The band edges **interleave** (0.95 / 0.80 / 0.60 vs 0.90 / 0.75), so
there is **no 1:1 label mapping** — e.g. matcher High spans service
Certain plus the top of Probable. The full score-range overlay table
lives in service
[`AGENTS/matching.md`](../thing-service-with-loco/AGENTS/matching.md).

Normative rule: the service MUST **re-classify from the raw `f64`
score**, never from the matcher's label. The adapter carries only the
domain record (`to_matcher_thing`); the matcher's `Confidence` label is
never translated back into the service vocabulary. `compute_match`
derives `MatchConfidence` solely via `MatchConfidence::from_score`, and
the API layer (`confidence_label`) renders that service band. The exact
cut points — including the matcher's interleaving edges (0.90, 0.75) —
are pinned by `MatchConfidence::from_score`'s
`test_confidence_boundary_pins` unit test. Any future "map at the
adapter" design (OQ-2) MUST still re-derive from the score, not from a
label.

### 5.4 Shared invariants

Every subproject MUST uphold:

- `name` is non-empty.
- An identifier is keyed by `(property_id, value)`; duplicates within
  one record are deduplicated.
- All URL-valued properties use `http://` or `https://`.
- Per-type identifier formats per
  [service spec §5.4](../thing-service-with-loco/spec/05-domain-model.md).
- A `ThingRelationship` references an **existing** `Thing`; **no thing
  relates to itself** (not its own container / sub-part). The directional
  kinds `Contains` / `ContainedIn` and `SuperPart` / `SubPart` must stay
  **acyclic** (no thing is its own container or super-part, directly or
  transitively) and, where both directions are stored, mutually consistent
  (A `Contains` B ⇔ B `ContainedIn` A; A `SuperPart` B ⇔ B `SubPart` A).
  Symmetric kinds (e.g. a future `SimilarTo`) must be stored symmetrically.
- Tags are short, trimmed, non-empty, and de-duplicated
  case-insensitively; the list is unordered and defaults to empty.
- Soft delete is the only delete.
- Match scores are `f64` in `[0.0, 1.0]` with a per-component
  breakdown — never a bare score.
