# Design

> Part of the [Case Tracking specification](index.md). This is the
> **how** — the system-level decisions that satisfy
> [requirements.md](requirements.md). Edition-internal design lives in
> each subproject's `spec/design.md` / `spec/architecture.md`.

## D-1 — Aggregator, not owner (NFR-4, NFR-3)

The tracker owns **no domain tables**. Every entity lives in one of five
upstream Main-X-Services (see [domain-model.md](domain-model.md)). The
tracker proxies them and writes only folders (Thing) and move events
(Event). _Rationale:_ keeps the regulatory surface minimal
([regulatory.md](regulatory.md)) and avoids duplicating systems of record.

## D-2 — Snapshot-on-write (NFR-2, NFR-3, AC-1, AC-3)

When the tracker writes a folder or a move event, it copies the patient
name, NHS Number, cabinet path, worker name, and worker role from
whatever the upstream authoritatively returned **at that moment**.
Snapshots are never refreshed. _Rationale:_ the audit trail and patient
lookups keep working through upstream renames, deletes, and outages.

## D-3 — Derived status (FR-9, AC-4)

Folder status is computed, not stored: latest move event with a
destination cabinet → `in-cabinet`; none → `in-transit`; with no move
history, fall back to the folder's current cabinet pointer. _Rationale:_
one source of truth (the move log) for "where is it now".

## D-4 — API-first contract (NFR-6)

The Loco edition is the contract. New capability is specified there (and
at the root if cross-cutting) before any client consumes it. Responses
go through typed structs; clients map a documented snake→camel shape.
_Rationale:_ schema-stable output; the UI can never outrun the API.

## D-5 — Stub-mode upstreams (NFR-7, AC-5)

A single env flag (`USE_UPSTREAM_STUBS=1`) swaps every upstream for an
in-process stub seeded with demo data, identical in shape to the real
services. _Rationale:_ the entire system — and the front-end e2e suite —
runs and is tested without standing up five external services.

## D-6 — Two-sided NHS validation (NFR-1, AC-2)

The identical Modulus 11 validator runs client-side (fast UX, blocks the
form) and server-side (authoritative, returns `422` with field errors).
_Rationale:_ responsive forms without trusting the client.

## D-7 — Opaque-UUID coupling (NFR-3)

Cross-service references are opaque UUIDs; no service enforces
referential integrity against another. Reconciliation is via snapshots
only. _Rationale:_ services evolve and deploy independently.

## D-8 — Resilience policy

Default to **fail loud** (`503`) when an upstream is unreachable so
callers can retry. Two **intentional soft-fail** exceptions return
partial data instead: aggregate stats (zeros for the unavailable slice)
and patient lookup (snapshot fallback). _Rationale:_ a dashboard partial
render and an audit-survives-outage lookup beat a hard failure.

## D-9 — Magic-link auth with stateless signed tokens (FR-10..12, NFR-8, NFR-9)

Authentication is passwordless: a short-lived signed **magic** JWT is
emailed; clicking the link exchanges it (via `verify`) for a longer-lived
signed **session** JWT delivered as an HttpOnly cookie. No auth tables —
identity comes from a configured allowlist; `aud` separates magic vs
session tokens. A guard requires a valid session on `/api/*` except
`/api/auth/*`. _Rationale:_ real auth that preserves the no-local-tables
invariant (D-1) and keeps the token out of JavaScript. _Trade-off:_
stateless tokens can't be revoked before expiry without a denylist;
acceptable for the demo, flagged for production. Full detail:
[auth.md](auth.md).

## D-10 — Click-through views are derived from the move log (FR-13..15)

Worker→folders, place→presence-history, and event→detail are **read-only
projections** computed from the move-event audit log plus the folder and
place lists — no new stored data, preserving the aggregator decision
([D-1](design.md)). A worker's folders come from events filtered by
`worker_id`; a cabinet's presence intervals come from pairing `to_cabinet`
(enter) and `from_cabinet` (leave) events per folder; an event is fetched
by id. Rooms/buildings aggregate the histories of contained cabinets.
_Rationale:_ the audit log already records everything these views need;
deriving keeps a single source of truth and avoids write paths. _Trade-off:_
the worker/place/event endpoints scan the full event list (acceptable at
demo scale; the existing `/api/moves` and `/api/stats` already do this).

## D-11 — Volumes are movable Thing bundles, not a new service (FR-16..19)

A **volume** is stored in the Main Thing Service as a `Thing` with
`thing_type = "Volume"` (a sibling of the `"CaseFile"` folder Things), so
no new upstream service or local table is introduced ([D-1](design.md)).
Membership is a single optional pointer on the folder (`volume_id` +
`volume_title` snapshot, carried as a Thing keyword); a volume's members
are the folders whose `volume_id` matches. A volume carries its own
`cabinet_id`. _Moving a volume_ is a controller-level fan-out: it updates
the volume's cabinet and, for each member folder, records a move event and
updates the folder's cabinet — reusing the existing per-folder move
machinery so the audit trail and folder-status derivation are unchanged.
_Rationale:_ supports real-world bundle moves while keeping the per-folder
audit log authoritative. _Trade-off:_ moving a large volume writes N
events; acceptable at demo scale and semantically correct (each folder
genuinely moved).

## D-12 — iFIT software features over the existing log + APIs (FR-20..23)

The iFIT-inspired features are layered on what already exists, with no new
storage. **Geofence alerts** are derived in a controller: each move whose
from/to cabinets resolve (via the place hierarchy) to different buildings
is a boundary crossing — exposed at `GET /api/alerts`, mirroring how
`/api/places/{id}/history` derives presence ([D-10](design.md)). The
derivation itself is a pure function (`detect_geofence_breaches`) so the
boundary-crossing rule — skip an in-transit/created-in-place endpoint, skip
an unresolvable cabinet, suppress same-building moves — is unit-tested in
isolation (`cargo test --lib`); the handler only supplies upstream data and
serialises the result.
**Reports** are composed client-side from the existing `stats`, `moves`,
`places`, `volumes`, and `workers` endpoints (API-first, [D-4](design.md)).
**Scan-to-move** is pure front-end: an NHS/id field reuses
`folders.list({nhsNumber})` and routes to the move form. **Role display**
reads the auth identity's role ([D-9](design.md)); audit-by-worker reuses
the move log. _Rationale:_ delivers the buildable, software part of iFIT
without inventing hardware or new services. _Out of scope:_ RFID/BLE/GPS
sensing and GIS mapping — not implementable in a browser demo.

## Requirement → design trace

| Requirement              | Satisfied by      |
| ------------------------ | ----------------- |
| FR-1, FR-9               | D-2, D-3          |
| FR-2                     | D-4, D-6          |
| FR-3, FR-4, FR-5, FR-8   | D-1, D-4          |
| FR-7                     | D-2, D-8          |
| NFR-1                    | D-6               |
| NFR-2, NFR-3             | D-2, D-7, D-8     |
| NFR-4                    | D-1               |
| NFR-6                    | D-4               |
| NFR-7                    | D-5               |
| FR-10, FR-11, FR-12      | D-9               |
| NFR-8, NFR-9             | D-9               |
| FR-13, FR-14, FR-15      | D-10              |
| FR-16, FR-17, FR-18, FR-19 | D-11            |
| FR-20, FR-21, FR-22, FR-23 | D-12            |
