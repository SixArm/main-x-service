# Governance

This project uses **sole-maintainer governance** (sometimes called
BDFL): Joel Parker Henderson (see [MAINTAINERS.md](MAINTAINERS.md))
holds final decision authority over scope, design, releases, and
membership — with two narrow, explicit delegations, both per
[AI_STATEMENT.md](AI_STATEMENT.md) §5/§6: the project's AI tooling may
merge a pull request into `main` once it clears an explicit checklist
(discipline followed, CI green, evidence documented, no unresolved
specification-facing question), and may judge an already-merged,
already-decided version bump ready to publish to crates.io and execute
that publish. What a change contains — scope, design, a release's
content — stays a maintainer decision, decided in the pull request
itself before either checklist is ever consulted; the checklists
govern only whether an already-decided change is merged or released.
**One boundary on the merge delegation:** a pull request changing this
document, [AI_STATEMENT.md](AI_STATEMENT.md), or
[MAINTAINERS.md](MAINTAINERS.md) is merged by the maintainer, not by
AI — the delegation cannot expand itself.

## How decisions are made

- **Design decisions** are recorded in the specs — each subproject's
  `spec/` directory and the shared design docs under
  [agents/share/](agents/share/index.md). A decision doc supersedes
  discussion; re-litigating a recorded decision requires new evidence.
- **Open questions** live in each spec's open-questions section until
  resolved, then are marked RESOLVED in place with the rationale.
- **Working agreements** for contributors and agents are in
  [AGENTS.md](AGENTS.md) and [CONTRIBUTING.md](CONTRIBUTING.md).

## Changing this document

By pull request, decided by the maintainer.
