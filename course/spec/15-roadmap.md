## 15. Roadmap

Phased path from today's MVP trio to a governmental-scale national
course registry. Crate-internal milestones stay in the crate roadmaps
(service [§15](../course-service-rust-crate/spec/15-roadmap.md),
front-end [§15](../course-front-end-with-svelte/spec/15-roadmap.md));
this is the entity-level sequence.

- **E-1 — Secure the surface (next).** JWT enforcement in the
  service (verify RS256 against the
  [authentication entity](../../authentication/) JWKS, offline) +
  front-end sign-in / token carriage / 401-403 handling. One
  coordinated change cycle (§13 T-7). Nothing below ships to a
  governmental environment before this.
- **E-2 — Close the operator loop.** Instance + syllabus edit UI
  (after the service's syllabus read/write API),
  `check-duplicates` pre-submit preview, masked-view toggle,
  GDPR-export download, batch-dedup results UI. Outcome: catalogue
  stewardship is fully UI-complete.
- **E-3 — Durable events.** Fluvio adapter behind the service's
  feature flag; define and document the downstream consumer contract
  (audit mirroring, sync to peer registries). Outcome: `CourseEvent`s
  survive restart and feed other systems.
- **E-4 — Bulk catalogue import.** National curriculum / university
  catalogue / OER feed ingestion: batch create with deterministic
  short-circuits doing the heavy lifting, review queue absorbing the
  ambiguous remainder, dry-run mode for feed onboarding. Outcome:
  millions of records ingested without operator-per-record cost.
- **E-5 — Localization.** Operator UI localized to the
  [`agents/share/locales.md`](../../agents/share/locales.md) set;
  controlled multilingual vocabulary for `EducationalLevel`
  (A-levels, Abitur, Baccalauréat, … — service OQ-4 / §16 OQ-5);
  locale-aware search analyzers. Outcome: usable by non-English
  ministries day one.
- **E-6 — Scale out.** Stateless service replicas behind a load
  balancer; externalized / per-replica-rebuilt search index (the
  local Tantivy directory is the known pinch point, §8.4);
  multi-region PostgreSQL replication; CDN-served front-end; SLOs
  promoted from the single-node figures in §7. Outcome: NFR-2 /
  NFR-6 met at population scale.
- **E-7 — Ecosystem.** `CourseInstance` ↔ [event entity](../../event/)
  references (§16 OQ-2), `Provider` ↔
  [organization entity](../../organization/) linkage (§16 OQ-3), LMS
  round-trip (LTI / xAPI) — kept on the radar from the service's
  v0.5+ roadmap.
