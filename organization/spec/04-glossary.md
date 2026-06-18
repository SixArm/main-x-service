## 4. Glossary

Entity-level terms. Per-subproject vocabularies: service
[spec §4](../organization-service-with-loco/spec/index.md), matcher
[spec §3](../organization-matcher-rust-crate/spec/index.md), front-end
[spec §4](../organization-front-end-with-svelte/spec/index.md).

| Term | Meaning |
|---|---|
| **Entity** | One domain concept (here: Organization) delivered as a trio of subprojects in one directory |
| **Trio** | The three subprojects: service crate, matcher crate, front-end project |
| **Entity-level spec** | This document set — source of truth for the cross-subproject contract |
| **Crate spec** | A subproject's own `spec/` — source of truth for that subproject's internals |
| **Organization** | The canonical record per schema.org/Organization: name, legal name, alternate names, identifiers, URL, sameAs, address, jurisdiction, founding date, telephone, email, keywords |
| **DTO contract** | The matcher's `Organization` type **is** the API body, the stored JSONB payload, and the matching input — one type across the trio, no adapter |
| **pid** | The public UUID of a stored organization record (route param, audit key) |
| **`data` payload** | The full `Organization` serialized as the JSONB `data` column; `name` is denormalised alongside for listing and search |
| **Deterministic identifier** | Globally unique by construction — LEI, DUNS, ISO 6523, GLN, Wikidata, ROR, ISNI, VAT. A shared value pins the match score to 1.0 (R-0) |
| **Jurisdiction-scoped identifier** | `TaxId` — unique only within a country/register; pins to 1.0 only when the jurisdiction also matches (R-1) |
| **Classification code** | `Naics` / `IsicV4` / `Sic` — describes the *sector*, never the entity; never a deterministic pin |
| **Legal-name normalisation** | Fold + strip legal-form suffix tokens (`Inc`, `Ltd`, `GmbH`, …) so `"Acme, Inc."` ≡ `"ACME"` |
| **Match** | A comparison between two organizations yielding a 0.00–1.00 score, a confidence band, an `is_match` boolean, and a per-component breakdown |
| **Confidence** | `High` ≥ 0.95, `Medium` ≥ 0.70, else `Low` — separate from the `is_match` threshold (default 0.85) |
| **Check-duplicates** | Posting a query `Organization` to find stored records that match above threshold, ranked by score |
| **Soft delete** | Retention with a `deleted_at` stamp; reads filter `deleted_at IS NULL`; never `DELETE FROM` |
| **Audit log** | Per-CRUD row in `audit_logs`: `entity_pid`, `action` (created / updated / deleted), optional `actor`, JSONB `snapshot` |
| **Event stream** | In-memory ring buffer of `OrgEvent { kind, pid, name, seq }` published on every CRUD (MVP; durable bus is roadmap) |
| **SSO** | Single sign-on via the [authentication entity](../../authentication/): magic-link, cookie sessions, short-lived PASETO v4.public cross-service tokens (see [`authentication-sessions.md`](../../agents/share/authentication-sessions.md); supersedes RS256-JWT + JWKS) |
| **Drift policy** | Front-ends keep per-project copies of types/client/forms; no shared package (repo decision 2026-06-02) |
