## 22. Open Questions and Risks

Open questions are tracked here until resolved.

- **OQ-1..OQ-6 — Resolved.** Middle-name scoring (T-25 / FR-49); email + `local_id` (T-11 / FR-35/36); `#[non_exhaustive]` on `Person` / `Address` (T-8 / FR-53); address sub-score weighted average (T-3 / §12); strict-mode enforcement (T-4 / FR-47); `MatchingError` cleanup leaving only `MissingField` (T-13).
- **OQ-7 — Resolved (T-38).** The phonetic bonus participates in `total_weight` only when it applies (i.e. only when `phonetic_name_score > 0.9`), never unconditionally just because a phonetic score was computed — pinned byte-for-byte by `matcher::tests::total_weight_includes_phonetic_bonus_only_when_bonus_applies` in `src/matcher.rs`, and documented in `agents/matching-algorithm.md`'s Probabilistic Scoring Pipeline (§12.3).

### 22.1 Risks

- Misuse as a decision oracle (Med/High) — documentation; require `MatchBreakdown` on every call.
- Diacritic-heavy name false negatives (Med/Med) — NFKD pipeline; T-9.1 phonetic encoder follow-up.
- Spec / code drift (High/Med) — T-7 CI check.
- Soundex collisions cluster too aggressively (Med/Low) — phonetic is bonus-only.
- `united-kingdom-national-health-service-number` dep becomes unmaintained (Low/Med) — pin minor version; vendored fallback documented.
- Cross-scheme identifier confusion (Med/High) — FR-13 forbids cross-scheme equality; consumers must record provenance at ingest.
- ES TSI lenient validation admits malformed regional values (Med/Low) — deliberate; consumers may layer a community-specific check.

---

