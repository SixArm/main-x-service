# Testing

## Service edition

- **Pure-core unit tests** (`flow/`): the bed state machine (every
  legal transition, every illegal one → error, `deep_clean_required`
  propagation), allocation eligibility (each rule in isolation +
  combinations, override paths), Red2Green day rules, DTOC clock
  edges (grace period, midnight boundary). DB-free.
- **Request tests** (loco request tests + a Postgres test database):
  every controller happy path + validation `422`s; the admit /
  transfer / discharge flows end-to-end including bed-state side
  effects; concurrency test that two simultaneous placements of one
  bed produce exactly one occupant; whiteboard / at-a-glance /
  locate read shapes; masked vs unmasked whiteboard.
- **Auth matrix** (family standard, DB-free): flag off ⇒ open; flag
  on ⇒ 401 / read-allow / write-403-without-attrs / `access=write`
  writes / `access=admin` topology+closure/delete; record-level
  `resource.ward` scoping; `mask` obligation redacts names.
- **Audit/event pins**: each mutation writes its audit row and emits
  its envelope in the same transaction; sensitive reads audited.
- **Stub-mode boot test**: the service boots and serves with all
  upstream clients stubbed.

## Front-end edition

- **vitest** component tests: bed card rendering per state ×
  flags matrix; whiteboard grid; at-a-glance table; masked mode.
- **Playwright** e2e against the service in stub mode (case-folder
  precedent): admit → appears on whiteboard → set EDD → mark ready →
  discharge → bed shows awaiting-clean → clean → available; a
  bed-request allocation walk; locate search.

## Cross-cutting

- `cargo clippy --all-targets` pedantic-clean, `cargo deny` per repo
  policy; CI mirrors the case-folder quality/security workflows.
- Seed task (`cargo loco task seed`) creates a demo hospital (2
  sites, ~6 wards incl. 1 virtual + 1 escalation, ~120 beds, ~90
  synthetic stays) so whiteboards demo instantly; synthetic data
  only ([regulatory.md](regulatory.md)).
