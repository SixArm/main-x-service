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

## Stitched-journey timeline (delivered 2026-08-24)

`GET /api/stays/{pid}/time-analysis` serves the timeline contract that
`care-pathway-service` follows across a `continues_as` link
([`agents/share/time-based-analysis.md`](../../agents/share/time-based-analysis.md),
[`cross-service-linking.md`](../../agents/share/cross-service-linking.md)
§9). It lets a journey that begins on a care pathway and continues into
an inpatient stay be measured end to end, instead of stopping at the
service boundary.

The contract is four numbers — clock bounds, elapsed span, value-adding
time — deliberately too small to couple this service's domain model to
anybody else's. What matters is what fills the fourth:

**A green Red2Green day is the value-adding time.** Time-based analysis
asks what share of an episode was *the work*, and Red2Green already
answers exactly that in the NHS's own vocabulary: a green day moves the
patient toward discharge, a red day does not. Deriving it any other way
would have meant this service making a clinical judgement it is not
entitled to make.

**Unclassified days count as non-value-adding**, matching the consuming
service's denominator rule — unrecorded time counts against you, because
the alternative rewards recording less. The figure is a **floor**, and
the response carries `coverage` and `confidence`: an unclassified stay
and a genuinely red one both report little value-adding time, and only
the confidence tells them apart. The distinction is the whole point —
one calls for filling in the board, the other for fixing the delay.

**A coverage ceiling.** `red-green` classifies *today* and takes no
`day`, so a stay admitted before the board was in use can never be fully
classified retrospectively. Coverage is capped by when classification
started, not by ward diligence. Adding a `day` to the classification
payload would lift the ceiling and is the obvious follow-up; it is not
done here because backfilling a day is a data-quality decision with its
own audit story.

The endpoint is a **sensitive read** ([auth.md](auth.md)): record-level
ABAC, audited, so a caller assembling a cross-service journey leaves the
same trail as one opening the record. The `mask` obligation is not
applied — the response carries durations and no identifiers, so there is
nothing in it to redact. The far service (care-pathway) forwards the
**caller's** credential rather than a service identity, so this
service's own policy applies to the real caller.

