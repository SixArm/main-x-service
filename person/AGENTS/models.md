# Domain Model — Entity-Level Orientation

One canonical model, three representations. Normative wording:
entity [spec §5](../spec/05-domain-model.md). This page is the map,
not the territory — field tables live in the per-crate docs.

## The three representations

| Representation | Shape | Where defined | Reference |
|---|---|---|---|
| Service `Person` (canonical) | FHIR-shaped: `HumanName`, `Vec<Identifier>` with system URIs, `Vec<Address>`, `Vec<ContactPoint>`, `Vec<IdentityDocument>`, emergency contacts, links, soft-delete flags | `src/models/person.rs` | [service AGENTS/models.md](../person-service-with-loco/AGENTS/models.md) — full field tables for `Person`, `HumanName`, `Identifier`, `IdentityDocument`, `EmergencyContact`, `MergeRecord`, `ReviewQueueItem`, `Consent`, … |
| Matcher `Person` (flat) | Builder shape: `family_name` / `given_name` / `date_of_birth` / `address` / `phone` / `email`, one field per national-identifier scheme (42), `passport_books` | matcher `src/models.rs` | [matcher spec §8](../person-matcher-rust-crate/spec/08-domain-model.md), [national-person-identifiers.md](../person-matcher-rust-crate/AGENTS/national-person-identifiers.md) |
| Front-end TypeScript types | Mirror of the service wire format | `src/lib/api/types.ts` | [front-end AGENTS.md](../person-front-end-with-svelte/AGENTS.md) ("what lives where") |

## The adapter (service → matcher)

`src/matching/adapter.rs` exposes
`to_matcher_person(&service::Person) -> person_matcher::Person` — a
**lossy, one-way projection**. Headline routing: names flatten,
first address wins (`state`→`county`, `postal_code`→`postcode`),
first telecom per system → `phone`/`mobile`/`email`, identifiers route
to scheme slots by `system` URI, `tax_id` defaults to `us_ssn`,
passports → `passport_books`. Registry-only fields are dropped.

- Normative rule list: entity [spec §5.3](../spec/05-domain-model.md)
  and service [spec §6.2](../person-service-with-loco/spec/06-functional-requirements.md).
- Pinned by: [`tests/duplicate_detection.rs`](../person-service-with-loco/tests/duplicate_detection.rs)
  (14 bridge tests).
- Changing a routing rule = seam change → entity spec §5.3 edit +
  bridge test in the same PR.

## Shared invariants (all three must uphold)

- `name.family` non-empty; no future `birth_date`.
- Identifiers unique per `(person_id, identifier_type, system, value)`
  and **scheme-local** — never cross-match schemes.
- Soft delete only (`active = false`), end to end.
- Scores in `[0.00, 1.00]`, always with a per-component breakdown.

Full list: entity [spec §5.5](../spec/05-domain-model.md).

## When a field changes

1. Service model changes first (its spec §5 + `AGENTS/models.md`).
2. If the field feeds matching → adapter + entity spec §5.3 + bridge
   test.
3. Front-end `types.ts` updated to match the wire format (do not let
   it drift).
