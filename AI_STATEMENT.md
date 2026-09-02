# AI Statement

Version 1.2.0, status is active.

Canonical location is `AI_STATEMENT.md` at the repository root.

Review at least at every major release.

Abstract This document discloses how artificial-intelligence tools are used
to develop this software. It states what the tools do and do not touch, who is
accountable, which controls bound the work and how each is enforced, the
licensing and data posture, the rules for contributors, the uses that are
prohibited, and the limitations that survive all of it. It is a self-declaration
by the maintainer, written for evaluators and regulated adopters performing
supplier due diligence, and it changes in the same pull request that changes the
practice it describes.

The key words shall, should, and may, are used as ISO/IEC Directives Part 2
defines them: requirement, recommendation, permission.

## 1. Scope

This document covers the use of AI tools in developing everything in this
repository.

It does not cover an AI system in the product, because there is none. AI is used
to build the software, in the same sense that autocomplete, syntax linters, and
compilers, are used to build it.

## 2. Which frameworks apply here, and which do not

- The EU AI Act imposes no obligation on this project. The Act binds
  providers and deployers of AI systems (Articles 2 and 3(1)); this software
  is not one. Article 50's marking duties bind the AI tool's provider, not
  the tool's user, and the European Commission's Article 50 FAQ places
  source code outside the content-marking obligation. This document is
  voluntary.
- Medical-device regulation does not classify this project as a device.
  Under MDCG 2019-11 Rev. 1, electronic patient record systems that store,
  transfer, and retrieve records are not qualified as medical devices. A
  downstream integrator who gives their product a medical purpose may bring
  their product into scope; that classification is theirs to make, and
  this document exists partly so they can answer their own supplier
  questions.
- ISO/IEC 42001 and the NIST AI RMF are used as vocabulary, not claimed
  as conformity. No certification is claimed, no audit has occurred, and
  the words "certified," "audited," and "validated" appear in this document
  only inside this sentence, to say they do not apply.

## 3. Terms

This document uses the W3C AI Content Disclosure vocabulary:

- none: entirely human-authored
- ai-assisted: means human-authored; AI edited, refined, or filled in boilerplate
- ai-generated: AI-generated with human prompting and review
- autonomous: AI-generated without meaningful human oversight
- agentic tool: a tool that and executes multi-step work, under a human's direction, as opposed to inline completion.

## 4. Accountability

[MAINTAINERS.md](MAINTAINERS.md) lists the accountable maintainers for
every change in this repository, whatever tool produced the bytes.

## 5. Where AI is used, and at what level

The tooling is agentic AI coding assistance (currently Claude Code, by
Anthropic), operated in sessions the maintainer directs. Levels below
use the §3 vocabulary, per development activity. Deliberately, no
percentage appears anywhere in this document: no defensible method
exists for measuring one.

| Activity                                                      | Level        | Notes                                                                                                                                  |
| ------------------------------------------------------------- | ------------ | -------------------------------------------------------------------------------------------------------------------------------------- |
| Application and tooling code                                  | ai-generated | written in directed sessions; merged once it clears the §6 merge checklist                                                             |
| Tests (unit, DB-gated, property, fuzz)                         | ai-generated | held to the same authority as the code they test: expectations cite the specification, and the attribution rules below govern failures |
| Documentation and this statement                              | ai-generated | held to the repository's own prose rules                                                                                               |
| Merging a pull request into `main`                             | ai-generated | AI may merge once the §6 merge checklist is met — CI green on both platforms, the change follows its kind's discipline, the evidence is documented, and no unresolved specification-facing question is raised; a failing item routes to the maintainer instead |
| Judging an already-merged release ready, and publishing it (`cargo publish`) | ai-generated | AI may decide that a version bump already on `main` is ready to release — the §7 gates are green, the changelog is accurate, nothing else is pending — and execute the crates.io publish; scoped to this repository's crates already published to crates.io (`agents/share/overview.md`'s library-crates table) — never an unmerged version, and never a first publish of a crate not already chosen for crates.io |
| Specification adjudications, owner rulings, and what a change contains | none         | *what* changes — scope and content, for a PR or a release alike — is decided by the maintainer and recorded in the spec, through the PR's own content; the two rows above execute an *already-decided* change (merging it, or releasing it), never decide one |
| Contribution and review verdicts on others' work              | none         | not in use                                                                                                                             |

## 6. Human oversight

The maintainer directs the work. Where the tools run sessions, the
decisions with consequences — what a specification silence means, what
ships in a release — are directed to be recorded in the specifications.
A decision that exists only inside a tool session is not a decision
this project made.

As of 2026-09-02, two mechanical steps that used to require the
maintainer's separate, per-instance action — merging a pull request,
and publishing an already-decided release — may be carried out by AI
directly, each against its own explicit checklist below. Neither
delegation touches who decides **what** changes: scope, design, and
specification-facing adjudications stay the maintainer's (§5, §11), made
in the PR's content before either checklist is ever consulted.

**Merging a pull request.** AI **may** merge a pull request into `main`
once:

1. It follows the repository's own discipline for its kind of change —
   a three-part change (spec edit + code edit + test edit) where that
   discipline applies (§7), or is self-evidently scoped to a kind the
   discipline does not cover (documentation, governance, a dependency
   or version bump with no behavioural change).
2. CI is green on both platforms this repository runs it on — the same
   `scripts/ci-check.sh` stages, via GitHub Actions and the
   Woodpecker/Codeberg mirror (§7).
3. The PR description documents what was verified, to the same
   evidence bar [CONTRIBUTING.md](CONTRIBUTING.md)'s "Green gate" asks
   of any contributor.
4. The PR raises no unresolved specification-facing question — those
   route to the maintainer regardless of whose name is on the PR (§11).
5. Merge with `--no-ff` (a real merge commit; never squash, never
   rebase — [CONTRIBUTING.md](CONTRIBUTING.md)'s existing convention),
   onto the branch the PR targets, never by force-pushing over history.

**Publishing a release.** AI **may** decide that an already-merged
version bump on `main` is ready to release, and execute `cargo publish`
for it, once:

1. The version bump is already merged to `main` — through the checklist
   above, or by the maintainer directly; either way, not by AI
   short-circuiting it.
2. The gates are green for the crate being released: `fmt`,
   `clippy -D warnings`, `test` (and `test-db` where the crate is
   enrolled), `deny`, and the other §7 stages `scripts/ci-check.sh`
   runs for it.
3. The crate's `CHANGELOG.md` accurately reflects the version's
   content, and the `Cargo.toml` version matches the changelog entry
   being released.
4. The crate is one this repository has already chosen to publish to
   crates.io (§5's library-crates table) — never a first publish — and
   nothing else is known to be pending against it (no open blocking
   issue, no unresolved regression report).
5. Run `cargo publish`.

Either checklist, any unmet item means the action does not happen — AI
says which item failed and why, rather than merging or publishing
anyway. Both checklists are executions of a decision the PR's content
already made (through the spec + code + test discipline, or through the
maintainer directly); neither checklist is itself where scope, design,
or a specification silence gets decided (§11).

**One standing exception to the merge checklist, not a sixth item to
satisfy but a boundary on what the checklist covers at all:** a pull
request that changes [AI_STATEMENT.md](AI_STATEMENT.md),
[GOVERNANCE.md](GOVERNANCE.md), or [MAINTAINERS.md](MAINTAINERS.md) —
the documents that state what AI may do — is merged by the maintainer,
not by AI, however green its gates. This is what keeps the delegation
in §6 from ever being able to expand itself: AI may act on an authority
these documents already grant; it does not get to grant itself a new
one by merging the PR that would write it. An AI session may still
draft, verify, and open such a PR (as this one was) — merging it is
the one merge action §6 does not delegate.

## 7. Quality controls

- Specification authority. Each subproject's `spec/` directory is its
  single source of truth; a behavioural change lands as a three-part
  change (spec edit + code edit + test edit), so the code cannot
  silently diverge from the recorded intent, whoever wrote the bytes.
- Executed gates. CI runs the same commands on two independent
  platforms via `scripts/ci-check.sh`: `fmt`, `docs` (index + content
  scan), `clippy` (`-D warnings` against `#![warn(clippy::pedantic)]`),
  `test`, DB-gated `test-db` suites against a real PostgreSQL, `deny`,
  `evidence` (SBOM), `fuzz`, `msrv`, and `bench`. These are the
  controls that catch a plausible-but-wrong implementation regardless
  of who wrote it.
- Static and supply-chain gates. No `unsafe`
  (`#![forbid(unsafe_code)]` on every crate root), `#![deny(missing_docs)]`,
  typed errors, property tests (`proptest`) and fuzz targets over the
  matchers and security-relevant parsers, and a `cargo deny`
  advisory/licence policy.

## 8. Licensing and provenance of AI output

The project is licensed (see [LICENSE.md](LICENSE.md)). The position taken here follows the Apache
Software Foundation's and LLVM's published reasoning: an AI tool's output does not launder anyone's copyright, the
full provenance of generated text is generally not knowable, and prompting
alone is not treated as authorship. In practice: contributions of
substantially copied third-party material are refused however they were
produced; generated code is held to the same originality expectations as
human code, under the same review; and if identifiable third-party
material is found in the tree, it is removed or licensed properly, exactly
as it would be for a human-introduced copy. The tools are used under terms
that do not restrict the output's use in licensed software.

## 9. Data

No patient data, no personally identifiable health information, and no
customer data exists anywhere in this project — not in the repository, not
in test fixtures, not in telemetry, and therefore not in any prompt. Test
data and the `examples/data/` fixtures are synthetic by standing rule
(realistic-but-invented, with deliberate duplicate pairs and no real
PII). This is a structural property a reader can check against the tree,
not a promise about tool behaviour. Vendor-side data handling is
governed by the tool vendor's terms; this document deliberately makes no
claim on the vendor's behalf, because such claims go stale silently.

## 10. Rules for contributors

Contributors **may** use AI tools. A contribution with **ai-generated**
content per §3 **shall** say so in the pull-request description — which
tool, and what it did. This repository's commit convention additionally
carries a `Co-Authored-By` trailer on AI-generated commits; the trailer
is attribution, not disclosure — this document is the maintained
disclosure, and the PR description is where a contribution's specifics
belong. Naming a tool in a `Co-Authored-By` trailer does not make it an
author, a co-author, or a signer in any accountable sense: the git
author and committer fields are always the human who ran the tool and
submitted the change (§4, §6), and the trailer changes none of that —
it is metadata, not a claim of authorship or a certification the
contributor could otherwise sign. The contributor
remains responsible for their submission in full, under the same
[CONTRIBUTING.md](CONTRIBUTING.md) bar as any other work: understood,
explained on request, tested, and honest.

## 11. Prohibited uses

In this project, AI **shall not**: merge a pull request that fails any
item of §6's merge checklist, or merge one by squash/rebase/force-push
instead of a real `--no-ff` merge commit; adjudicate, score, or answer
reviews of contributions (the PR reviewer of §7 is advisory input to
the maintainer, not a verdict); sign anything; decide a
specification-facing question (silences are adjudicated by the
maintainer and recorded) — including one a PR's own content raises,
which blocks that PR's merge under §6 item 4 regardless of its other
items; publish a crate whose version bump is not already on `main`, or
publish a first release of a crate this repository has not already
chosen to publish (§5); or weaken a test, an expectation, or a gate to
make something pass — the last being a standing hard rule for humans
and tools alike.

## 12. Limitations and residual risks

This section exists because a disclosure without one is marketing.

- **The gates prove what they test, not correctness.** The conformance
  suites demonstrate the behaviours they cover; coverage is
  broad, ratchets upward, and is itself reviewed — and it is still a
  boundary, published with the run records.
- **Review depth is one person's, and some merges now have none of it
  per-instance.** The project has a single maintainer; machine gates
  (§7, the §6 merge checklist) stand in for the review capacity a
  larger team would have ([GOVERNANCE.md](GOVERNANCE.md) says this
  plainly). Since 2026-09-02, a PR clearing the §6 checklist may merge
  without the maintainer looking at that specific diff first — the gate
  *is* the review for that merge, not a proxy for one that also
  happened. "The maintainer designed and can explain the gate every
  merge has to clear" is the honest claim; "the maintainer read every
  merged diff" or "every line was independently re-derived" would not
  be.
- **Retroactivity.** Commits predating this statement carry no disclosure
  markers; this document describes the practice, not a per-commit audit
  trail, and no such trail is claimed.
- **Provenance uncertainty survives.** Whether any generated fragment
  echoes unlicensed training material is not fully knowable with current
  tools; §8 states the handling, not a guarantee.
- **The legal ground is unsettled.** Copyright in AI output is an open
  question in most jurisdictions; this document records positions, and
  positions may have to change. §13 names the triggers.
- **This is a self-declaration.** No third party has audited it. The
  checkable artifacts in §7 are the counterweight: they can disagree with
  this document, and if they do, the document is wrong.

## 13. Review and change

This statement is reviewed at every major or minor release, and revised
off-cycle when any of these fires: the tooling changes materially, a tool
vendor's terms change in a way §8 or §9 relies on, a binding rule emerges
(EU AI Act guidance touching this use, a foundation policy this project
follows, a court decision on AI output and copyright), or a claim in this
document stops being true. The maintainer owns the review; the change
lands as a pull request like everything else, and the version and change
log update in the same PR.

## 14. Reporting

A suspected provenance, licensing, or quality problem in this repository —
including a claim in this document that does not survive checking — is a
report this project wants. Open an issue and cite this file; for anything
security-sensitive, use the private route in [SECURITY.md](SECURITY.md).
The handling commitment is the same as for any defect: attributed,
answered on the tracker, and never silently absorbed.

## 15. References

**Normative for this project** (the documents that bind the practice
described here): [LICENSE.md](LICENSE.md); each subproject's `spec/`
directory and the shared design docs under
[agents/share/](agents/share/index.md); the working agreements in
[AGENTS.md](AGENTS.md); [GOVERNANCE.md](GOVERNANCE.md),
[MAINTAINERS.md](MAINTAINERS.md), [CONTRIBUTING.md](CONTRIBUTING.md),
[SECURITY.md](SECURITY.md).

**Informative** (the sources this document's structure and positions draw
on, surveyed 2026-08-24):
the W3C AI Content Disclosure vocabulary; the ISO/IEC Directives Part 2
document conventions and verbal forms; RFC 7322's required-considerations
discipline; ICMJE's AI-authorship position; the Apache Software
Foundation's generative-tooling guidance; the Linux Foundation's
generative-AI policy; the Fedora Council's AI-assisted-contributions
policy; the Linux kernel, LLVM, Kubernetes, NumPy, Mozilla, QEMU, curl,
Gentoo, OpenInfra, Ghostty, Kyverno, and nf-core positions; the OpenSSF
security guidance for AI code assistants and the OpenSSF/CNCF maintainer
guide; NIST AI RMF and ISO/IEC 42001 as vocabulary; EU AI Act Articles 2,
3, and 50 with the European Commission's Article 50 FAQ; MDCG 2019-11
Rev. 1.

## Annex A. Change log

| Version | Date       | Change                                                                 |
| ------- | ---------- | ---------------------------------------------------------------------- |
| 1.0.0   | 2026-08-26 | First issue for this repository, adapted from the 2026-08-24 template. |
| 1.1.0   | 2026-09-02 | Authorizes AI to judge an already-merged version bump of a crate this repository already publishes to crates.io ready to release, and to execute `cargo publish` for it (§5, §6, §11), against an explicit five-item checklist (§6) — the version already merged, the §7 gates green, the changelog accurate, the crate already chosen for crates.io with nothing else pending, then publish; any unmet item means no publish. *What* a release contains remains the maintainer's decision, made via the same reviewed-and-merged PR every other change goes through; the readiness-to-publish judgment for an already-decided, already-merged version — and the mechanical publish step — no longer need a separate ask. §10 also spells out, plainly rather than left implicit, that the standing `Co-Authored-By` trailer convention names a tool as attribution only — never as an accountable author, co-author, or signer, which remains the human — prompted by finding a vendored external template in this same review that carried the opposite (wrong) no-trailer rule (`spec/special-files-for-public-repos/AI_STATEMENT.md`, fixed the same day). |
| 1.2.0   | 2026-09-02 | Authorizes AI to merge a pull request into `main` (§5, §6, §11), replacing the prior absolute "AI merges nothing" / "AI shall not merge anything" prohibition with an explicit five-item checklist: the change follows its kind's discipline (three-part where it applies), CI is green on both platforms, the PR documents its verification evidence, no unresolved specification-facing question is raised, and the merge is a real `--no-ff` commit (never squash/rebase/force-push). *What* a PR contains — scope, design, spec adjudications — is unaffected and stays the maintainer's, decided in the PR's own content before the checklist is ever consulted; the checklist governs only whether an already-decided change is merged. §12's "review depth" limitation is updated to say so plainly: a merge clearing the checklist may have had no maintainer eyes on that specific diff, and the honest claim narrows from "the maintainer read every merged diff" to "the maintainer designed and can explain the gate every merge has to clear." |

## Annex B. Machine-readable summary

Levels per the W3C AI Content Disclosure vocabulary (§3); the prose above
is authoritative where the two could ever disagree.

```yaml
ai-statement:
  version: 1.2.0
  last-updated: 2026-09-02
  vocabulary: w3c-ai-content-disclosure
  disclosure-default: ai-generated
  tools:
    - name: Claude Code
      provider: Anthropic
  processes:
    design: ai-assisted
    implementation: ai-generated
    testing: ai-generated
    documentation: ai-generated
    merging: ai-generated  # pull requests into main, gated on the §6 merge checklist
    release-publishing: ai-generated  # readiness judgment + cargo publish for an already-merged version bump, see §5/§6
    review: none
    adjudication: none
    release-decisions: none
  ships-ai-system: false
  autonomous-use: none
```
