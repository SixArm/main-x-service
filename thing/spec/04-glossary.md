## 4. Glossary

Entity-level terms. Per-crate glossaries:
[service §4](../thing-service-rust-crate/spec/04-glossary.md),
[matcher §2](../thing-matcher-rust-crate/spec/02-terminology.md).

| Term | Meaning |
|---|---|
| **Thing** | A discrete object — book, paper, software, device, product, public asset — per [schema.org/Thing](https://schema.org/Thing) |
| **Trio** | The three subprojects composing this entity: service + matcher + front-end |
| **System of record** | `thing-service-rust-crate` — owns persistence, audit, and the REST contract |
| **Canonical algorithm** | `thing-matcher-rust-crate` — the reference matching implementation the service embeds |
| **PropertyValue** | The schema.org identifier shape `{ propertyID, value, name?, url? }` |
| **Deterministic identifier** | DOI / ISBN / ISSN / GTIN / MPN / SerialNumber / UUID — globally unique by construction; a shared pair short-circuits matching |
| **Non-deterministic identifier** | SKU / URI / Custom — evidence, not a hard pin (service-side distinction; the matcher treats `property_id` as opaque) |
| **DTO contract** | The projection `to_matcher_thing(&service::Thing) -> thing_matcher::Thing` in [`adapter.rs`](../thing-service-rust-crate/src/matching/adapter.rs) — see §5.3 |
| **Bridge tests** | [`tests/duplicate_detection.rs`](../thing-service-rust-crate/tests/duplicate_detection.rs) — black-box tests pinning both sides of the DTO contract |
| **Match quality** | Service vocabulary: Certain ≥ 0.95 / Probable ≥ 0.80 / Possible ≥ 0.60 / Unlikely |
| **Confidence band** | Matcher vocabulary: High ≥ 0.90 / Medium ≥ 0.75 / Low — fixed across presets (see §16 OQ-2) |
| **Review queue** | Duplicate candidates held as `Pending` / `Confirmed` / `Rejected` / `AutoMerged` |
| **Soft delete** | `is_deleted = true`; rows are never `DELETE`d — the only delete in the entity |
| **Operator** | A signed-in registry user working through the front-end |
