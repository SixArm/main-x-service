# AGENTS.md — Working Guide for AI Coding Agents

This file is the entry point for AI coding agents (Claude, Cursor, Aider, Devin, etc.) working in the `worker-matcher` Rust crate. Humans are welcome too — these are the rules of the road regardless of who is at the keyboard.

> **Navigating the docs:** [`index.md`](./index.md) is the top-level documentation index — start there if you don't know what to read first.
>
> If you only read one file, read [`spec.md`](./spec.md). It is the living, authoritative specification of the crate. This guide tells you **how to work**; the spec tells you **what to build**.

---

## Quick Orientation

| Question | Answer |
|---|---|
| What does the crate do? | Pairwise worker-record matching (deterministic + probabilistic) for healthcare information exchange. |
| Where is the canonical spec? | [`spec.md`](./spec.md). |
| Where does new behaviour get specified? | In `spec.md` first, then code. |
| What is the build command? | `cargo build` |
| What is the test command? | `cargo test` |
| What is the lint command? | `cargo clippy --all-targets -- -D warnings` |
| What is the format command? | `cargo fmt` |
| Where do public types live? | `src/lib.rs` re-exports; defined under `src/{models,matcher,scorer,normalizer,nicknames,identifiers,error}.rs`. |
| Where are demo runs? | `cargo run` and `cargo run --example basic_usage`. |
| Which national identifier schemes are supported? | **42 personal-identifier schemes** plus 9 per-country passport-format validators feeding `PassportBook`. The personal-identifier list (one Worker field per scheme): UK NHS, FR NIR, ES TSI, IE IHI, UK NI H&C, US SSN, AU IHI, DE KVNR, IT CF, NL BSN, SE Personnummer, UK Scotland CHI, BE NN, BG EGN, CZ RČ, DK CPR, EE IK, ES DNI, FI HETU, HR OIB, IS KT, LT AK, LV PK, MT ID, NO FNR, PL PESEL, RO CNP, SI EMŠO, SK RČ, UK NINO, GR DSS, LI ID, NL ID, PL NIP, PT NIF, plus the T-17.1 batch: BR CPF, CN RRN, IN Aadhaar, JP My Number, MX CURP, NZ NHI, ZA ID. Passport-format validators: CY, CZ, LI, LT, MT, NL, PT, RO, SK. One parser per scheme in `src/identifiers.rs`; never cross-match across schemes. |
| How do I record passports? | `Worker::passport_books: Vec<PassportBook>`. Each `PassportBook` carries an ISO 3166-1 alpha-2 `country`, a `number`, and optional `issued` / `expires` dates. A worker may carry several books (multi-country, current + historical). The matcher treats any shared `(country, number)` pair across the two workers' lists as a match; cross-country values with the same number never cross-match. See spec §6.4a / §8.6. |
| Which phone-number jurisdictions are recognised by E.164 normalisation? | 39 countries (covers every national-identifier scheme jurisdiction the crate parses) — see spec §14.3.2 / §21.4 (T-19) for the full table. |

---

## Golden Rules

1. **Spec-first.** If you change observable behaviour, update [`spec.md`](./spec.md) in the same change. If the spec is silent, propose a spec update before writing code.
2. **Pure library.** No IO, no logging, no global state inside `src/` (excluding `src/main.rs`, which is a demo binary).
3. **No `unsafe`.** This is a clinical-adjacent library. `unsafe` is forbidden.
4. **Deterministic.** No clocks, no RNGs, no environment variables. Same inputs ⇒ same outputs, byte-for-byte.
5. **Explainability over cleverness.** Every probabilistic match returns a per-field breakdown. Don't add black boxes.
6. **Diacritic-correct.** Unicode diacritics (`â`, `ŷ`, `é`, `ü`, `ł`, …) must round-trip through normalisation. Don't break this.
7. **No PII in tests.** Use synthetic examples only. The existing fixtures are illustrative, not real, and new fixtures MUST follow the same rule.
8. **No `println!` in `src/` except `src/main.rs`.**
9. **Run the full test suite before declaring success.** `cargo test` must pass; `cargo clippy --all-targets -- -D warnings` must be clean.

---

## Workflow for Any Change

1. **Read** [`spec.md`](./spec.md). Locate the section(s) affected.
2. **Decide** whether your change is editorial (docs/format only) or behavioural (touches what the library does).
3. **For behavioural changes:**
   - Update `spec.md` first (or alongside) with the new wording.
   - Update or add tests that pin the new behaviour.
   - Implement the change.
   - Update `CHANGELOG.md` under an "Unreleased" header.
4. **For editorial changes:** small batched edits are fine; no spec update required.
5. **Validate locally:**
   - `cargo fmt`
   - `cargo clippy --all-targets -- -D warnings`
   - `cargo test`
   - Optionally `cargo run` to smoke-test the demo.
6. **Open a PR** with a description that references the spec section(s) you touched.

---

## Detailed Guides

The `AGENTS/` directory contains topic-specific guidance. Read the one that matches your task before editing:

- [AGENTS/architecture.md](./AGENTS/architecture.md) — Module layout, layering rules, dependency graph.
- [AGENTS/coding-style.md](./AGENTS/coding-style.md) — Rust style, naming, doc comments, error handling.
- [AGENTS/testing.md](./AGENTS/testing.md) — Test pyramid, naming, fixtures, coverage expectations.
- [AGENTS/matching-algorithm.md](./AGENTS/matching-algorithm.md) — Deterministic and probabilistic scoring, weights, phonetics; carries the verbatim spec §12 detail.
- [AGENTS/normalization.md](./AGENTS/normalization.md) — Name, postcode, phone, identifier, and phonetic normalisation rules; carries the verbatim spec §14 detail.
- [AGENTS/security-and-privacy.md](./AGENTS/security-and-privacy.md) — PII, data protection, clinical-safety guardrails.
- [AGENTS/release.md](./AGENTS/release.md) — Versioning, CHANGELOG, publishing checklist.
- [AGENTS/spec-driven-development.md](./AGENTS/spec-driven-development.md) — How `spec.md` is maintained as the single source of truth.
- [AGENTS/national-person-identifiers.md](./AGENTS/national-person-identifiers.md) — Reference table of the supported national identifier schemes (jurisdiction, endonym, ISO 3166-1 code, format). 42 schemes total.
- [AGENTS/delivered-tasks.md](./AGENTS/delivered-tasks.md) — Archive of delivered §23.1 + T-1..T-16 tasks (originally inline in `spec.md §23`).
- [AGENTS/delivered-tasks-2.md](./AGENTS/delivered-tasks-2.md) — Archive of delivered T-17..T-32 plus project-level acceptance criteria.
- [AGENTS/roadmap-research.md](./AGENTS/roadmap-research.md) — Archive of research-spike outcomes (T-17, T-9, T-19, T-14) originally inline in `spec.md §21.4`.

---

## What Not To Do

- ❌ Do not add network or filesystem dependencies to library code.
- ❌ Do not introduce `tokio`, `async-std`, or any runtime into the library crate.
- ❌ Do not change default weights or thresholds without updating §13 of the spec and CHANGELOG.
- ❌ Do not "fix" the address sub-score arithmetic silently — see Open Question OQ-4 in `spec.md` and follow the agreed resolution path.
- ❌ Do not silently widen or narrow the public API; every re-export from `lib.rs` is part of the SemVer contract.
- ❌ Do not add panicking unwraps in library code. `Option`/`Result` is the answer.
- ❌ Do not log worker data. Do not `Debug`-format records into traces.
- ❌ Do not cross-match national identifiers across schemes (e.g. a UK NHS Number against a UK NI H&C Number with the same digits). Per spec §12.1 / FR-13, identifiers are scheme-local.
- ❌ Do not score `local_id`. Different organisations may issue colliding values (resolved §22 OQ-2).
- ❌ Do not seed `NicknameTable::english()`'s exact contents into tests as a stable assumption — the dictionary may gain entries in minor releases. Test against `with_class` constructs you control.

---

## When You Are Unsure

- The spec wins. If the spec disagrees with the code, **trust the spec** and write a task in `spec.md` §23 to bring the code in line. Do not silently align the spec to broken code.
- If the spec is silent, propose an update in `spec.md` and ask for human sign-off via a PR rather than guessing.
- Prefer adding to §22 (Open Questions) over making a unilateral design decision.

---

## File Layout Reminder

```
/
├── AGENTS.md                 ← this file
├── AGENTS/                   ← topic-specific agent guides
├── CHANGELOG.md
├── CITATION.cff
├── CODE_OF_CONDUCT.md
├── CONTRIBUTING.md
├── Cargo.toml
├── IMPLEMENTATION_SUMMARY.md ← historical; superseded by spec.md
├── README.md                 ← user-facing
├── examples/
├── help/                     ← source research papers
├── index.md                  ← documentation entry point
├── spec.md                   ← LIVING SPECIFICATION (read this)
├── src/
└── tests/
```
