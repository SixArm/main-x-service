# Governance

This project uses **sole-maintainer governance** (sometimes called
BDFL): Joel Parker Henderson (see [MAINTAINERS.md](MAINTAINERS.md))
holds final decision authority over scope, design, releases, and
membership — with one narrow, explicit delegation: judging an
already-merged, already-decided version bump ready to publish to
crates.io may be done by the project's AI tooling directly, and it may
execute that publish, per [AI_STATEMENT.md](AI_STATEMENT.md) §5/§6.
What a release contains stays a maintainer decision, made through the
normal spec-driven change and merge process below; only the
readiness-to-publish judgment for an already-merged version is
delegated.

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
