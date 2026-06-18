## 13. Tasks

Live entity-level work queue. The entity is **spec-only; no code
exists yet** (§14), so every task below is **unchecked** — this is the
build-out backlog. Tasks that belong to one subproject's internals
migrate into that crate's spec §13 once the crate is scaffolded; they
are listed here while the trio is being stood up. Each task has an
acceptance criterion; tick the box when an automated test or clearly
described manual check confirms it. Split tasks too big for one PR
(`T-2a`, `T-2b`).

- [ ] **T-1 — Scaffold the trio.**
  - [ ] Create `plan-matcher-rust-crate/`, `plan-service-with-loco/`,
    and `plan-front-end-with-svelte/` from the care-pathway siblings
    (copy-adapt; drift accepted — repo decision 2026-06-02).
  - [ ] Each subproject ships its own `spec/` (matcher §1–§25; service
    + front-end §1–§18) referencing this entity spec's §5 as the
    canonical domain model rather than redefining it.
  - [ ] Add the entity `AGENTS/` reference set (`index.md`,
    `models.md`, `matching.md`, `restful.md`, `testing.md`,
    `subprojects.md`, `spec-driven-development.md`).
  - [ ] Register the trio in the root `AGENTS.md`, `agents/share/overview.md`,
    and the front-end table.
  - **Acceptance:** every link in this entity spec resolves to a real
    file or section.
- [ ] **T-2 — Matcher crate: domain model + matching.**
  - [ ] The canonical `Plan` type + `Goal` + all enums (§5.1–§5.4),
    serde round-trip, NFKC-folding diacritic-preserving normalisation.
  - [ ] Deterministic short-circuits: R-0 (each deterministic
    identifier scheme — `Uri`, `Uuid`, `JiraProjectKey`, `AsanaGid`,
    `TrelloBoardId`, `MsProjectId`, `GitHubProjectId`, `LinearId`;
    owner-scoped `PlanCode`/`LocalId`/`Custom` excluded), R-1 (same
    `owner_org_id` + equal normalised `plan_code`), R-2 (`same_as`
    overlap) → 1.0.
  - [ ] Probabilistic components + weights per §6.7 (name JW + Soundex
    bonus, goal-title Jaccard, plan code, owner org, plan type,
    timeframe date-proximity, keywords, relationships typed-set
    Jaccard, tags), renormalised over present components; presets
    strict/default/lenient.
  - **Acceptance:** `cargo test` green; the FR-7 weight table sums to
    1.00 in a test; each rule + component has a unit test; public-API
    + doctest suites pass.
- [ ] **T-3 — Service crate: chassis + thin-record CRUD + matching.**
  - [ ] loco 0.16 / Axum 0.8 / SeaORM 1.1 chassis; `plans` migration;
    `cargo loco start`; config yamls; port 5150.
  - [ ] CRUD over the thin `Plan` (JSONB `data`), name search
    (`ILIKE`), `/match`, `/check-duplicates`, validation (§FR-1a).
  - **Acceptance:** DB-free matcher-embedding + JSON round-trip tests
    green; blank-name / malformed-`EntityRef` / malformed-deterministic-id
    → `422` pinned un-gated.
- [ ] **T-4 — Service crate: operational sub-resources + derived views.**
  - [ ] Tables + CRUD for tasks, issues, posts, comments, members;
    goals via `data.goals[]` mutation (the §5.3 bridge).
  - [ ] Derived `timeline` + `burndown` read endpoints; `task_snapshots`
    feeding burndown.
  - [ ] Real-time `409` duplicate detection on create; record merge
    that **re-homes sub-resources** to the survivor.
  - **Acceptance:** DB-gated request tests cover sub-resource CRUD,
    the goals bridge (a goal write changes a subsequent match score),
    `409` on create, and merge re-homing; the partition test (no
    sub-resource field in `data`; a non-goal sub-resource write does
    not change the match score) passes.
- [ ] **T-5 — Auditability + security.**
  - [ ] `audit_logs` (plan + sub-resource actions) + read endpoints;
    in-memory `PlanEvent` stream + `…/events/recent`.
  - [ ] Offline PASETO v4 public verification (`authentication-verifier`);
    switch `src/auth.rs` per
    [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
    (supersedes the RS256-JWT model). `AuthUser`/`MaybeAuthUser`; `whoami`
    protected; audit / merge `actor` stamped from the token.
  - **Acceptance:** create + update + delete a plan and a task → audit
    rows + events read back; no token → `401`, valid token → `2xx`.
  - [ ] *Follow-up:* blanket `/api/*` enforcement + paseto-keys-over-HTTP
    fetch (awaits the coordinated family SSO rollout; the front-end
    must attach the bearer token first); membership-scoped write
    authorisation.
- [ ] **T-6 — Front-end: routes + sub-resource workspaces + tests.**
  - [ ] `/`, `/new`, `/[pid]`, `/[pid]/edit` over the thin record;
    sub-resource workspaces (`/[pid]/{goals,tasks,issues,posts,members}`)
    and derived views (`/[pid]/{timeline,burndown}`).
  - [ ] vitest units (`ApiClient` + repositories) + Playwright smoke;
    `pnpm run check` strict 0/0 + production build.
  - **Acceptance:** both suites green; a contract-drift in any
    endpoint path fails a test.
- [ ] **T-7 — Cross-service links (write-side).**
  See §9.5 and
  [cross-service-linking.md](../../agents/share/cross-service-linking.md).
  - [ ] `entity_links` migration (`UNIQUE (from_pid, kind, to_ref,
    valid_from)`); `POST`/`GET`/`DELETE …/{pid}/links`; `linked` /
    `unlinked` events; the `EntityRef` value type (copied per project).
  - [ ] The partition rule (§7 there): links are never stored in
    `relationships` and never fed to the matcher.
  - **Acceptance:** a link create emits `linked`, a delete emits
    `unlinked`, and a test asserts no link ever reaches the matcher.
- [ ] **T-8 — Bulk import / export.**
  See §9.6, §10.4 and
  [bulk import/export](../../agents/share/bulk-import-export.md).
  - [ ] `bulk_jobs` migration (shared doc §3 schema, with
    `UNIQUE (entity, kind, idempotency_key)`).
  - [ ] The five endpoints (§9.6); `bg_pg` worker draining `queued →
    running → completed | completed_with_errors | failed`.
  - [ ] JSONL (lossless reference) + CSV (flattening per §9.6: every
    repeated / nested field a JSON-in-cell) codecs; Parquet
    **export-only**, feature-gated.
  - [ ] Per-row pipeline reusing the single-create validators +
    matcher + review queue: upsert by stable key (deterministic
    external-PM identifier, `(owner_org_id, plan_code)`, or `pid`,
    §9.6); keyless / unmatched rows → duplicate detection → review
    queue with `provenance = import`; events + audit not bypassed.
  - [ ] Downloadable per-row error report
    (`row_number, source_line, field, code, message`); one bad row
    never aborts the load; counts reconcile.
  - [ ] Export masking + audit: `masking_profile` (masked default —
    people references hidden; full gated), `include_soft_deleted`
    gated, every export audited (even zero-row).
  - **Acceptance:** integration tests cover idempotent re-import,
    the per-row error report, a keyless dedupe-to-review row
    (`provenance = import`), masked vs full export of a plan with
    people references, and that a zero-row export still writes an
    audit record.
- [ ] **T-9 — OpenAPI / Swagger + richer validation.**
  - [ ] OpenAPI 3 schema (hand-written, dependency-light, same
    approach as the organization / care-pathway services) + Swagger UI
    at `/api-docs/openapi.json` · `/swagger-ui`.
  - [ ] Validation of deterministic identifier shapes (UUID,
    external-PM-id patterns), `EntityRef` syntax, `in_language`
    (BCP-47), and relationship integrity (no self-reference,
    inverse-consistency, acyclicity — §5.8); `422` on failure.
  - **Acceptance:** Swagger UI serves the documented endpoints; a
    malformed-identifier / self-referencing-relationship test returns
    `422`.
