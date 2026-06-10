# Event matcher — specification

**Crate:** `event-matcher` &nbsp;·&nbsp; **Version targeted:** `0.4.0` (place matcher — historical) &nbsp;·&nbsp; **Status:** partially superseded

> **Notice — domain change in 0.5.0.** This document was authored against the 0.4.x **place matcher** surface. From 0.5.0 the crate matches **schema.org/Event** records. Sections that remain accurate for 0.5.0 (no translation needed): §4 (text normalisation), §6.1 (string similarity primitives), §6.3 (Gaussian decay shape, reused for temporal proximity), §8 (determinism / safety), §9 (SemVer policy). Sections describing the data model (§1, §3, §5), per-field scoring weights (§7), and worked examples are **out of date** until the spec is rewritten; consult `src/`, [`README.md`](../README.md), and [`CHANGELOG.md`](../CHANGELOG.md) for the live 0.5.0 surface. High-level rename table: `Place → Event`, `PlaceBuilder → EventBuilder`, `PlaceCategory → EventCategory`, `PlaceId / PlaceIdScheme → EventId / EventIdScheme` (Eventbrite, Meetup, Ticketmaster, Wikidata, …), `latitude` / `longitude` on `Place` → `Location.latitude` / `Location.longitude` inside `Event.location`, `address` on `Place` → `Location.address`, `phone` / `email` removed, `coordinates_score` → `start_date_score` (time) + `location_score` (venue / coords), deterministic rule becomes "shared `event_id` OR same name + same `start_date` instant". `Scorer::coordinates_score(d, scale)` over Haversine is retained, plus new `Scorer::start_date_score(d, scale)` over `Scorer::seconds_between`.

This document is the living, single source of truth (SSOT) for the place-matcher surface of the crate. Every other document in the repository (`README.md`, `index.md`, `AGENTS.md`, `AGENTS/*.md`, `CHANGELOG.md`) summarises or quotes this file — none contradicts or extends it. When prose elsewhere disagrees with this file, this file wins; when this file disagrees with the code, see §9.

The keywords **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are to be interpreted in the sense of RFC 2119 / RFC 8174.

---

## Table of contents

1. [Scope](01-scope.md)
2. [Terminology](02-terminology.md)
3. [Data model](03-data-model.md)
4. [Normalisation](04-normalisation.md)
5. [Matching pipeline](05-matching-pipeline.md)
6. [Per-field scoring algorithms](06-per-field-scoring-algorithms.md)
7. [Configuration](07-configuration.md)
8. [Determinism and safety](08-determinism-and-safety.md)
9. [Public API contract (SemVer)](09-public-api-contract-semver.md)
10. [Open questions](10-open-questions.md)
11. [Worked examples](11-worked-examples.md)
12. [Glossary cross-reference](12-glossary-cross-reference.md)
13. [References](13-references.md)
