## 7. Non-Functional Requirements

Targets are for the worldwide governmental deployment the Main X Index
aims at. Items the build does not yet meet are marked *(roadmap)* and
tracked in §15.

- **NFR-1 — Scale.** Millions of case records across participating
  agencies and source systems; millions of read-side users via
  integrators. *(roadmap: today `check-duplicates` scans at most 1 000
  stored rows in memory — `CHECK_DUPLICATES_SCAN_CAP`, see OQ-2.)*
- **NFR-2 — Performance.** Read ≤ 5 ms p50; list ≤ 20 ms p50; pairwise
  match is pure CPU (no IO) and ≤ 1 ms; `check-duplicates` ≤ 500 ms p99
  at target volumes *(roadmap: requires candidate blocking via
  search)*.
- **NFR-3 — Availability.** Stateless app tier, horizontal scaling,
  PostgreSQL replication, health checks (`/_health`, `/_ping`),
  graceful shutdown, non-root containers. See
  [`agents/share/availability.md`](../../agents/share/availability.md).
- **NFR-4 — i18n / locales.** Case titles and keywords are
  Unicode-correct end to end; the matcher folds with NFKC and
  **preserves diacritics** (never strips them). `in_language` records
  the case's language(s). *(roadmap: multilingual operator UI per
  [`agents/share/locales.md`](../../agents/share/locales.md);
  cross-language linkage of the same matter via `same_as` /
  deterministic identifiers.)*
- **NFR-5 — Security.** TLS at the edge; SSO via the central
  [authentication entity](../../authentication/) — the service verifies
  RS256 JWTs offline against the auth-service JWKS (delivered for
  `whoami` + `actor` stamping; *blanket `/api/*` enforcement is
  roadmap*); no secrets in code or images (JWKS/issuer/audience injected
  via env: `CASE_JWKS` / `CASE_JWT_ISSUER` / `CASE_JWT_AUDIENCE`).
- **NFR-6 — Auditability.** Soft delete + a durable `audit_logs` row
  per create/update/delete/merge (who/what/when, JSON snapshot) +
  in-memory event streaming today, per
  [`agents/share/auditability.md`](../../agents/share/auditability.md);
  *durable event bus is roadmap*.
- **NFR-7 — Privacy.** **Case records are personal data** (§12). The
  domain model keeps personal detail out of the registry by design
  (subjects are opaque ids), but operator identities in audit data and
  any free text that strays into the payload are personal data.
  Per-field masking and GDPR data-subject export are *roadmap and
  raised in priority* relative to most sibling entities — see §12,
  §15, §13 T-10.
- **NFR-8 — Observability.** Structured tracing (`tracing` +
  `tracing-subscriber`, JSON logs in production) today; OTLP traces /
  metrics / logs *(roadmap)* per
  [`agents/share/observability.md`](../../agents/share/observability.md).
- **NFR-9 — Determinism and explainability.** Matching is a pure,
  total, deterministic function — no clocks, RNGs, IO, or panics in the
  matcher library; every score carries a breakdown an auditor can
  replay.
- **NFR-10 — Quality gates.** Green `cargo test`,
  `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`
  (both crates); `pnpm run check` strict 0/0 and production build
  (front-end).
