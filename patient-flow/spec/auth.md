# Authentication & authorization

Patient Flow adopts the family stack unchanged
([authentication-sessions.md](../../agents/share/authentication-sessions.md),
[authorization-attributes.md](../../agents/share/authorization-attributes.md),
[security.md](../../agents/share/security.md)).

## Authentication

- Humans: cookie session against the central
  authentication-service; the front-end is a **BFF** (SvelteKit
  server) that exchanges the session for a short-lived **PASETO
  v4.public** token per outbound API call. No token in browser JS.
- Services: bearer PASETO, verified **offline** via
  `authentication-verifier` (`PATIENT_FLOW_PASETO_KEYS_URL` with
  `PATIENT_FLOW_PASETO_KEYS` env fallback, per the family pattern).

## Blanket guard

`PATIENT_FLOW_REQUIRE_AUTH` (default **off**, per the family
activation-gate posture — see security.md §4; activation is a
release gate for any real deployment). Guard-all /
deny-unless-public; public allow-list: `/api/health`, `/_health`,
`/_ping`, docs, `/metrics.prom`.

## ABAC

Standard action derivation (read / write / delete / destructive) with
the shared policy engine. Patient-flow-specific notes:

- **Destructive named POSTs**: none in v1 (no merge/dedupe/import);
  `DELETE` covers it.
- **Suggested deployment vocabulary** (policy config, not code):
  `access = write` for ward staff actions (bed states, stays,
  red2green), `access = admin` for topology changes (wards/bays/
  beds), closures, and deletes; `ward` / `dept` attributes to scope
  staff to their wards via `resource.ward` record-level rules.
- **Record-level checks + masking**: `GET /api/locate/*`, stay
  detail, and whiteboard responses derive `resource.ward` and honour
  the `mask` obligation by redacting patient display names and
  alerts (bed states remain visible — a domestic team member can see
  a bed needs cleaning without seeing who was in it).

## Sensitivity

An inpatient stay reveals that a person is in hospital — that is
**health-adjacent personal data**. Consequences, mirrored in
[audit.md](audit.md) and [regulatory.md](regulatory.md):

- locate and stay reads are audited (who looked up whom, when);
- whiteboard endpoints support masked rendering for
  public-facing/corridor screens;
- infection flags are clinical information: visible on ward
  whiteboards (operationally necessary) but excluded from any
  wider-than-ward view except as anonymous counts
  ([capacity.md](capacity.md) reports numbers, never names).
