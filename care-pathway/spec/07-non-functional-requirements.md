## 7. Non-Functional Requirements

Targets are for the worldwide governmental health-system deployment
the Main X Index aims at. Items the MVP does not yet meet are marked
*(roadmap)* and tracked in §15.

- **NFR-1 — Scale.** Hundreds of thousands of pathway records (every
  national guideline, trust pathway, and integrated care pathway
  across participating health systems); millions of read-side users
  via integrators. *(roadmap: today `check-duplicates` scans at most
  1 000 stored rows in memory — see OQ-2.)*
- **NFR-2 — Performance.** Read ≤ 5 ms p50; list ≤ 20 ms p50; pairwise
  match is pure CPU (no IO) and ≤ 1 ms; `check-duplicates` ≤ 500 ms
  p99 at target volumes *(roadmap: requires candidate blocking via
  search)*.
- **NFR-3 — Availability.** Stateless app tier, horizontal scaling,
  PostgreSQL replication, health checks (`/_health`, `/_ping`),
  graceful shutdown, non-root containers. See
  [`agents/share/availability.md`](../../agents/share/availability.md).
- **NFR-4 — i18n / locales.** Pathway names and keywords are
  Unicode-correct end to end; the matcher folds with NFKC and
  **preserves diacritics** (never strips them). `in_language` records
  the pathway's language. *(roadmap: multilingual operator UI per
  [`agents/share/locales.md`](../../agents/share/locales.md);
  cross-language linkage of the same pathway via `same_as` /
  deterministic identifiers.)*
- **NFR-5 — Security.** TLS at the edge; SSO via the central
  [authentication entity](../../authentication/) — the service
  verifies PASETO v4 public tokens offline against the auth-service's
  published Ed25519 key *(roadmap)*; no secrets in code or images. See
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md),
  which supersedes the prior RS256-JWT model.
- **NFR-6 — Auditability.** Soft delete with audit-friendly
  timestamps today; full audit log (who/what/when, old + new JSON)
  and event streaming on every CRUD *(roadmap)* per
  [`agents/share/auditability.md`](../../agents/share/auditability.md).
- **NFR-7 — Observability.** Structured tracing (`tracing` +
  `tracing-subscriber`, JSON logs in production) today; OTLP traces /
  metrics / logs *(roadmap)* per
  [`agents/share/observability.md`](../../agents/share/observability.md).
- **NFR-8 — Determinism and explainability.** Matching is a pure,
  total, deterministic function — no clocks, RNGs, IO, or panics in
  the matcher library; every score carries a breakdown an auditor can
  replay.
- **NFR-9 — Quality gates.** Green `cargo test`,
  `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`
  (both crates); `pnpm run check` strict 0/0 and production build
  (front-end).
