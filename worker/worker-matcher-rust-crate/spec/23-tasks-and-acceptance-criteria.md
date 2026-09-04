## 23. Tasks and Acceptance Criteria

Single source of truth for outstanding work; absorbs what an SDD workflow would otherwise put in a separate `tasks.md`. Tasks are tagged `T-NN`. Status legend: `[ ]` open, `[~]` in progress, `[x]` done. Delivered tasks archived in [`agents/delivered-tasks.md`](../agents/delivered-tasks.md) (T-1..T-16 + §23.1 changelog roll-up) and [`agents/delivered-tasks-2.md`](../agents/delivered-tasks-2.md) (T-17..T-32 + project-level acceptance criteria). Only open tasks below.

### 23.1 Open tasks

**T-9.1 — Phonetic encoder enum (implementation follow-up to T-9).**
- [ ] Add `rphonetic` as optional dep behind the `phonetic-rphonetic` Cargo feature flag.
- [ ] Add `PhoneticEncoder` enum (`Soundex` default + `DoubleMetaphone` + `DaitchMokotoff`) and `MatchConfig::phonetic_encoder` field; default preserves current behaviour exactly.
- [ ] Refactor `Normalizer::phonetic_code(name)` → `Normalizer::phonetic_code(name, encoder)` (additive overload; no-encoder form retained for backward compat).
- [ ] Wire `MatchingEngine::score_phonetic_names` to honour `config.phonetic_encoder`.
- [ ] Define + test multi-code comparison semantics for Daitch-Mokotoff (FR-22a candidate): non-empty code-set intersection → `1.0`; single-name match → `0.5`; disjoint → `0.0`.
- **Acceptance:** Default-config behaviour and existing tests unchanged. New unit tests cover Double Metaphone primary/secondary equality (`Stephen`/`Steven`) and Daitch-Mokotoff Slavic-cluster equality (`Schwarz`/`Shvarts`). Documented "opt-in only" until T-9's corpus methodology is run.

**T-17.1 (residual).**
- [ ] TSV rows in `agents/national-person-identifiers.tsv` for the 7 FR-85..FR-91 schemes (parsers shipped without their TSV rows; verified missing — the file has no `br_cpf`/`cn_rrn`/`in_aadhaar`/`jp_my_number`/`mx_curp`/`nz_nhi`/`za_id` rows as of this audit).

**T-33 — Relationships as a weighted component (§8.1 / §8.6a / §12.2 / §13.1). Done 2026-08-28 (v0.7.0).**
- [x] Add `relationships: Vec<RelationshipRef>` to `Worker` and the `RelationshipRef` / `RelationKind` types (`LineManagerOf`, `ReportsTo`; re-export from crate root).
- [x] Score relationships by typed-set Jaccard over `(relation, worker_id)` pairs; `None` when either side is empty; add `relationships_score` to `MatchBreakdown`.
- [x] Add `relationships_weight` (default `0.05`) and include the field in the probabilistic weighted average (§12.3); keep weights renormalised; update `agents/matching-algorithm.md` detail tables + `CHANGELOG.md`.
- **Acceptance:** two records sharing related-worker ids score higher with a documented `relationships_score`; empty relationships do not participate; default weights renormalise correctly; `cargo test` + `cargo clippy --all-targets -- -D warnings` clean.

**T-34 — Tags as a weighted component (§8.1 / §8.5 / §12.2 / §13.1). Done 2026-08-28 (v0.7.0).**
- [x] Add `tags: Vec<String>` to `Worker` (default empty; normalised case-insensitively).
- [x] Score tags by set Jaccard over the normalised tag sets; `None` when either side is empty; add `tags_score` to `MatchBreakdown` (`#[serde(default)]`).
- [x] Add `tags_weight` (default `0.05`, supporting-signal cluster) and include the field in the probabilistic weighted average (§12.3); keep weights renormalised; update `agents/matching-algorithm.md` detail tables + `CHANGELOG.md`.
- **Acceptance:** two records sharing tags score higher with a documented `tags_score`; empty tags do not participate; default weights renormalise correctly; `cargo test` + `cargo clippy --all-targets -- -D warnings` clean.

**T-35 — Fuzz coverage for the national-identifier parsers (`src/identifiers.rs`).**
- [ ] Add a `fuzz/fuzz_targets/identifiers.rs` libFuzzer target that feeds arbitrary UTF-8 directly into each of the 42+ per-scheme parse/validate functions in `src/identifiers.rs` (verified: `fuzz/fuzz_targets/` has only `match_workers.rs`, `scorer.rs`, `normalizer.rs` — none references `identifiers::`, and the JSON-blob `match_workers` target cannot reliably reach deeply-nested per-scheme fields by byte fuzzing alone).
- [ ] Pin the never-panic + no-overflow invariant (`agents/share/security.md` invariant 2) across every scheme's checksum/regex validation, including the T-17.1 residual schemes once their TSV rows land.
- **Acceptance:** `cargo +nightly fuzz run identifiers` runs clean for the CI smoke duration; every parser in `src/identifiers.rs` is reachable from the new target's call list.

**T-36 — Library-level input-size bounds for standalone use (`agents/share/security.md` invariant 3).**
- [ ] Add `MAX_TEXT_LEN`/`MAX_ARRAY_LEN`-style caps (mirroring the family-wide SEC-M1 bounds) inside `MatchingEngine`/`Scorer` itself, not only at the embedding service's HTTP boundary (verified: no `MAX_TEXT_LEN`/`MAX_ARRAY_LEN`/length-guard hits in `src/matcher.rs`, `src/scorer.rs`, or `src/models.rs`; the crate is published as "usable standalone" per `agents/share/overview.md`, so a standalone caller can pass unbounded strings straight into the O(n·m) Jaro-Winkler/Levenshtein scorers with no library-side ceiling).
- [ ] Document the caps + their defaults in §7 (NFR) and §13 (configuration).
- **Acceptance:** a pathological multi-megabyte name/address input degrades gracefully (bounded time) rather than unboundedly scaling; new unit + proptest cases pin the ceiling; `cargo test` and `cargo clippy --all-targets -- -D warnings` clean.

**T-37 — Document the assessments-data boundary in §2 Scope.**
- [ ] Add an explicit "out of scope" clause to `spec/02-scope.md` stating that `worker-service`'s aptitude / personality / psychometric / selection assessment data (per-scale results, score bands, derived profile — the `worker-service` row of `agents/share/overview.md`) is never a matcher input, output, or scoring signal, mirroring the existing organisation-level-identifiers out-of-scope paragraph already in that section (verified: `grep -rni 'assessment|psychometric|aptitude|personality'` across this crate's `spec/` and `src/` returns zero hits — the boundary holds in practice but is nowhere stated).
- **Acceptance:** §2 states the boundary in the same style as the organisation-level-identifiers paragraph, cross-referencing `worker-service`.

---

