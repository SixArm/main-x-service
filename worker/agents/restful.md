# RESTful API — Worker entity

Entity-level orientation. The full endpoint reference (parameters,
envelopes, status codes, FHIR, library API, metrics) is
[service `agents/restful.md`](../worker-service-with-loco/agents/restful.md);
the normative inventory is
[service spec §9](../worker-service-with-loco/spec/09-api-surface.md)
and [entity spec §9](../spec/09-api-surface.md).

## Shape of the surface

All HTTP belongs to the service. One base URL, three tiers:

| Tier | Mount | State |
|---|---|---|
| REST (Axum) | `/api/workers/*`, `/api/audit/*`, `/api/health` | 15 endpoints, live |
| FHIR R5 | `/fhir/*` (Practitioner) | partial — see entity spec §13 T-1 for the path discrepancy |
| gRPC (Tonic) | — | stub |
| Docs / ops | `/swagger-ui`, `/metrics.prom` | live |

REST groups: CRUD (create returns `409` + candidates on duplicate),
search (`/search` with `q` / `fuzzy` / `phonetic` / `mask_sensitive`),
matching & dedup (`/match`, `/check-duplicates`, `/merge`,
`/deduplicate`), privacy (`/{id}/export`, `/{id}/masked`), audit
(`/{id}/audit`, `/api/audit/recent`, `/api/audit/user`).

## Conventions every caller relies on

- Envelope `{ success, data, error: { code, message, details } }`.
- `201` create, `204` delete, `409` duplicate, `422` validation,
  `404`, `500`.
- Pagination: `limit` + `offset`.
- These are part of the **entity-level contract** — changing them is
  an entity-spec change (entity §9.3, §18), not a service-internal
  tweak.

## The front-end as reference consumer

`worker-front-end-with-svelte/src/lib/api/` (`types.ts`,
`client.ts`, `workers.ts`) is the working example of consuming this
API, route map in [entity spec §9.2](../spec/09-api-surface.md).
Authentication: none yet — blanket auth enforcement and SSO wiring are
queued (entity spec §13 T-4; service §13 T-1). When it lands, peers
verify a short-lived PASETO v4.public token (published Ed25519 key) and
browsers carry an httpOnly cookie session via the front-end BFF — see
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md).
