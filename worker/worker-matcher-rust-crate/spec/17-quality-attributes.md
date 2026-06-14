## 17. Quality Attributes

Correctness (behaviour matches §12; verified by §18 tests). Explainability (per-field `MatchBreakdown` on every call). Performance (`< 50 µs` per `match_workers` on 2024-era Mac; verified by `benches/match_pair.rs` — single-pair fuzzy ~4 µs). Maintainability (no single file > 500 lines, `matcher.rs` exempt pending refactor). Portability (pure Rust, no C deps beyond `chrono` / `strsim` defaults; `cargo build` on Linux + macOS). Auditability (all score combinations documented in §12).

---

