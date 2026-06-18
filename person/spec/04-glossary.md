## 4. Glossary

Entity-level terms. Per-subproject vocabularies:
service [spec §4](../person-service-with-loco/spec/04-glossary.md),
matcher [spec §4](../person-matcher-rust-crate/spec/04-glossary.md),
front-end [spec §4](../person-front-end-with-svelte/spec/04-glossary.md).

| Term | Meaning |
|---|---|
| **Entity** | One domain concept (here: Person) delivered as a trio of subprojects in one directory |
| **Trio** | The three subprojects: service crate, matcher crate, front-end project |
| **Entity-level spec** | This document set — source of truth for the cross-subproject contract |
| **Crate spec** | A subproject's own `spec/` — source of truth for that subproject's internals |
| **Person** | The canonical record for an individual: HumanName, identifiers, addresses, documents, emergency contacts |
| **Service model** | The service's FHIR-shaped `Person` (`src/models/person.rs`) — what the REST API serves |
| **Matcher model** | The matcher's flat `Person` builder shape — what `MatchingEngine` scores |
| **Adapter** | `src/matching/adapter.rs` in the service — the lossy projection service model → matcher model (§5.3) |
| **Canonical algorithm** | The matcher crate's scoring — the reference the service embeds as `matcher_lib` |
| **Envelope** | The REST response wrapper `{ "success": bool, "data": …, "error": … }` shared by service and front-end |
| **Match** | A comparison between two persons yielding a 0.00–1.00 score plus per-component breakdown |
| **Merge** | Transfers a duplicate's data onto a surviving record, soft-deletes the duplicate, writes a `Replaces` link |
| **Review queue** | Persisted candidate duplicate pairs: `Pending` / `Confirmed` / `Rejected` / `AutoMerged` |
| **Soft delete** | Retention with `active = false`; never `DELETE FROM` — the entity-wide erasure mechanism |
| **SSO** | Single sign-on via the [authentication entity](../../authentication/): magic-link, RS256 JWT + JWKS |
| **Bridge test** | Service-side test (`tests/duplicate_detection.rs`) that pins both the adapter and the matcher output |
| **Drift policy** | Front-ends keep per-project copies of types/client/forms; no shared package (repo decision 2026-06-02) |
