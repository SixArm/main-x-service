## 15. Roadmap

Versioned roadmap, aligned to the design's rollout
([cross-service-linking.md §11](../../../agents/share/cross-service-linking.md#11-rollout)).
**The version numbers below were never adopted as real `Cargo.toml`
versions** (the crate has stayed at `0.1.0` throughout — this is an
internal service, never published, so semver bumps track nothing an
external consumer needs); they remain useful only as a rollout
*ordering*, and delivery did not land in this exact order (governance,
T-16..18, landed alongside hardening rather than strictly before it;
LNK-4, below, landed as a distinct follow-on program well after
"v0.5"). [`spec/13-tasks.md`](13-tasks.md) is the authoritative
per-task status; treat the version labels here as a map of *what*, not
*when* or *as what version number*.

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
  (T-21); tracing / OTLP / health (T-22 — **OTLP landed 2026-08-05**,
  the family's first working exporter; container hardening still open).
- Flip transport to the durable bus per entity as Fluvio topics go live;
  retire lazy verify-on-read per entity (T-23) — design §11 step 5.

### Delivered beyond v0.5 — LNK-4 cross-service identity suggestion (2026-08-04)

Both items originally listed here as "candidate" future work are
**done**, closing LNK-4 (§13 T-29..T-33, §16 OQ-9):

- **Cross-service `same_identity` matcher** as an edge *producer* —
  landed as the `src/suggest/` comparator + periodic job (never a
  within-entity matcher signal — partition rule, design §7, still
  honoured; see §1.3, §5.5).
- **Suggestion review queue** for `provenance = matcher_suggested`
  edges (design §5.2) — landed, but **not as a queue in this service**:
  operator confirmation happens in **person's own** existing
  `review_queue` (T-32), which this service's job populates by
  `POST`ing suggestions to person's write API.

### Still candidate (not yet scheduled)

- **New edge kinds** by registry extension (e.g. course `taught_by`
  worker), each a row + endpoint-type pair + inverse (design §9 note).
- **Arbitrary-depth traversal** if real query patterns justify lifting
  the v1 depth cap (§16 OQ-1).
- **Cross-service suggestion for other edge kinds** (e.g. `works_at`/
  `employed_by` inference) — LNK-4 covers `same_identity` only; a
  generalisation is unscoped.
