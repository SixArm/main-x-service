# Spec-driven development — agent guide

This crate practises **spec-driven development**: the specification ([`../spec.md`](../spec/index.md)) is the canonical artefact. Code conforms to the spec, not the other way around.

## What that means in practice

- The spec is *living* — it changes whenever observable behaviour changes.
- A behavioural change PR has **three** parts: spec edit, code edit, test edit. All in one PR.
- When spec and code disagree, the code is what is shipped today; the spec is updated to match the code, and the divergence is flagged in `CHANGELOG.md` so a human can decide whether the spec's original intent should be restored in a follow-up. See `spec.md` §9.3.
- When the spec is silent, propose an addition in `spec.md` before writing code.

## When to update which spec section

| You're changing… | Update spec section |
|---|---|
| The list of `Event` fields or any `Event` field's semantics | §3.1 |
| `EventBuilder` ergonomics | §3.2 |
| `Location` fields or scoring weights | §3.3, §6.4 |
| `Address` fields or scoring weights | §3.4, §6.4 |
| `EventCategory` variants | §3.5 |
| `EventStatus` / `EventAttendanceMode` variants | §3.6, §3.7 |
| `EventId` or `EventIdScheme` variants | §3.8 |
| `Confidence` band boundaries | §3.9 |
| `MatchBreakdown` / `MatchResult` shape | §3.10 |
| `MatchingError` variants | §3.12 |
| Normalisation behaviour (name, postcode, phone, email, phonetic, address, ISO 8601) | §4 |
| Deterministic-match rules | §5.1 |
| Probabilistic-match pipeline | §5.2 |
| Strict-mode behaviour | §5.2.3 |
| Batch entry points | §5.3 |
| Per-field scoring algorithm for any component | §6.* |
| Default weights or threshold; new preset; new `MatchConfig` knob | §7 |
| Determinism / safety guarantees | §8 |
| SemVer policy or `#[non_exhaustive]` posture | §9 |
| Open Question resolution | move from §10 into the relevant section |
| A new worked example | §11 |
| A new public symbol or rename | §12 (glossary) |

## Anatomy of a good spec edit

- **Precise language.** Use RFC 2119 keywords (MUST / SHOULD / MAY) for normative statements.
- **Examples for normalisation, tables for weights, prose for algorithms.** Mix forms only when the data calls for it.
- **No code in the spec beyond minimal type signatures.** The spec is what, not how. Worked examples (§11) are the exception.
- **No screenshots or diagrams that can't be diffed.** ASCII / Mermaid is fine; PNGs are not.
- **One concept per section.** If a section grows past ~300 lines, split it.

## Open questions (§10)

The spec carries a live list of design questions that are deliberately unresolved. Each is assigned a stable code (`OQ-A`, `OQ-B`, …) so other docs and code comments can reference it.

### Adding a new OQ

```
- **OQ-X — Short imperative title.** One paragraph describing the question, the surface trade-off, and (if known) the most likely resolution path.
```

Append to §10 in the next available letter. Cross-reference from any code or other doc that touches the question.

### Resolving an OQ

When an OQ is resolved:

1. Update the relevant spec section to reflect the decision.
2. Remove the OQ entry from §10.
3. Search the repository for `OQ-X` references and re-anchor them to the spec section that now carries the decision (or drop the reference if it is now stale).
4. Add a CHANGELOG entry under "Unreleased" noting the resolution.

## Closing the loop on a behavioural change

When you finish a behavioural change:

1. Update the relevant spec section in the same PR.
2. Add a `CHANGELOG.md` entry under "Unreleased".
3. Verify the change is pinned by an automated test (or a clearly described manual check) and that the doctest on the affected public item still compiles.
4. If the change resolved an Open Question, follow the resolution checklist above.

## CI enforcement

If the repo carries a spec-drift CI check (`.github/workflows/spec-drift.yml`, plus `scripts/spec-drift-check.sh` and `.spec-allow`), it fails any pull request that modifies watched source files without also updating `spec.md`. Path-pattern exceptions live in `.spec-allow`.

Run it locally before pushing:

```bash
bash scripts/spec-drift-check.sh main HEAD
```

If you have a genuinely spec-irrelevant change (e.g. an internal refactor of a private helper), prefer to add a one-line note to the spec — that is almost always the right answer — over adding a `.spec-allow` pattern. Every allow pattern erodes the discipline the check exists to enforce.

## Anti-patterns

- "I'll write the code now and update the spec later" — later never comes.
- "The spec is wrong; let me just fix it to match the code" — without first confirming the code's behaviour is the *intended* behaviour, you are laundering a bug into a feature.
- Adding behaviour gated by a flag that the spec does not mention.
- "It's only a refactor" used to justify a behavioural shift.
- Citing `spec.md §23` — the spec runs §1–§13. Earlier doc revisions had a section §23 that was retired; if you see a reference to §23 anywhere, fix it.

## Document hierarchy

```
spec.md                  ← what the library is and how it behaves (authoritative, SSOT)
README.md                ← user-facing intro (must stay consistent with spec)
CHANGELOG.md             ← what changed when (history)
AGENTS.md + AGENTS/*.md  ← how to work in the repo
index.md                 ← navigation aid (must cross-reference spec sections accurately)
```

There is intentionally **no** `plan.md` and **no** `tasks.md`. SDD artefacts that some projects split across multiple files are consolidated into the numbered sections of `spec.md` (in particular §10 Open Questions for outstanding work, §11 Worked Examples for illustrative scenarios).

If you find disagreement between any of these, file it as an issue and fix it in a follow-up PR.
