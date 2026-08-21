# AGENTS.md — Working Guide for AI Coding Agents

This file is the entry point for AI coding agents (Claude, Cursor,
Aider, Devin, etc.) working in the `course-matcher` Rust crate. Humans
are welcome too — these are the rules of the road regardless of who is
at the keyboard.

> **Navigating the docs:** [`index.md`](./index.md) is the top-level
> documentation index — start there if you don't know what to read
> first.
>
> If you only read one file, read [`spec.md`](./spec/index.md). It is the
> living, authoritative specification of the crate. This guide tells
> you **how to work**; the spec tells you **what to build**.

---

## Quick orientation

| Question | Answer |
|---|---|
| What does the crate do? | Pairwise course-record matching (deterministic + probabilistic) per [schema.org/Course](https://schema.org/Course), modelling only the properties that carry identity signal. |
| Where is the canonical spec? | [`spec.md`](./spec/index.md) — §1–§25 (matcher-crate shape). |
| Where does new behaviour get specified? | In `spec.md` first, then code. |
| What is the build command? | `cargo build` |
| What is the test command? | `cargo test` |
| What is the run command? | `cargo run` — demo binary (`src/main.rs`), illustrative only, not part of the SemVer surface. |
| What is the lint command? | `cargo clippy --all-targets -- -D warnings` |
| What is the format command? | `cargo fmt` |
| Where do public types live? | `src/lib.rs` re-exports; defined under `src/{course,matcher,scoring,normalize,phonetic,config,error}.rs`. |
| Which deterministic identifier schemes short-circuit? | DOI, Wikidata, OER ID, LOM ID, generic URI, UUID. Plus per-provider course-code equality and `same_as` URL equality. See spec §15–§16. |
| Which probabilistic components score? | Name (Jaro-Winkler), course code (same-provider), educational level, keywords (Jaccard), teaches / competencies (Jaccard). Spec §9–§14. |
| What is the public API shape? | `MatchingEngine::new(MatchConfig::default()).match_courses(&a, &b) -> MatchResult { score, is_match, confidence, breakdown }`. Mirrors the family-wide convention used by `person-matcher` and `event-matcher`. |

---

## Golden rules

1. **Spec-first.** If you change observable behaviour, update
   [`spec.md`](./spec/index.md) in the same change. If the spec is silent,
   propose a spec update before writing code.
2. **Pure library.** No IO, no logging, no global state inside `src/`
   (excluding `src/main.rs` if present, which is a demo binary).
3. **No `unsafe`.** This is an identity-adjacent library. `unsafe` is
   forbidden.
4. **Deterministic.** No clocks, no RNGs, no environment variables.
   Same inputs ⇒ same outputs, byte-for-byte.
5. **Explainability over cleverness.** Every probabilistic match
   returns a per-field breakdown. Don't add black boxes.
6. **Diacritic-correct.** Unicode diacritics in course names
   (`Café`, `Über`, `Élève`, …) must round-trip through normalisation.
7. **Total functions only.** No `unwrap` / `expect` / `panic!` in
   library code. `Option` / `Result` is the answer.
8. **Run the full test suite before declaring success.** `cargo test`
   must pass; `cargo clippy --all-targets -- -D warnings` must be
   clean.

---

## Workflow for any change

1. **Read** [`spec.md`](./spec/index.md). Locate the section(s) affected.
2. **Decide** whether your change is editorial (docs / format only) or
   behavioural (touches what the library does).
3. **For behavioural changes:**
   - Update `spec.md` first (or alongside) with the new wording.
   - Update or add tests that pin the new behaviour.
   - Implement the change.
   - Update `CHANGELOG.md` under an "Unreleased" header.
   - If the public surface changed, update the bridge test in
     [`course-service/tests/duplicate_detection.rs`](../course-service-with-loco/tests/duplicate_detection.rs).
4. **For editorial changes:** small batched edits are fine; no spec
   update required.
5. **Validate locally:**
   - `cargo fmt`
   - `cargo clippy --all-targets -- -D warnings`
   - `cargo test`
6. **Open a PR** with a description that references the spec
   section(s) you touched.

---

## Detailed guides

The `agents/` directory contains topic-specific guidance. Read the one
that matches your task before editing:

- [agents/spec-driven-development.md](./agents/spec-driven-development.md)
  — How `spec.md` is maintained as the single source of truth;
  three-part PRs, section mapping, anti-patterns.
- [agents/matching-algorithm.md](./agents/matching-algorithm.md) —
  The algorithm itself: per-component derivations, weights, deterministic
  short-circuits, renormalisation, confidence classification.
- [agents/normalization.md](./agents/normalization.md) — String
  normalisation rules (case-fold, NFKC, course-code shape, keyword /
  teaches tokenisation).
- [agents/testing.md](./agents/testing.md) — Unit + bridge test
  strategy; what each layer pins.

---

## What not to do

- Do not add network or filesystem dependencies to library code.
- Do not introduce `tokio`, `async-std`, or any runtime into the
  library crate.
- Do not change default weights or thresholds without updating
  `spec.md §7` (`MatchConfig::default`) and `CHANGELOG.md`.
- Do not silently widen or narrow the public API; every re-export
  from `lib.rs` is part of the SemVer contract.
- Do not add panicking `unwrap` / `expect` in library code.
- Do not score course codes across providers. Per spec §11, the
  course-code component only contributes when `provider_id` matches
  on both sides — otherwise CS101 at one school != CS101 at another.
- Do not add a deterministic identifier scheme without a bridge test
  in the sibling `course-service`. A false positive at score 1.0 is
  the worst-case bug.
- Do not sneak normalisation behaviour into `matcher.rs`. Push it
  into `normalize::` and document the rule.

---

## When you are unsure

- The spec wins. If the spec disagrees with the code, **trust the
  spec** and write a task in `spec.md §23` to bring the code in line.
  Do not silently align the spec to broken code.
- If the spec is silent, propose an update in `spec.md` and ask for
  human sign-off via a PR rather than guessing.
- Prefer adding an Open Question over making a unilateral design
  decision.

---

## File layout reminder

```
/
├── AGENTS.md                 ← this file
├── agents/                   ← topic-specific agent guides
│   ├── index.md
│   ├── spec-driven-development.md
│   ├── matching-algorithm.md
│   ├── normalization.md
│   └── testing.md
├── CHANGELOG.md
├── CLAUDE.md                 ← @AGENTS.md (Claude Code entry)
├── Cargo.toml
├── README.md                 ← user-facing (symlink → index.md)
├── index.md                  ← documentation entry point
├── spec/                     ← LIVING SPECIFICATION (read this)
│   ├── index.md              ← spec entry point
│   └── 01..25-*.md           ← numbered §1–§25 section files
└── src/
    ├── lib.rs                ← public re-exports
    ├── main.rs               ← demo binary (cargo run; not SemVer surface)
    ├── course.rs             ← domain types
    ├── matcher.rs            ← MatchingEngine, per-component fns
    ├── scoring.rs            ← MatchResult, MatchBreakdown, Confidence
    ├── normalize.rs          ← normalisation rules
    ├── phonetic.rs           ← phonetic encoding (name component)
    ├── config.rs             ← MatchConfig (weights + threshold)
    └── error.rs              ← error type
```
