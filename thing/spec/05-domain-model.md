## 5. Domain Model

Field-by-field references:
[service `AGENTS/models.md`](../thing-service-rust-crate/AGENTS/models.md)
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

The front-end mirrors this shape in
[`src/lib/api/types.ts`](../thing-front-end-with-svelte/src/lib/api/types.ts)
(its own copy — drift policy 2026-06-02).

### 5.2 Matcher `Thing` (comparison shape)

The matcher's `Thing` is `#[non_exhaustive]`, builder-constructed,
and slightly wider where comparison needs it: `additional_types` and
`subject_of` are lists, `image` is singular, identifiers are opaque
`(property_id: String, value: String)` pairs, and it carries a
`local_id` (data-only, never scored). See
[matcher spec §3.1](../thing-matcher-rust-crate/spec/03-data-model.md).

### 5.3 The DTO contract: service → matcher adapter

The service embeds `thing-matcher` (Cargo.toml: `thing-matcher =
"0.6.1"`) and re-exports it from `src/matching/mod.rs` as
`matcher_lib`. The bridge is
[`src/matching/adapter.rs`](../thing-service-rust-crate/src/matching/adapter.rs):

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
| `id`, `is_deleted`, timestamps, `potential_action` | — | dropped (no matcher counterpart) |

Both sides of this contract are pinned by
[`tests/duplicate_detection.rs`](../thing-service-rust-crate/tests/duplicate_detection.rs)
(15 bridge tests). A change to either the adapter's routing rules or
the matcher's scoring MUST update this section, the adapter, and the
bridge tests in one PR.

### 5.4 Shared invariants

Every subproject MUST uphold:

- `name` is non-empty.
- An identifier is keyed by `(property_id, value)`; duplicates within
  one record are deduplicated.
- All URL-valued properties use `http://` or `https://`.
- Per-type identifier formats per
  [service spec §5.4](../thing-service-rust-crate/spec/05-domain-model.md).
- Soft delete is the only delete.
- Match scores are `f64` in `[0.0, 1.0]` with a per-component
  breakdown — never a bare score.
