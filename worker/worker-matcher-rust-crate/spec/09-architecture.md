## 9. Architecture

Full per-file detail in [`agents/architecture.md`](../agents/architecture.md).

### 9.1 Module Layout

`src/` contains: `lib.rs` (re-exports); `models.rs` (`Worker`, `WorkerBuilder`, `Address`, `Gender`, `BloodType`, `PassportBook`); `identifiers.rs` (42 parsers + 9 passport-format validators); `matcher.rs` (`MatchConfig`, `MatchingEngine`, `MatchResult`, `MatchBreakdown`); `scorer.rs` (similarity primitives); `nicknames.rs` (`NicknameTable`); `normalizer.rs` (`Normalizer`); `error.rs` (`MatchingError`, `Result`); `main.rs` (demo binary).

### 9.2 Dependency Graph

`matcher` → `normalizer`, `scorer`, `models`, `identifiers`, `error`. `identifiers` → `nhs-number`. `models` → `serde`, `chrono`. `scorer` → `strsim`. `normalizer` → `unicode-normalization`, `soundex`. `error` → `thiserror`. No cycles; `lib.rs` only re-exports.

### 9.3 Layering Rules

`models` MUST NOT depend on any other crate module. `identifiers` MUST NOT depend on `matcher`, `normalizer`, or `scorer`. `normalizer` and `scorer` MUST NOT depend on `matcher`. `matcher` is the only orchestration layer. `main.rs` is the only place that performs `println!`.

---

