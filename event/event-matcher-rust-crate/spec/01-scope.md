## 1. Scope

### 1.1 In scope

- Pairwise matching of two `Event` records (modelled on [schema.org/Event](https://schema.org/Event)) to decide whether they refer to the same event — festivals, conferences, concerts, sports fixtures, screenings, hackathons, meetups, theatre runs, and other instances of `schema:Event`.
- A **deterministic** strategy returning `bool`.
- A **probabilistic** strategy returning a renormalised score in `[0.0, 1.0]` with a per-field `MatchBreakdown` for explainability.
- Batch entry points: scoring and ranking a single query against a slice of candidates.
- Supporting text-normalisation primitives (names, postcodes, phones, emails, addresses, phonetic codes) and ISO 8601 date-time parsing to Unix seconds.
- Temporal primitives (absolute second difference between two ISO 8601 instants, Gaussian-decay similarity).
- Geographic primitives (Haversine distance on a sphere, Gaussian-decay similarity), used by the `location` sub-score.
- Configurable weights, threshold, time / distance scales, and similarity algorithm via `MatchConfig`.
- Serde round-trip of every public data type.

### 1.2 Out of scope

- **Calendar arithmetic as a service** — `Normalizer::parse_iso8601_unix_seconds` is a total, dependency-free parser for ISO 8601 / RFC 3339 date and date-time strings, not a general date library. Recurrence rules (RFC 5545 `RRULE`), durations, and named time zones are not interpreted.
- **Geocoding** (address → coordinates) and **reverse geocoding** (coordinates → address).
- **Routing** or network distance — only great-circle (Haversine) distance is provided.
- **Address parsing as a service**: `Normalizer::parse_address_line` is a best-effort structural decomposition for matching purposes, not a postal-reference lookup.
- **Full-text search** and **event suggestions** — the crate scores known pairs in memory.
- **Persistent storage and indexing** — the crate never reads or writes external state.
- **Candidate blocking** — pre-filtering large candidate sets is a consumer concern.
- **Machine learning** — the algorithm is rule-based; weights are tuneable but the structure is fixed.
- **Sub-event / super-event graph resolution** — `super_event_id` is carried as data only; the matcher does not traverse event hierarchies.
- **Locale-aware street-type vocabularies** — only English abbreviations are expanded.

### 1.3 Audience

Data engineers, event-aggregation and ticketing-platform integrators, and deduplication-pipeline authors who need an explainable, deterministic library for joining or de-duplicating event records drawn from heterogeneous sources (Eventbrite, Meetup, Ticketmaster, iCalendar feeds, …).

---
