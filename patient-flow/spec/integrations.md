# Integrations

Patient Flow is a consumer of the Main X Index family. It holds
**EntityRef URNs** and never duplicates upstream records.

## Upstream services

| Service | Used for | How |
|---|---|---|
| [person-service](../../person/person-service-with-loco/) | patient identity | `person:<pid>` on Stay, BedRequest; resolve display name + recorded sex (allocation rule input); locate is keyed by this URN |
| [worker-service](../../worker/worker-service-with-loco/) | staff identity | `worker:<pid>` for named nurse, consultant, transfer/clean actors |
| [place-service](../../place/place-service-with-loco/) | physical sites | optional `place:<pid>` on Site/Ward |
| [organization-service](../../organization/organization-service-with-loco/) | the trust | `organization:<pid>` on Site |
| [authentication-service](../../authentication/authentication-service-with-loco/) | SSO + tokens | offline PASETO v4.public verification via `authentication-verifier`; ABAC `attrs` |
| [link-graph-service](../../link/link-graph-service-with-loco/) | (roadmap) | Patient Flow events could feed a future `located_at` read model; **not** a v1 edge kind |

Client modules follow the case-folder precedent: one module per
upstream service with an `http` implementation and a `stub`
implementation behind a trait, selected by config — so the service
runs and tests with zero upstream dependencies (stub mode) and wires
to real services in deployment.

Upstream lookups are **read-only, cached, and non-blocking for
writes**: a stay admit validates the URN shape locally and resolves
the display name best-effort; an unreachable person-service degrades
the whiteboard to showing the URN, never blocks an admission.

## EntityRef

The URN format and `entity_type → service` map come from the shared
[`entity-ref`](../../link/entity-ref-rust-crate/) crate
([cross-service-linking.md](../../agents/share/cross-service-linking.md) §3).
Patient Flow depends on the crate directly (`entity-ref = { path =
"../../link/entity-ref-rust-crate" }` in `Cargo.toml`), using
`EntityRef`/`EntityType` in `src/validation.rs` (URN shape checks)
and `src/clients.rs` (display-name resolution) — the family's earlier
"copy per project" plan never happened in practice; see
[cross-service-linking.md](../../agents/share/cross-service-linking.md)
§12 for the family-wide accounting (eight real dependents, patient-flow
among them).

## Cross-service links — deliberately not used for flow state

A stay's bed is **operational state**, changing many times a day; the
`entity_links` machinery is for durable identity/affiliation edges.
Patient Flow therefore does **not** write `entity_links` rows for
bed occupancy. If a future need arises to expose "person is currently
an inpatient at site" as a graph edge, that is a roadmap item with
its own governance discussion (it is sensitive — see
[auth.md](auth.md)).

## ADT posture (deferred)

The domain verbs deliberately mirror HL7 v2 ADT: admit ≈ A01,
transfer ≈ A02, discharge ≈ A03. A future `adt-adapter` can consume
an ADT feed and drive the same API, making Patient Flow deployable
beside an existing PAS without dual keying. No HL7 listener, parser,
or FHIR `Encounter`/`Location` surface ships in v1
([roadmap.md](roadmap.md)).

## Event emission

Patient Flow emits family-standard event envelopes for every
mutation ([audit.md](audit.md)), using the same
`PATIENT_FLOW_EVENT_TRANSPORT=memory|outbox` seam as the entity
services, so its stream can join the durable bus when Phase 3
(Fluvio relay) lands family-wide.
