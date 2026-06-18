## 15. Roadmap

Versioned roadmap, aligned to the design's rollout
([cross-service-linking.md §11](../../../agents/share/cross-service-linking.md#11-rollout)).
Versions are indicative until the crate is scaffolded (§14).

### v0.1.0 — Contracts & scaffold

- Loco service skeleton, read-only to the world (§13 T-1).
- Copied `EntityRef` + `entity_type → service` map (T-2) and closed
  edge-kind registry (T-3).
- Read-model migrations: `edges`, `entity_presence`, `consumer_offsets`,
  `processed_events`, `audit_log` (T-4).
- No behaviour beyond the contracts (design §11 step 1).

### v0.2.0 — `same_identity` backbone

- Consume person + worker topics; envelope decode; idempotency;
  freshness watermark (T-5, T-6).
- Graph projector + symmetric canonicalisation; presence oracle; merge
  repointing (T-7, T-8, T-9).
- Interim lazy verify-on-read (T-10).
- `GET /neighbors` + `GET /single-view` over `same_identity`; `as_of`
  on every response (T-11, T-13, T-14) — design §11 step 3.

### v0.3.0 — Affiliations

- `person ↔ org` (`works_at` / `member_of`) and `worker ↔ org`
  (`employed_by`, with `role`); employer derivation in `single-view`.
- `GET /edges` full filter set (T-12).
- OpenAPI / Swagger (T-15) — design §11 step 4 (first half).

### v0.4.0 — `case ↔ person` governance

- `subject_of` / `about` edges with access control, audit, and privacy
  masking (T-16, T-17, T-18); no-leak governance tests (§11.4) —
  design §11 step 4 (second half).
- JWT verification via `authentication-verifier` (T-19).

### v0.5.0 — Hardening

- Reconciliation worker + divergence metric (T-20); Prometheus metrics
  (T-21); tracing / OTLP / health / container hardening (T-22).
- Flip transport to the durable bus per entity as Fluvio topics go live;
  retire lazy verify-on-read per entity (T-23) — design §11 step 5.

### Beyond v0.5 (candidate)

- **Suggestion review queue** for `provenance = matcher_suggested`
  edges (design §5.2) — operator confirmation promotes to
  `confidence = 1.0`.
- **Cross-service `same_identity` matcher** as an edge *producer*
  (still never a within-entity matcher signal — partition rule, design
  §7).
- **New edge kinds** by registry extension (e.g. course `taught_by`
  worker), each a row + endpoint-type pair + inverse (design §9 note).
- **Arbitrary-depth traversal** if real query patterns justify lifting
  the v1 depth cap (§16).
