## 4. Glossary

Entity-level terms. Per-subproject vocabularies:
service [spec §4](../place-service-with-loco/spec/04-glossary.md),
matcher [spec §2](../place-matcher-rust-crate/spec/02-terminology.md),
front-end [spec §4](../place-front-end-with-svelte/spec/04-glossary.md).

| Term | Meaning |
|---|---|
| **Entity** | One domain concept (here: Place) delivered as a trio of subprojects in one directory |
| **Trio** | The three subprojects: service crate, matcher crate, front-end project |
| **Entity-level spec** | This document set — source of truth for the cross-subproject contract |
| **Crate spec** | A subproject's own `spec/` — source of truth for that subproject's internals |
| **Place** | The canonical record for a geographic place per [schema.org/Place](https://schema.org/Place): names, type, address, geo, identifiers, hierarchy |
| **Service model** | The service's schema.org-shaped `Place` (`src/models/place.rs`) — what the REST API serves |
| **Matcher model** | The matcher's flat 15-field `Place` builder shape — what `MatchingEngine` scores |
| **Adapter** | `src/matching/adapter.rs` in the service — the lossy projection service model → matcher model (§5.3) |
| **Canonical algorithm** | The matcher crate's scoring — the reference the service embeds as `matcher_lib` |
| **GLN** | GS1 Global Location Number — 13 digits with check digit; exact match is a deterministic short-circuit to 1.0 |
| **PostalAddress** | Structured address: `street_address`, `address_locality`, `address_region`, `address_country`, `postal_code` |
| **GeoCoordinates** | WGS 84 decimal-degree `latitude` / `longitude` (+ optional `elevation`) |
| **Haversine** | Great-circle distance on a sphere — the geo-matching and geo-radius-search primitive |
| **Geo-radius search** | `GET /api/places/nearby?lat=&lon=&radius_km=` — find places within R km of a coordinate |
| **Hierarchy** | `contained_in_place` / `contains_place` parent–child links; cycles are rejected |
| **Gazetteer** | An authoritative geographic-name register (national mapping agency, GeoNames, OSM) — an import source, roadmap §15 |
| **Envelope** | The REST response wrapper shared by service and front-end |
| **Match** | A comparison between two places yielding a 0.00–1.00 score plus per-component breakdown |
| **Merge** | Transfers a duplicate's data onto a surviving record, soft-deletes the duplicate, writes a `Replaces` link |
| **Review queue** | Persisted candidate duplicate pairs: `Pending` / `Confirmed` / `Rejected` / `AutoMerged` |
| **Soft delete** | Retention with `is_deleted = true`; never `DELETE FROM` — the entity-wide erasure mechanism |
| **Masking** | Privacy view: phone / fax redaction + coordinate rounding to 2 dp (~1 km) |
| **SSO** | Single sign-on via the [authentication entity](../../authentication/): magic-link, RS256 JWT + JWKS |
| **Bridge test** | Service-side test (`tests/duplicate_detection.rs`) that pins both the adapter and the matcher output |
| **Drift policy** | Front-ends keep per-project copies of types/client/forms; no shared package (repo decision 2026-06-02) |
