## 13. Tasks

Live work queue for the **entity level** — cross-subproject work, or
single-subproject work with cross-subproject consequences. Work that
is purely internal to one crate belongs in that crate's queue
(service [§13](../course-service-with-loco/spec/13-tasks.md),
matcher [§23](../course-matcher-rust-crate/spec/23-tasks.md),
front-end [§13](../course-front-end-with-svelte/spec/13-tasks.md)).

- [x] T-1: Entity-level spec (§1–§18) + `AGENTS/` reference set
      established (this document set).
- [x] T-2: Bring the service crate's spec §8/§9 and
      `AGENTS/restful.md` in line with the loco conversion — they
      still described the pre-loco Axum boot (`main.rs` →
      `api::rest::serve`). Done 2026-06-13: §8 now documents the
      `src/app.rs` Hooks boot, `config/*.yaml`, the `/api` `Routes`
      prefix, the `FromRef<AppContext>` shared-store bridge, and the
      loco Migrator; §9 adds loco's `/_health` / `/_ping`; also
      refreshed §2/§10/§11/§14, `AGENTS/restful.md`,
      `AGENTS/spec-driven-development.md` section mapping, and the
      crate `index.md` configuration section (server binding now
      owned by loco config, not `SERVER_HOST`/`SERVER_PORT`).
- [x] T-3: Fix post-nesting link rot in the crate docs. Done
      2026-06-13: audited all three subprojects' markdown; fixed
      repo-root links (`../` → `../../` from top-level files,
      `../../` → `../../../` from `spec/` and `AGENTS/` files),
      cross-entity sibling links (now `../../../<entity>/<crate>/`),
      and renamed shared docs (`stack-for-rust-loco.md` →
      `rust-loco-stack.md`, `technology.md` → `loco.md`). Verified
      zero dangling relative links under `course/`.
- [x] T-4: Fix the front-end `README.md` detail-route description —
      "identifiers, addresses, telecom, emergency contacts" was a
      person-entity copy-paste. Done 2026-06-13: README/`index.md`
      route table, spec §2.1, and FR-4/FR-5 now describe what the
      detail page actually renders (course code, status, educational
      level, credits, identifiers, teaches, keywords, alternate
      names, same-as links, read-only instances); also corrected the
      false "/consents endpoints" and "service exposes FHIR routes"
      claims in the front-end `AGENTS.md` / spec §1.3.
- [ ] T-5: Close the instance-editing composition gap — the service
      ships full instance CRUD (`/api/courses/{id}/instances/*`) but
      the front-end renders instances read-only (front-end T-15).
- [ ] T-6: Syllabus-section round-trip — service read/write API
      (service roadmap v0.4) then front-end edit UI, in that order.
- [ ] T-7: SSO rollout across the trio — service verifies RS256 JWT
      against the [authentication entity](../../authentication/)
      JWKS (service T-15); front-end sign-in + token carriage +
      401/403 redirect handling (front-end OQ-3). One coordinated
      change cycle; blocks any governmental deployment (§12.3).
- [ ] T-8: Durable event bus — land the service's Fluvio adapter
      behind its feature flag so `CourseEvent`s survive process
      restart; document the consumer contract here when it exists.
- [ ] T-9: Wire `check-duplicates` into the front-end create form as
      a pre-submit preview (front-end T-17) — composition polish on
      FR-19.
- [ ] T-10: Operator-UI localization to the
      [`agents/share/locales.md`](../../agents/share/locales.md) set
      (NFR-7); front-end is English-only today.
- [ ] T-11: Bulk catalogue import path (national curriculum / OER
      feeds) with batch dedup — entity-level design needed before
      crate tasks can be cut (§15).
- [ ] T-12: Implement the **Lesson** sub-resource (§5.1) — a course
      contains 0..many ordered lessons (schema.org `LearningResource`,
      `hasPart`). Service: model + persistence + nested CRUD under
      `/api/courses/{id}/lessons` (mirror the `instances` sub-resource);
      front-end: list/edit lessons on the course detail page; matcher:
      no change (lessons are registry-only, dropped by the adapter §5.3).
      Spec-first: this entry tracks the code + test work for the concept
      now defined in §5.
