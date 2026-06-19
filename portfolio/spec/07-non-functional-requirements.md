## 7. Non-Functional Requirements

Targets are for the worldwide governmental-portfolio deployment the
Main X Index aims at. The entity is **spec-only; no code exists yet**
(§14), so every target is aspirational and tracked in §13 / §15.

- **NFR-1 — Scale.** Hundreds of thousands of work-item records across
  the four collections (every portfolio, project, product, and program
  across participating departments); each may own thousands of tasks
  and issues — hence the matchable/operational partition (§5.6), which
  keeps the high-volume sub-resources out of both the matcher payload
  and the cheap list/read projections. *(roadmap: `check-duplicates`
  must use search-blocked candidates within a collection rather than an
  in-memory scan — see OQ-2.)*
- **NFR-2 — Performance.** Read ≤ 5 ms p50; list ≤ 20 ms p50; pairwise
  match is pure CPU (no IO) and ≤ 1 ms (the kind gate makes a
  cross-kind comparison cheaper still); `check-duplicates` ≤ 500 ms p99
  at target volumes *(roadmap: requires candidate blocking via
  search)*. Sub-resource list endpoints paginate; timeline / burndown
  are computed within p99 ≤ 200 ms over a single work item's
  sub-resources.
- **NFR-3 — Availability.** Stateless app tier, horizontal scaling,
  PostgreSQL replication, health checks (`/_health`, `/_ping`),
  graceful shutdown, non-root containers. See
  [`agents/share/availability.md`](../../agents/share/availability.md).
- **NFR-4 — i18n / locales.** Work-item names, goal titles, keywords,
  and free-text are Unicode-correct end to end; the matcher folds with
  NFKC and **preserves diacritics** (never strips them). `in_language`
  records the work item's language. *(roadmap: multilingual operator UI
  per [`agents/share/locales.md`](../../agents/share/locales.md);
  cross-language linkage of the same initiative via `same_as` /
  deterministic identifiers.)*
- **NFR-5 — Security.** TLS at the edge; SSO via the central
  [authentication entity](../../authentication/) — the service
  verifies PASETO v4 public tokens offline against the auth-service's
  published Ed25519 key
  ([`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md),
  supersedes the RS256-JWT model); leads / assignees are user
  identities resolved by `EntityRef`; no secrets in code or images.
- **NFR-6 — Auditability.** Soft delete with audit-friendly
  timestamps; full audit log (who/what/when, old + new JSON) and event
  streaming on every CRUD **and sub-resource write** per
  [`agents/share/auditability.md`](../../agents/share/auditability.md).
  A durable event bus is roadmap (§15).
- **NFR-7 — Observability.** Structured tracing (`tracing` +
  `tracing-subscriber`, JSON logs in production); OTLP traces /
  metrics / logs *(roadmap)* per
  [`agents/share/observability.md`](../../agents/share/observability.md).
- **NFR-8 — Determinism and explainability.** Matching is a pure,
  total, deterministic function — no clocks, RNGs, IO, or panics in
  the matcher library; the kind gate and every score carry a breakdown
  an auditor can replay. Date-proximity scoring is computed from the
  records' declared dates, never from "now".
- **NFR-9 — Quality gates.** Green `cargo test`,
  `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`
  (both crates); `pnpm run check` strict 0/0 and production build
  (front-end).
