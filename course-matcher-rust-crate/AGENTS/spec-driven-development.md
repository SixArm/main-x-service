# Spec-driven development — course-matcher

**The spec is the source of truth.** Every behavioural change is a
single PR containing:

1. **Spec edit** — `spec.md` §1–§25 (or §16 question + follow-up).
2. **Code edit** — `src/`.
3. **Test edit** — `src/**/tests` and the service-side bridge test
   in [`course-service/tests/duplicate_detection.rs`](../../course-service-rust-crate/) when the public surface changes.

## Section mapping

| Spec section | Corresponds to |
|---|---|
| §1 Purpose / §2 Scope | repo-level positioning (also `AGENTS.md`) |
| §3 Glossary | `src/course.rs` types, `src/scoring.rs` enums |
| §4 Research basis | `AGENTS/matching-algorithm.md` |
| §5 Algorithm overview | `src/matcher.rs` |
| §6 Domain model | `src/course.rs` |
| §7 Configuration | `src/config.rs` |
| §8 Normalisation | `src/normalize.rs`, `AGENTS/normalization.md` |
| §9–§18 per-component scoring | `src/matcher.rs` component fns |
| §19–§23 quality / consumption | top-level docs, integration with `course-service` |
| §24 Testing | `AGENTS/testing.md` |
| §25 Change control | this file |

## Anti-patterns

- **Adjusting a default weight without a §7 spec edit.** Reviewers
  diff the spec against `MatchConfig::default()`.
- **Adding a deterministic-identifier scheme without bridge-test
  coverage.** A false positive at score 1.0 is the worst-case bug.
- **Sneaking normalisation behaviour into `matcher.rs`.** Push it
  into `normalize::` and document the rule.
- **Using `unwrap` / `expect` in library code.** Total functions only.
