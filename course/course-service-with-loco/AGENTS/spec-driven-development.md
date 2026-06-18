# Spec-driven development — Course Service

The discipline: **the spec is the source of truth.** Code conforms
to the spec; not the other way around.

## Three-part PRs

Every behavioural change comes as a single PR with three parts:

1. **Spec edit** — `spec.md` §1–§18 (or open a §16 question, then a
   follow-up PR with the resolution).
2. **Code edit** — `src/`.
3. **Test edit** — `tests/` or `src/**/tests`.

Reviewers reject PRs that change behaviour without touching all
three.

## Section mapping

| Spec section | Corresponds to |
|---|---|
| §5 Domain Model | `src/models/`, `migrations/` |
| §6 Functional Requirements | `src/api/rest/handlers.rs`, validation, matcher adapter |
| §7 Non-Functional Requirements | benches, profiling, deployment config |
| §8 Architecture | `src/lib.rs`, `src/app.rs` (loco Hooks), `src/bin/main.rs`, module layout |
| §9 API Surface | `src/api/rest/mod.rs` + `AGENTS/restful.md` |
| §10 Persistence | `migrations/` + `migration/` (loco Migrator), `src/db/` |
| §11 Testing Strategy | `tests/`, `AGENTS/testing.md` |
| §12 Compliance | `src/privacy/`, audit-log writes |
| §13 Tasks | live work queue — the only place tasks live |
| §14 Implementation Status | rolled up from `cargo test`, CI badges |

## Anti-patterns

- **Adding a field to `Course` without updating `migrations/`,
  `models.md`, and §5.** Schema drift is the most common SDD
  failure mode.
- **Changing a match-component weight without a §6 FR update.** The
  weights are normative; reviewers check them against §6.
- **Marking a §13 task `[x]` without writing a test.** Done = code
  + test + spec edit, all merged.
- **Putting roadmap items in `README.md`.** Use §15.
- **Putting "ideas" in a separate notes file.** Use §16 Open
  Questions.
