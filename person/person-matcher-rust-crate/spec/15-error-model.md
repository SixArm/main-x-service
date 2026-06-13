## 15. Error Model

`MatchingError` is a `thiserror`-derived enum with `#[non_exhaustive]` (future variants do not break SemVer). One variant: `MissingField(String)`. `type Result<T> = std::result::Result<T, MatchingError>;`. `MissingField` is returned by `Person::validate` when no name / identifier / `passport_books` entry is populated. The matching engine is infallible; identifier parsers return `Option<String>` rather than `Result` (the parser is the source of truth); `MatchConfig::default` / `strict` / `lenient` are infallible. Earlier `InvalidData` / `InvalidUnitedKingdomNationalHealthServiceNumber` / `InvalidDate` / `ConfigError` variants were removed in T-13 (OQ-6).

---

