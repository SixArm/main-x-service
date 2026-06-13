## 5. Research Basis

Grounded in: Grannis SJ et al. *Person matcher within a Health Information Exchange* (AMIA Annu Symp Proc, 2014; PMC4696093); Reisman M. *Patient Identification Techniques* (NCVHS, 2020; PMC7442501). PDFs in [`help/`](../help/).

**Findings:** real-world error rates average ~8% (can reach 20%); even best-in-class techniques top out at 90–98% accuracy; hybrid deterministic + probabilistic strategies outperform either alone; data standardisation before matching is essential (most gains come from normalisation, not cleverer scoring); single-identifier reliance is brittle — multi-factor matching is more robust.

**Application:** inputs are normalised before scoring (§14); multiple weak signals combine via weighted average (§12); match results are transparent — every component score is in `MatchBreakdown`; defaults are conservative (threshold `0.85`) and can be tightened (`strict()`) or relaxed (`lenient()`).

---

