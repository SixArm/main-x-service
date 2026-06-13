# Requirements

> Part of the [Case Tracking specification](index.md). This is the
> **what + why** that drives delivery. Each requirement traces to a
> design decision in [design.md](design.md) and a delivery item in
> [tasks.md](tasks.md). Edition specs implement these; they do not
> add new product requirements without a row here first.

## Functional requirements

| ID    | Requirement                                                                                          | Use case |
| ----- | --------------------------------------------------------------------------------------------------- | -------- |
| FR-1  | A user can record a folder **move** to a destination cabinet, attributing it to a worker.            | UC-1     |
| FR-2  | A user can **register a new folder** for a patient (creating the patient upstream if needed).        | UC-2     |
| FR-3  | A user can **find a folder** by free-text query (title, patient, NHS Number).                        | UC-3     |
| FR-4  | A user can **audit folder history** — the full move log, filterable by free text.                    | UC-4     |
| FR-5  | A user can **inspect cabinet utilisation** and dashboard KPIs (counts, in-transit, recent moves).    | UC-5     |
| FR-6  | A user can **register a cabinet**, and the building/room it belongs to.                              | UC-6, UC-7 |
| FR-7  | A user can **view a patient's folders** by NHS Number, with snapshot fallback when upstream is down. | UC-8     |
| FR-8  | A user can **look up / search workers** to attribute a move.                                         | UC-9     |
| FR-9  | A folder's **current location and status** (`in-cabinet`/`in-transit`) are always derivable.          | UC-1, UC-5 |
| FR-10 | A user can **sign in via an email magic link** and the app keeps them signed in for a session.          | UC-A1      |
| FR-11 | A user can **sign out**, ending the session.                                                           | UC-A2      |
| FR-12 | The domain API and UI are **only usable while signed in** (auth endpoints + health excepted).          | UC-A1      |
| FR-13 | A user can **click a worker** to see the folders they've moved AND all of their patients' folders.       | UC-W1      |
| FR-14 | A user can **click a place** (cabinet/room/building) to see its **folder presence history** (what was in it, when). | UC-P1 |
| FR-15 | A user can **click an event** (move) to see the folder involved and jump to that patient's other folders. | UC-E1      |
| FR-16 | A user can **create and rename a volume** — a named bundle of one patient's folders.                     | UC-V1      |
| FR-17 | A user can **assign a folder to / remove it from a volume** (folder and volume must share a patient).    | UC-V2      |
| FR-18 | A user can **view a volume** — its member folders, current location, and move history.                   | UC-V3      |
| FR-19 | A user can **move a whole volume** to a cabinet, relocating every member folder in one action.           | UC-V4      |
| FR-20 | A user can see **geofence alerts** — moves that took a folder across a building boundary.                 | UC-I1      |
| FR-21 | A user can open a **reports** view: cabinet utilisation, in-transit, throughput, per-worker activity.    | UC-I2      |
| FR-22 | A user can **scan** an NHS Number / folder id to jump straight to that folder's move form (Scan4Safety). | UC-I3      |
| FR-23 | The UI shows the **signed-in user's role**, and the audit log is filterable by worker.                   | UC-I4      |

## User stories

- **As records staff**, I want to record where a folder went so the next
  person can find it — _so that_ care is not delayed hunting for paper.
- **As a clinician**, I want to confirm a folder's whereabouts before an
  appointment — _so that_ I have the notes when the patient arrives.
- **As a records manager**, I want cabinet utilisation and a complete
  move log — _so that_ I can plan capacity and prove chain of custody.
- **As an integrator**, I want a clean JSON API — _so that_ I can build
  other front-ends without re-implementing the domain.

## Non-functional requirements

| ID     | Requirement                                                                                   |
| ------ | --------------------------------------------------------------------------------------------- |
| NFR-1  | **NHS Number validity** is enforced by Modulus 11 on both client and server.                  |
| NFR-2  | **Audit immutability** — move events are append-only; referenced labels are snapshotted.      |
| NFR-3  | **Resilience** — the audit trail and patient lookups survive upstream outages via snapshots.  |
| NFR-4  | **No domain data owned locally** — all entities live in the five upstream services.           |
| NFR-5  | **Accessibility** — the UI targets WCAG 2.2 AA (single h1, skip link, labelled fields, etc.). |
| NFR-6  | **Contract stability** — responses go through typed structs; clients map a documented shape.  |
| NFR-7  | **Testability** — a stub-mode upstream lets the whole system run + be tested without 5 services. |
| NFR-8  | **Passwordless auth** uses stateless signed tokens — no auth tables (preserves NFR-4).            |
| NFR-9  | **Session secrecy** — the session token is an HttpOnly cookie, never readable by JavaScript.      |

## Acceptance criteria (cross-cutting)

- **AC-1 (FR-1, NFR-2):** recording a move appends a `MoveEvent` with
  snapshotted patient/cabinet/worker labels and updates the folder's
  current cabinet + status; the prior location is preserved in history.
- **AC-2 (FR-2, NFR-1):** registering a folder with an invalid NHS
  Number is rejected with a field-level error on both client and server.
- **AC-3 (FR-7, NFR-3):** a patient lookup returns folders from
  snapshots even when the Main Patient Service is unreachable, flagged as
  an unmatched/fallback result.
- **AC-4 (FR-9):** a folder with a destination cabinet reads
  `in-cabinet`; one moved out with no destination reads `in-transit`.
- **AC-5 (NFR-7):** the full suite runs against stub-mode upstreams with
  no external services.
- **AC-6 (FR-10, FR-12, NFR-8):** an unauthenticated request to a domain
  endpoint is rejected (`401`); after `request`→`verify` with a valid
  magic token, the same request succeeds carrying the session.
- **AC-7 (FR-10, NFR-9):** `verify` sets an HttpOnly session cookie;
  `GET /api/auth/me` returns the signed-in user; `logout` clears it.

### Use cases (auth)

| UC    | Trigger                  | Outcome                                                            |
| ----- | ------------------------ | ----------------------------------------------------------------- |
| UC-A1 | Sign in                  | Enter email → receive magic link → click → session established.   |
| UC-A2 | Sign out                 | Session cookie cleared; protected routes redirect back to sign-in. |
| UC-W1 | Inspect a worker         | Click a worker → folders they moved + all their patients' folders. |
| UC-P1 | Inspect a place          | Click a cabinet/room/building → folder presence history (in/out timeline). |
| UC-E1 | Inspect a move event     | Click a move → the folder involved, the worker, places, and the patient's other folders. |
| UC-V1 | Create / rename a volume | Make a named bundle for a patient; edit its title. |
| UC-V2 | Curate a volume          | Add a folder to a volume, or remove one. |
| UC-V3 | Inspect a volume         | Click a volume → its folders, location, and move history. |
| UC-V4 | Move a volume            | Relocate a volume → every member folder moves with it, one audit event each. |
| UC-I1 | Review geofence alerts   | See case notes that crossed a building boundary (an iFIT-style geofence breach). |
| UC-I2 | Run reports              | Open the reports view for utilisation, in-transit, throughput, per-worker activity. |
| UC-I3 | Scan to move             | Scan/enter an NHS Number or folder id and jump to that folder's move form. |
| UC-I4 | See role + audit by worker | The chrome shows the signed-in role; the audit log filters by worker. |

### iFIT-inspired software features (FR-20..23)

Derived from [the iFIT overview](../case-folder-front-end-with-svelte/spec/ifit.md).
**Hardware/infrastructure (RFID, BLE, GPS, fixed sensors, handheld scanners,
GIS mapping) is out of scope** for this web app; these are the
software-equivalent capabilities.

- **AC-I1 (FR-20):** a move whose origin cabinet and destination cabinet
  resolve to **different buildings** is reported as a geofence alert,
  newest first, derived from the move log + place hierarchy.
- **AC-I3 (FR-22):** scanning a valid NHS Number routes to the matching
  folder's move form (or a disambiguation list); an unknown value shows a
  not-found message — no hardware scanner required (keyboard/wedge input).

### Volumes (FR-16..19)

- **AC-V1 (FR-17):** assigning a folder whose `patientId` differs from the
  volume's is rejected (`422`).
- **AC-V2 (FR-19):** moving a volume to a cabinet updates the volume's
  location, sets every member folder's cabinet to that cabinet, and appends
  one move event per member folder.
- **AC-V3 (FR-18):** a volume's detail returns its member folders (derived
  from folders whose `volumeId` matches) and the merged move history of those
  folders.

### Click-through navigation (FR-13..15)

These are read-only **derived views** over the existing move-event audit
log; no new domain data is stored. Two of the five click-throughs
already exist and are unchanged: **folder → its move history**
(`/folders/{id}`) and **patient → their folders** (`/patients/{nhs}`).

- **AC-W1 (FR-13):** a worker's detail returns (a) the distinct folders
  that worker has moved, and (b) every folder belonging to any patient
  that worker has moved a folder for — derived from move events where
  `worker_id` matches.
- **AC-P1 (FR-14):** a cabinet's presence history pairs each `to_cabinet`
  (entered) with the next `from_cabinet` (left) per folder; a still-resident
  folder has an open interval. A room/building aggregates the histories of
  the cabinets it contains.
- **AC-E1 (FR-15):** a single move event is addressable by id and surfaces
  its one folder plus links to that folder and the patient's other folders.

## Out-of-scope (explicit non-requirements)

Authentication/RBAC, clinical content, local persistence, real-time
push, multi-tenancy — see [scope.md](scope.md). These are production
gates tracked in [regulatory.md](regulatory.md) and [roadmap.md](roadmap.md),
not current requirements.
