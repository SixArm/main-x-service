## 6. Functional Requirements

Each requirement names its owning subproject. Crate-internal detail
lives in the owning crate's spec; this section is the entity-level
contract.

### 6.1 Delivered

- **FR-1 — Organization CRUD** *(service)*. Create / read / update /
  soft-delete organization records whose body is the canonical
  `Organization` DTO. Create and replace require a non-blank `name`
  (`422` otherwise); soft delete stamps `deleted_at`. Endpoints: §9.
- **FR-2 — Explicit matching** *(service + matcher)*.
  `POST /api/organizations/match` ranks a `{query, candidates}` set
  through `MatchingEngine::rank` with default config — no
  persistence, usable as a pure scoring API.
- **FR-3 — Duplicate check** *(service + matcher)*.
  `POST /api/organizations/check-duplicates` scores a query
  `Organization` against stored active records (current scan cap
  `CHECK_DUPLICATES_SCAN_CAP` = 1 000, a named constant) and returns
  the ones with `is_match == true` as
  `{pid, name, score, confidence, is_match}`, ranked by score
  descending. The cap is a known scale cliff: when the scan returns
  exactly the cap the handler emits a `WARN` log so the truncation is
  observable rather than a silent miss of candidates beyond the cap.
  Lifting the cap via blocking / candidate pre-selection is task T-7.
- **FR-4 — Deterministic short-circuits** *(matcher)*. Score pinned
  to 1.0 on: R-0 shared value on a deterministic scheme (LEI, DUNS,
  ISO 6523, GLN, Wikidata, ROR, ISNI, VAT); R-1 shared non-empty
  `jurisdiction` AND `TaxId` value; R-2 case-folded `same_as` URL
  overlap. Classification codes (`Naics` / `IsicV4` / `Sic`) and
  `Custom` MUST NOT pin.
- **FR-5 — Probabilistic scoring** *(matcher)*. Renormalised weighted
  average over the components present on both sides (defaults):

  | Component | Weight | Algorithm |
  |---|---:|---|
  | Name | 0.35 | Legal-suffix-aware Jaro-Winkler over all name keys + Soundex +0.05 bonus capped at 0.95 |
  | Address | 0.20 | Field-by-field Jaro-Winkler (street 0.30, locality 0.25, postal 0.20, region 0.15, country 0.10) |
  | URL / domain | 0.15 | Registered-domain equality → 1.0, else Jaro-Winkler |
  | Jurisdiction | 0.10 | Case-folded country exact (1.0 / 0.0) |
  | Founding date | 0.10 | Same year 1.0, ±1 yr 0.5, else 0.0 |
  | Keywords | 0.10 | Jaccard on `fold_set` |

  Threshold 0.85 (presets: `strict` 0.95, `lenient` 0.70);
  confidence `High` ≥ 0.95, `Medium` ≥ 0.70, else `Low`. Absent
  fields never drag the score down.
- **FR-6 — Name search** *(service)*.
  `GET /api/organizations/search?q=` — case-insensitive substring
  match (PostgreSQL `ILIKE`) on the denormalised `name`, active rows
  only, capped 50. Blank `q` → `400`.
- **FR-7 — OpenAPI / Swagger** *(service)*. Hand-written OpenAPI 3
  document at `/api-docs/openapi.json` (the matcher crate is
  dependency-light, so no `utoipa` derive); Swagger UI at
  `/swagger-ui`.
- **FR-8 — Audit log** *(service)*. Every create / update / delete
  writes an `audit_logs` row (`entity_pid`, `action`, optional
  `actor`, JSONB `snapshot`), best-effort (an audit failure never
  fails the request). Query endpoints: recent system-wide and
  per-record (§9).
- **FR-9 — Event streaming** *(service)*. Every CRUD publishes an
  `OrgEvent { kind: created|updated|deleted, pid, name, seq }` to an
  in-memory ring buffer (capacity 1 000);
  `GET /api/organizations/events/recent` returns the newest 100.
  Durable bus is roadmap (§15).
- **FR-10 — Operator UI** *(front-end)*. Four routes: list (`/`),
  create (`/new`), detail + delete + check-duplicates (`/[pid]`),
  edit (`/[pid]/edit`). Check-duplicates posts the current record and
  lists matches (name, score, confidence) excluding the record
  itself.

### 6.2 Deferred (explicitly not yet implemented)

Tracked in §13 and §15; mature-entity parity is the goal.

| Feature | Status | Owner-to-be |
|---|---|---|
| Privacy: per-field masking, GDPR export endpoint, consent model | Deferred | service |
| Record merge (link tracking, former-alias, `Replaces`, snapshot, `Merged` event) | Deferred | service |
| Duplicate review queue + batch deduplication scan | Deferred | service |
| Tantivy full-text / fuzzy search (replacing `ILIKE`) | Deferred | service |
| Real-time `409` duplicate detection on create | Open question (§16) | service |
| PASETO v4.public verification (auth-service Ed25519 key) + bearer wiring — see [`authentication-sessions.md`](../../agents/share/authentication-sessions.md) (supersedes RS256-JWT + JWKS) | Deferred | service + front-end |
| Search box, audit views, auth token in UI | Deferred | front-end |
| Telephone / email match component | Deferred (matcher spec §23) | matcher |
