# Domain model — Thing Entity orientation

Three representations of one schema.org/Thing concept. This page gives
the shape and the pointer — full field tables live in the per-crate
docs; do not duplicate them here.

## The three shapes

| Shape | Where | Reference |
|---|---|---|
| Registry `Thing` | `thing-service-rust-crate/src/models/thing.rs` — schema.org properties + `PropertyValue` identifiers + registry-internal `id` / soft-delete / timestamps | [service AGENTS/models.md](../thing-service-rust-crate/AGENTS/models.md) |
| Matcher `Thing` | `thing-matcher-rust-crate/src/models.rs` — comparison-oriented, `#[non_exhaustive]`, builder-constructed, opaque string identifiers, unscored `local_id` / `owner` | [matcher spec §3](../thing-matcher-rust-crate/spec/03-data-model.md) |
| Wire types | `thing-front-end-with-svelte/src/lib/api/types.ts` — TypeScript mirror of the service's REST DTOs (per-project copy; drift accepted) | [front-end AGENTS.md](../thing-front-end-with-svelte/AGENTS.md) |

## Key shape differences (why the adapter exists)

| Concern | Service | Matcher |
|---|---|---|
| Identifier | `ThingIdentifier { property_id: IdentifierType, value, name?, url? }` | `Identifier { property_id: String, value: String }` — opaque, case-sensitive |
| `additional_type` | singular `Option<String>` | `additional_types: Vec<String>` |
| `image(s)` | `images: Vec<String>` | `image: Option<String>` (first wins) |
| `subject_of` | singular | list |
| Registry fields (`id`, soft delete, timestamps, `potential_action`) | present | absent |
| Construction | struct + `Thing::new(name)` | `Thing::builder()` only (`#[non_exhaustive]`) |

The projection is `to_matcher_thing` in
[`adapter.rs`](../thing-service-rust-crate/src/matching/adapter.rs);
the normative mapping table is entity spec
[§5.3](../spec/05-domain-model.md). Identifier `property_id` maps to
schema.org canonical tokens (`doi`, `isbn`, `issn`, `gtin`, `sku`,
`mpn`, `serialNumber`, `uri`, `uuid`); `Custom(s)` passes through
verbatim.

## Identifier semantics (entity-wide invariant)

- **Deterministic** (globally unique; short-circuit to 1.0): DOI,
  ISBN, ISSN, GTIN, MPN, SerialNumber, UUID.
- **Non-deterministic** (evidence only): SKU, URI, Custom. The
  deterministic / non-deterministic distinction is enforced on the
  **service side**; the matcher treats any shared `(property_id,
  value)` pair as deterministic.
- Format rules (ISBN digit counts, DOI `10.` prefix, …): service
  spec [§5.4](../thing-service-rust-crate/spec/05-domain-model.md).

## Supporting types

Consent (`DataProcessing` / `DataSharing` / `Marketing` / `Research`),
merge records, review-queue items — service-side only; see
[service AGENTS/models.md](../thing-service-rust-crate/AGENTS/models.md).
