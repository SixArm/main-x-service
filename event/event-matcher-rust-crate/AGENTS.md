# AGENTS.md — working guide for AI coding agents

This file is the entry point for AI coding agents (Claude, Cursor, Aider, Devin, etc.) working in the `event-matcher` Rust crate. Humans are welcome too — these are the rules of the road regardless of who is at the keyboard.

> **Navigating the docs:** [`index.md`](./index.md) is the top-level documentation index — start there if you don't know what to read first.
>
> **Domain change in 0.5.0.** The crate was repurposed from geographic *place* matching to a schema.org/Event matcher. `spec.md` has been rewritten against the event surface and is authoritative again; the 0.4.x place behaviour survives only in the 0.5.0 CHANGELOG entry and the `0.4.x` releases.

---

## Quick orientation

| Question | Answer |
|---|---|
| What does the crate do? | Pairwise matching of event records (festivals, conferences, concerts, sports fixtures, screenings, hackathons, meetups), modelled on [schema.org/Event](https://schema.org/Event), deterministic and probabilistic, for de-duplication and record linkage. |
| Where is the spec? | [`spec.md`](./spec/index.md) — living SSOT for the event-matcher surface. |
| Where does new behaviour get specified? | In `spec.md` and `CHANGELOG.md` in the same PR as the code change. See [AGENTS/spec-driven-development.md](./AGENTS/spec-driven-development.md). |
| Build command | `cargo build` |
| Test command | `cargo test` (unit + integration + property + doctest) |
| Lint command | `cargo clippy --all-targets -- -D warnings` |
| Format command | `cargo fmt` |
| Where do public types live? | `src/lib.rs` re-exports; defined under `src/{models,matcher,scorer,normalizer,error}.rs`. |
| Where are demo runs? | `cargo run` and `cargo run --example basic_usage`, `cargo run --example custom_config`, and `cargo run --example location_matching`. |
| What's the deterministic-match rule? | Any shared `(scheme, value)` pair across `event_ids`, OR identical normalised `name` plus a `start_date` that parses to the same Unix instant. |
| What's the probabilistic-match pipeline? | Weighted, weight-renormalised sum across name / start_date / end_date / location / category / country_code / event_ids / organizer / performers / url; missing fields skip. Optional Soundex bonus when phonetic gating clears. |
| Default match threshold | `0.80`. Strict: `0.95`. Lenient: `0.65`. |
| Default start_date scale | `3600.0` seconds (1 hour). Gaussian decay `exp(-(d/s)^2)`. |
| Default coordinates scale (inside `location`) | `100.0` metres. |
| `#[non_exhaustive]` items | `Event`, `Address`, `Location`, `EventCategory`, `EventStatus`, `EventAttendanceMode`, `EventIdScheme`, `MatchingError`. Construct via builders / `new`. |

---

## Golden rules

1. **Spec-first.** If you change observable behaviour, update [`spec.md`](./spec/index.md) in the same change. If the spec is silent, propose a spec update before writing code. (`spec.md` §9.3, [AGENTS/spec-driven-development.md](./AGENTS/spec-driven-development.md))
2. **Pure library.** No IO, no logging, no global state inside `src/` (excluding `src/main.rs`, which is a demo binary). (`spec.md` §8)
3. **No `unsafe`.** Enforced by `#![forbid(unsafe_code)]` in `lib.rs`. Do not remove the attribute.
4. **Deterministic.** No clocks, no RNGs, no environment variables. Same inputs => same outputs, byte-for-byte. (`spec.md` §8)
5. **Explainability over cleverness.** Every probabilistic match returns a per-field `MatchBreakdown`. Don't add black boxes. (`spec.md` §3.7)
6. **Diacritic-correct.** Unicode diacritics (`â`, `ŷ`, `é`, `ü`, `ł`, …) must round-trip through normalisation. Don't break this. (`spec.md` §4.1)
7. **No real personal data in tests.** An event can carry an organiser, performers, a virtual URL, or a local-id that is personal data; use synthetic examples only. Reuse existing illustrative fixtures (`example.org` per RFC 2606, drama-reserved `07700 900xxx` UK ranges, fictitious `(415) 555-…` US ranges). See [AGENTS/security-and-privacy.md](./AGENTS/security-and-privacy.md).
8. **No `println!` in `src/` except `src/main.rs`.**
9. **Run the full test suite before declaring success.** `cargo test` must pass; `cargo clippy --all-targets -- -D warnings` must be clean; `cargo doc --no-deps` must build without warnings.

---

## Workflow for any change

1. **Read** [`spec.md`](./spec/index.md). Locate the section(s) affected.
2. **Decide** whether your change is editorial (docs / formatting only) or behavioural (touches what the library does).
3. **For behavioural changes:**
   - Update `spec.md` first (or alongside) with the new wording.
   - Update or add tests that pin the new behaviour. See [AGENTS/testing.md](./AGENTS/testing.md).
   - Implement the change.
   - Update `CHANGELOG.md` under an "Unreleased" header.
4. **For editorial changes:** small batched edits are fine; no spec update required.
5. **Validate locally:**
   - `cargo fmt`
   - `cargo clippy --all-targets -- -D warnings`
   - `cargo test`
   - `cargo doc --no-deps`
   - Optionally `cargo run` and `cargo run --example basic_usage` to smoke-test.
6. **Open a PR** with a description that references the spec section(s) you touched.

---

## Detailed guides

The `AGENTS/` directory contains topic-specific guidance. Read the one that matches your task before editing:

- [AGENTS/architecture.md](./AGENTS/architecture.md) — module layout, layering rules, dependency graph. (`spec.md` §3, §8)
- [AGENTS/coding-style.md](./AGENTS/coding-style.md) — Rust style, naming, doc comments, error handling. (`spec.md` §8, §9)
- [AGENTS/testing.md](./AGENTS/testing.md) — test pyramid, fixtures, property tests, doctest hygiene.
- [AGENTS/matching-algorithm.md](./AGENTS/matching-algorithm.md) — deterministic and probabilistic scoring, weights, phonetics, coordinates. (`spec.md` §5, §6, §7)
- [AGENTS/normalization.md](./AGENTS/normalization.md) — name, postcode, phone, email, address, phonetic rules. (`spec.md` §4)
- [AGENTS/security-and-privacy.md](./AGENTS/security-and-privacy.md) — personal-data handling, no-IO posture, threat model. (`spec.md` §8)
- [AGENTS/release.md](./AGENTS/release.md) — versioning, CHANGELOG, publishing checklist. (`spec.md` §9)
- [AGENTS/spec-driven-development.md](./AGENTS/spec-driven-development.md) — how `spec.md` is maintained as the single source of truth.

---

## What not to do

- Do not add network or filesystem dependencies to library code.
- Do not introduce `tokio`, `async-std`, or any runtime into the library crate.
- Do not change default weights or thresholds without updating `spec.md` §7 and `CHANGELOG.md` in the same PR.
- Do not silently widen or narrow the public API; every re-export from `lib.rs` is part of the SemVer contract. (`spec.md` §9)
- Do not add panicking unwraps in library code. `Option` / `Result` is the answer. (`spec.md` §8)
- Do not log event data. Do not `Debug`-format records into traces.
- Do not score `local_id`. Different organisations may issue colliding values. (`spec.md` §3.1.1)
- Do not cross-match `EventId` values across schemes. An `(Eventbrite, "abc")` and a `(Meetup, "abc")` refer to different things and must not match. (`spec.md` §3.8)
- Do not construct `Event`, `Location`, `Address`, `EventCategory`, `EventStatus`, `EventAttendanceMode`, `EventIdScheme`, or `MatchingError` via struct-literal syntax from downstream code. All carry `#[non_exhaustive]`.

---

## When you are unsure

- The spec wins. If the spec disagrees with the code, propose a fix in `spec.md` rather than silently realigning. Then bring the code in line in the same PR. See `spec.md` §9.3 and [AGENTS/spec-driven-development.md](./AGENTS/spec-driven-development.md).
- If the spec is silent, propose an update in `spec.md` and ask for human sign-off via a PR rather than guessing.
- Prefer adding to `spec.md` §10 Open Questions over making a unilateral design decision.

---

## File layout reminder

```text
/
├── AGENTS.md                 ← this file
├── AGENTS/                   ← topic-specific agent guides
├── CHANGELOG.md              ← version history
├── CITATION.cff              ← citation metadata
├── CODE_OF_CONDUCT.md
├── CONTRIBUTING.md
├── Cargo.toml
├── LICENSE.md
├── README.md                 ← user-facing
├── benches/                  ← criterion benchmarks
├── examples/                 ← runnable examples (basic_usage, custom_config, location_matching)
├── index.md                  ← documentation entry point
├── spec/                     ← LIVING SPECIFICATION (SSOT — index.md + NN-*.md; read this)
├── scripts/                  ← spec-drift check
├── src/                      ← crate source
└── tests/                    ← integration tests, property tests
```

> **Note on `spec.md`.** Throughout these docs `spec.md` is shorthand for
> the spec **directory**: [`spec/index.md`](./spec/index.md) (table of
> contents) plus the numbered sections `spec/01-*.md` … `spec/13-*.md`.
> There is no single top-level `spec.md` file on disk. Markdown links to
> `spec.md` resolve to `./spec/index.md`.
