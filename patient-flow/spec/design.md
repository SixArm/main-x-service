# Design decisions

Numbered, stable; tasks trace to these. Rationale lives with the
decision so it is not re-litigated.

## PF-D1 — Consumer app, not an entity registry

Patient Flow follows the **case-folder** consumer-application shape
(cross-cutting `spec/` + service edition + front-end edition), not
the matcher-backed entity-trio shape. There is no matcher: two stays
are never "the same" record to deduplicate; identity questions
belong to person-service. (→ PF-R1…R15 all)

## PF-D2 — Operational state is owned; identities are referenced

Unlike case-folder (a pure aggregator owning no tables), Patient
Flow **owns** wards/bays/beds/stays/requests/flags in its own
PostgreSQL schema, because bed state and stay lifecycle exist
nowhere upstream. Identities remain EntityRef URNs; the only cached
personal field is a refreshable, maskable display name. (→ PF-R1,
PF-R4, PF-R9; [scope.md](scope.md) boundary table)

## PF-D3 — Normalized schema, not DTO-as-JSONB

The loco entity services store the matcher DTO as JSONB; Patient
Flow instead uses proper relational tables with FKs, enums, and
state columns, because its value is **constraints and transitions**
(one occupant per bed, legal state changes) which the database
should help enforce. JSONB only for leaf lists (`alerts`,
`requirements`, `delay_reasons`). (→ PF-R2, PF-R6)

## PF-D4 — Pure `flow/` core

Bed state machine, allocation eligibility + ranking, Red2Green
rules, and the DTOC clock are pure functions in `src/flow/`,
DB-free and exhaustively unit-tested — the same "reference logic as
a pure library" posture the matcher crates set. Controllers stay
thin. (→ PF-R2, PF-R3, PF-R6, PF-R7)

## PF-D5 — ADT-shaped verbs, no ADT transport in v1

The API verbs are admit / transfer / discharge so a later HL7 ADT
adapter is a mapping, not a redesign. No HL7 listener, no FHIR
surface in v1. (→ PF-R4–R6; [integrations.md](integrations.md))

## PF-D6 — Whiteboards are derived reads

Whiteboard, at-a-glance, locate, and capacity are queries over the
operational tables with an `as_of` stamp — never a second store.
Polling with ETag in v1; SSE push is roadmap. (→ PF-R8–R10)

## PF-D7 — Allocation advises, the operator decides

The allocator returns ranked eligible beds; it never auto-places.
Sex-segregation and ward-fit rules are overridable only with a
recorded reason (audited governance events). Side-room conservation
is a ranking concern, not a hard rule. (→ PF-R3)

## PF-D8 — Virtual wards are wards

`kind = virtual` + virtual slot beds; same whiteboard, same
capacity arithmetic, no cleaning cycle. One mechanism, no parallel
feature. (→ PF-R12)

## PF-D9 — Transactional integrity

State transition + audit row + event outbox commit in one
transaction; placement paths lock bed rows `FOR UPDATE`. Family
security invariant 8 applied to flow. (→ PF-R2, PF-R13)

## PF-D10 — Family auth stack unchanged

Offline PASETO + shared ABAC engine + `PATIENT_FLOW_REQUIRE_AUTH`
default-off gate + `mask` obligation for patient names on
board/locate reads. Ward scoping via `resource.ward` record-level
checks, the case-service pattern. (→ PF-R14; [auth.md](auth.md))

## PF-D11 — Stub-first upstream clients

Stub-first, `http`/`stub` mode selected by
`PATIENT_FLOW_UPSTREAM_MODE` (case-folder precedent); upstream reads
are best-effort (a display-name lookup) and never block writes. **As
landed** (PF-T3), this is one generic `EntityRef`-keyed resolver in
`src/clients.rs` — not a per-upstream-service trait/module split as
originally sketched here — because every upstream lookup does the
same thing (resolve a display name from a `GET
{PATIENT_FLOW_<TYPE>_SERVICE_URL}/api/<type>s/{id}`), so a trait per
service would have four near-identical impls. (→ PF-R15)

## PF-D12 — Loco-idiomatic layout

`src/controllers/` shape (organization/case template), crate-root
`migration/`, family fixtures (`#![forbid(unsafe_code)]`, thiserror,
OTLP, OpenAPI, `Accepts-version`, Podman). (→ PF-R15)
