## 11. Public API Specification

Stable re-exports from `lib.rs`: `pub mod identifiers;` (42 personal-identifier parsers + 9 passport-format validators); `pub use error::{MatchingError, Result};`; `pub use matcher::{Confidence, MatchConfig, MatchResult, MatchBreakdown, MatchingEngine};`; `pub use models::{Address, BloodType, Gender, PassportBook, Person, PersonBuilder};`; `pub use nicknames::NicknameTable;`; `pub use normalizer::{Normalizer, ParsedAddressLine};`; `pub use scorer::{Scorer, SimilarityAlgorithm};`.

Stability rules: `Person` / `Address` carry `#[non_exhaustive]` (FR-53), constructed via builder or `Address::new().with_*(...)`. Adding fields → minor bump; removing / renaming → major bump; changing default weights → minor bump (CHANGELOG "Behaviour Change"); changing the meaning of `is_match` for the same `score` → major bump.

---

