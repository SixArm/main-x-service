## 5. Research Basis

Grounded in Grannis SJ et al. (AMIA, 2014) and Reisman M. (NCVHS, 2020); PDFs in [`help/`](../help/). Findings: real-world error rates ~8% (reach 20%); best-in-class tops 90–98%; hybrid deterministic + probabilistic beats either alone; data standardisation matters more than cleverer scoring; multi-factor more robust than single-identifier reliance. Application: inputs normalised before scoring (§14); weak signals combined via weighted average (§12.3); per-field `MatchBreakdown` transparency; conservative default threshold `0.85` tunable via `strict()` / `lenient()`.

---

