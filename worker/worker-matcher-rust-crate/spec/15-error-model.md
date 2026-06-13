## 15. Error Model

`MatchingError` is a `thiserror`-derived `#[non_exhaustive]` enum; current variants: `MissingField(String)`. `type Result<T> = std::result::Result<T, MatchingError>;`. `MissingField` is returned only by `Worker::validate` when no identifying field is populated. The matching engine itself is infallible — scoring two workers always produces a `MatchResult`. Identifier parsers return `Option<String>` (the parser is the source of truth on validity). Config builders are infallible. Four legacy variants (`InvalidData`, `InvalidNhsNumber`, `InvalidDate`, `ConfigError`) were removed in T-13; `#[non_exhaustive]` keeps future additions SemVer-safe.

---

