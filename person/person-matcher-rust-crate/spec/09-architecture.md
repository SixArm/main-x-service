## 9. Architecture

### 9.1 Module Layout

`src/lib.rs` (public API re-exports), `src/models.rs` (Person, PersonBuilder, Address, Gender, BloodType, PassportBook), `src/identifiers.rs` (42 per-scheme parsers + 9 passport-format validators), `src/matcher.rs` (MatchConfig, MatchingEngine, MatchResult, MatchBreakdown, Confidence), `src/scorer.rs` (similarity primitives), `src/nicknames.rs` (NicknameTable), `src/normalizer.rs` (name / postcode / phone / address / phonetic / email normalisation), `src/error.rs` (MatchingError, Result), `src/main.rs` (demo binary; not library API). See `agents/architecture.md` for diagrams.

### 9.2 Dependency Graph

`matcher → normalizer / scorer / models / identifiers / error`; `identifiers → united-kingdom-national-health-service-number`; `models → serde / chrono`; `scorer → strsim`; `normalizer → unicode-normalization / soundex`; `error → thiserror`. No cycles. `lib.rs` only re-exports.

### 9.3 Layering Rules

`models` MUST NOT depend on any other crate module. `identifiers` MUST NOT depend on `matcher` / `normalizer` / `scorer` — it is a leaf beneath `matcher`. `normalizer` and `scorer` MUST NOT depend on `matcher`. `matcher` is the only orchestration layer. `main.rs` is the only place that performs `println!`.

---

