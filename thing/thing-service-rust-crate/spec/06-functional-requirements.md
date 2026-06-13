## 6. Functional Requirements

### 6.1 Identity management

- Create / read / update / soft-delete Thing records.
- Multiple typed identifiers per Thing (`PropertyValue` list).
- Multiple `alternate_names`, `images`, `same_as` URLs.
- Automatic event publish on every CRUD.

### 6.2 Matching

Algorithm reference: [`AGENTS/matching.md`](../AGENTS/matching.md).

Default component weights (sum to 1.0):

| Component | Weight | Algorithm |
|---|---:|---|
| Name | 0.40 | Jaro-Winkler (case-insensitive) |
| Identifier | 0.30 | Exact `(property_id, value)` |
| Description | 0.10 | Jaro-Winkler |
| URL | 0.10 | Scheme/case-normalized host + path |
| Same-as | 0.10 | Best URL pair across `sameAs` lists |

Deterministic short-circuit: any matching DOI / ISBN / ISSN / GTIN /
MPN / SerialNumber / UUID → 1.0.

Phonetic bonus: +0.05 when name Soundex matches and base score < 0.95.

Match quality (configurable thresholds):

| Quality | Score |
|---|---|
| Certain | ≥ 0.95 |
| Probable | ≥ 0.80 |
| Possible | ≥ 0.60 |
| Unlikely | < 0.60 |

#### Interoperability with `thing-matcher`

The service embeds the sibling `thing-matcher` crate (declared in
`Cargo.toml`) and re-exports it from `src/matching/mod.rs` as
`matcher_lib`. The matcher crate is the **canonical reference
algorithm** — it scores 10 schema.org/Thing components (the service
scores 5), exposes 12 tunable config knobs including three presets
(`strict` / `default` / `lenient`), and uses an opaque `(property_id,
value)` identifier shape that accepts any vocabulary the service can
emit.

Bridge: [`src/matching/adapter.rs`](../src/matching/adapter.rs) exposes
`to_matcher_thing(&service::Thing) -> thing_matcher::Thing`. The
projection lifts the service's `Thing` (schema.org-shaped with the
`PropertyValue` identifier wrapper) into the matcher's builder:

- `name`, `description`, `disambiguating_description`, `url`,
  `main_entity_of_page`, `owner` map 1:1
- `alternate_names: Vec<String>` → `alternate_names`
- `additional_type` (singular) → first entry of `additional_types`
- `subject_of` (singular) → first entry of `subject_of`
- `images: Vec<String>` → first entry of `image` (matcher takes one)
- `same_as: Vec<String>` → `same_as`
- `identifiers[]` mapped via `map_identifier_property`: schema.org
  canonical tokens (`doi`, `isbn`, `issn`, `gtin`, `sku`, `mpn`,
  `serialNumber`, `uri`, `uuid`); `Custom(s)` passes the carried
  label through verbatim. Identifier `name` / `url` metadata is
  dropped (the matcher does not consume it).

Registry-only fields (`id`, `is_deleted`, `created_at`,
`updated_at`, `potential_action`) are dropped — they have no
matcher counterpart. See [`AGENTS/matching.md`](../AGENTS/matching.md)
for the in-service algorithm and the matcher crate's
[`spec.md §5–§6`](../../thing-matcher-rust-crate/spec/index.md) for the
canonical algorithm.

### 6.3 Search

Tantivy across `name`, `alternate_names`, `description`,
`identifier.value`, `url`, `same_as`. Full-text + fuzzy + boolean.
Pagination (`offset` + `limit`). Optional masking for Things subject
to consent constraints.

### 6.4 Duplicate detection and merging

- Real-time `409 Conflict` on `POST /api/things` when an existing
  Thing matches on a deterministic identifier or on name + URL.
- Explicit `POST /api/things/check-duplicates`.
- Batch `POST /api/things/deduplicate`.
- Review queue (`Pending` / `Confirmed` / `Rejected` / `AutoMerged`).
- Merge transfers identifiers, `alternate_names`, `same_as`,
  `images`; appends the duplicate's name as `alternate_name` on the
  survivor; adds a `Replaces` link; soft-deletes the duplicate;
  records a JSON snapshot; emits a `Merged` event.

### 6.5 Validation and normalisation

Required `name`; URL formats on every URL-valued property; identifier
formats per type (see §5.4). Normalisation trims text, lowercases URL
schemes (host / path preserved), dedupes `alternate_names`,
`same_as`, `images`. Failed validation → `422`.

### 6.6 Privacy

Per-field masking: `owner` → `"[owner withheld]"`; identifier `value`
→ `"****<last 4 chars>"`; per-identifier `url` cleared on mask;
`property_id` preserved. GDPR Article 15 export at
`GET /api/things/{id}/export`. Consent model when a Thing is attached
to a person.

### 6.7 Audit

Every CRUD / merge / link writes to `audit_log` with old + new JSON,
user ID, IP, user agent, timestamp.

