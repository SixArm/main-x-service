## 6. Functional Requirements

Each requirement maps to its owning subproject. Detail lives in the
owner's spec — links given per row; this table is the entity-wide
checklist.

| # | Requirement | Owner | Detail |
|---|---|---|---|
| FR-1 | Create / read / update / soft-delete Thing records, with automatic event publish on every CRUD | service | [service §6.1](../thing-service-rust-crate/spec/06-functional-requirements.md) |
| FR-2 | Multiple typed `PropertyValue` identifiers per record (DOI / ISBN / ISSN / GTIN / SKU / MPN / SerialNumber / URI / UUID / Custom), plus `alternate_names`, `images`, `same_as` | service | [service §5–§6.1](../thing-service-rust-crate/spec/05-domain-model.md) |
| FR-3 | Probabilistic matching — renormalised weighted score with per-field breakdown; canonical algorithm in the matcher (10 components, presets `strict` / `default` / `lenient`), summary scorer in the service (5 components) | matcher (canonical), service (embedded) | [matcher §5–§6](../thing-matcher-rust-crate/spec/05-matching-engine.md), [service §6.2](../thing-service-rust-crate/spec/06-functional-requirements.md) |
| FR-4 | Deterministic short-circuits — any shared deterministic identifier pair pins the score to 1.0; the matcher additionally short-circuits on shared `same_as` URL or equal canonical `url` | matcher + service | [matcher §5.1](../thing-matcher-rust-crate/spec/05-matching-engine.md) |
| FR-5 | Full-text + fuzzy + boolean search (Tantivy) over name / alternate names / description / identifier value / URL / same_as, with pagination and optional masking | service | [service §6.3](../thing-service-rust-crate/spec/06-functional-requirements.md) |
| FR-6 | Duplicate detection — real-time `409` on create, explicit check endpoint, batch deduplicate scan | service | [service §6.4](../thing-service-rust-crate/spec/06-functional-requirements.md) |
| FR-7 | Merge — transfer identifiers / aliases / `same_as` / images, `Replaces` link, soft-delete duplicate, JSON snapshot, `Merged` event | service | [service §6.4](../thing-service-rust-crate/spec/06-functional-requirements.md) |
| FR-8 | Review queue with `Pending` / `Confirmed` / `Rejected` / `AutoMerged` statuses and configurable auto-merge threshold | service | [service §6.4](../thing-service-rust-crate/spec/06-functional-requirements.md) |
| FR-9 | Privacy — per-field masking (`owner`, identifier values), GDPR Article 15 export, consent model for things linked to individuals | service | [service §6.6](../thing-service-rust-crate/spec/06-functional-requirements.md) |
| FR-10 | Audit — every CRUD / merge / link writes old + new JSON, user ID, IP, user agent, timestamp; queryable per-record and system-wide | service | [service §6.7](../thing-service-rust-crate/spec/06-functional-requirements.md) |
| FR-11 | Event streaming — `Created` / `Updated` / `Deleted` / `Merged` / `Linked` events on every change (in-memory today; durable bus is roadmap) | service | [agents/share/auditability.md](../../agents/share/auditability.md) |
| FR-12 | Operator UI — dashboard, list + search grid, create with 409 surfacing, detail / edit / soft-delete, audit view, match check, merge with preview | front-end | [front-end §5–§6](../thing-front-end-with-svelte/spec/index.md) |
| FR-13 | Validation and normalisation at the boundary — required `name`, URL formats, per-type identifier formats, dedupe, scheme lowercasing; failures return `422` | service | [service §6.5](../thing-service-rust-crate/spec/06-functional-requirements.md) |
| FR-14 | Standalone matching — the matcher MUST remain usable outside the service: pure library, builder API, batch `match_one_to_many` / `rank_one_to_many` | matcher | [matcher §5.3, §8](../thing-matcher-rust-crate/spec/08-public-api-surface.md) |

### Cross-subproject acceptance

- FR-3 / FR-4 are accepted entity-wide only when the **bridge tests**
  ([`tests/duplicate_detection.rs`](../thing-service-rust-crate/tests/duplicate_detection.rs))
  pass — they drive service-shaped records through the adapter into
  the matcher engine.
- FR-12 is accepted only against the real REST contract — the
  front-end's Playwright e2e suite plus a live-service walkthrough
  (currently pending, see §14).
