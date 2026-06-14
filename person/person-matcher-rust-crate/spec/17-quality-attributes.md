## 17. Quality Attributes

- **Correctness** — behaviour matches §12; verified by §18 unit + integration tests.
- **Explainability** — every score carries a per-field `MatchBreakdown`.
- **Performance** — `< 50 µs` per `match_persons` on a 2024-era Mac; verified by `benches/match_pair.rs` (criterion, T-5; single-pair fuzzy match ≈ 4 µs).
- **Maintainability** — no single file > 500 lines (`matcher.rs` exempt pending refactor).
- **Portability** — pure Rust, no C deps beyond `chrono` / `strsim` defaults.
- **Auditability** — all score combinations documented in §12.

---

