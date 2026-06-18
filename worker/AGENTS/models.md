# Domain Models — Worker entity

Three model surfaces exist; know which one you are editing.

## 1. Service `Worker` (system-of-record shape)

Rich, FHIR-shaped, vector sub-records: `HumanName`, `Identifier`
(MRN / SSN / DL / NPI / PPN / TAX / ODS / Other), `IdentityDocument`,
`EmergencyContact`, `Address`, `ContactPoint`, `WorkerLink`, plus
merge / review-queue / consent / organization support types and the
SeaORM table entities.

→ Full field tables:
[service `AGENTS/models.md`](../worker-service-with-loco/AGENTS/models.md);
normative shape: [service spec §5](../worker-service-with-loco/spec/05-domain-model.md).

## 2. Matcher `Worker` (comparison shape)

Flat, builder-shaped: explicit `family_name` / `given_name` /
`middle_name`, `phone` / `mobile` / `email`, one `address` +
`previous_addresses`, `passport_books: Vec<PassportBook>`, and one
field per national-identifier scheme (42 schemes, scheme-local —
never cross-matched). `#[non_exhaustive]`; construct via
`Worker::builder()`.

→ Normative shape:
[matcher spec §8](../worker-matcher-rust-crate/spec/08-domain-model.md);
public API: [matcher spec §11](../worker-matcher-rust-crate/spec/11-public-api-specification.md);
scheme table: [matcher `AGENTS/national-person-identifiers.md`](../worker-matcher-rust-crate/AGENTS/national-person-identifiers.md).

## 3. Front-end TypeScript types (wire mirror)

`src/lib/api/types.ts` mirrors the service's JSON wire format
(`Worker`, `HumanName`, `MatchResult`, …). When a service field
changes, this file changes in the same effort — see
[front-end `AGENTS.md`](../worker-front-end-with-svelte/AGENTS.md).

## The adapter between shapes 1 and 2

`to_matcher_worker()` in
[`src/matching/adapter.rs`](../worker-service-with-loco/src/matching/adapter.rs)
is the only sanctioned projection from the service shape to the
matcher shape. It is lossy (service-only fields dropped) but
well-defined (routing table inline in the file), and pinned by
[`tests/duplicate_detection.rs`](../worker-service-with-loco/tests/duplicate_detection.rs).

**The routing rules are the entity-level contract** — specified in
[entity spec §5.3](../spec/05-domain-model.md). Change them only with
a three-part PR that touches the entity spec, the adapter, and the
bridge tests together.

## Invariants to preserve

- Identifiers are scheme-local; an unmapped service identifier type
  (e.g. ODS) falls through rather than being routed to a wrong slot.
- Only the service persists; matcher and front-end models are
  in-memory projections.
- Soft delete only — `active: false`, never row deletion.
