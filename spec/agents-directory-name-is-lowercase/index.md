# The agents directory name is lowercase

The AI-agent reference directories in this repository are named
**`agents`**, lowercase — the repository-wide `agents/share/` and every
per-subproject `agents/`.

| | |
|---|---|
| Directory name | **`agents`** (lowercase, always) |
| File names | **`AGENTS.md`**, **`CLAUDE.md`** (uppercase, unchanged) |
| Directories holding to this | 34 — the root `agents/` (holding `share/`), 11 entity-level, 22 per-subproject |
| Enforced by | `scripts/ci-check.sh docs` — see §4 |

## 1. The rule

- A directory of agent-facing reference documents is `agents/`.
- Every reference to one — a markdown link, a Rust doc comment, a
  `@include` — spells it `agents/`.
- **The files keep their conventional uppercase names.** `AGENTS.md` and
  `CLAUDE.md` are cross-tool conventions with the same standing as
  `README.md` and `CHANGELOG.md`; an agent, an editor, or a person
  looking for them expects that spelling. This rule is about the
  **directory**, and a directory sitting next to `AGENTS.md` is
  `agents/`.

So a subproject root reads:

```
CLAUDE.md          @AGENTS.md
AGENTS.md          working agreements
agents/            topic guides: testing.md, restful.md, …
spec/              the living specification
```

## 2. Why lowercase, and why this is not a taste question

It was settled by a defect, not a preference. Until 2026-08-21 the
directories were tracked as `AGENTS/`, while **878 files referenced them
as `agents/`** — including `AGENTS.md`'s own first include,
`@agents/share/overview.md`. Only two references used the tracked
spelling.

Nobody had noticed, for a specific reason: `core.ignorecase = true` on
macOS, where the work was done, resolves either spelling. The mismatch is
invisible until the repository meets a **case-sensitive** filesystem —
and it meets one constantly:

- **Both remotes' web UIs** are case-sensitive regardless of anyone's
  local disk, so every one of those relative links was already dead where
  these documents are actually read.
- **Linux CI, containers, and Linux checkouts** get `AGENTS/` from the
  index, so `agents/…` does not exist there.
- Worst of the three, and the reason this is a correctness rule rather
  than a tidiness one: `AGENTS.md`'s `@agents/share/overview.md` include
  **silently fails to resolve** on such a filesystem. An agent session
  started on Linux loads the project instructions *without* the
  capability matrix — the document written specifically to stop this
  family being over-claimed. A broken link announces itself; a missing
  include does not.

Lowercase won because it is what 878 references already said, against
two. Renaming 34 directories was a far smaller and safer change than
rewriting 878 files, and it left the majority spelling — the one every
author had reached for unprompted — as the correct one.

**Nothing in the build ever read the path.** Every occurrence was prose:
markdown links and Rust doc comments. Verified across the tree before the
rename that there was no `COPY`, `ADD`, `include_str!`, or runtime path
read against it — which is why the defect was three weeks of dead links
and a silently-thinner agent context rather than a red build.

## 3. Renaming case-only paths

Worth writing down, because doing it the obvious way appears to work and
does not. On a case-insensitive filesystem `git mv AGENTS agents` is a
no-op the OS reports as success while the index keeps the old spelling.
The rename must go through a temporary name:

```sh
git mv path/to/AGENTS path/to/AGENTS__tmpcase
git mv path/to/AGENTS__tmpcase path/to/agents
```

Verify with `git ls-files`, never with `ls`: `ls` answers from the
filesystem, which is exactly the layer lying to you.

```sh
git ls-files | grep -cE '(^|/)AGENTS/'   # must be 0
```

## 4. How it is verified

`ls` cannot see this class of defect on the machine most likely to
introduce it, so the check reads the git index instead:

```sh
scripts/ci-check.sh docs        # runs repo-wide, not per crate
```

It fails when either half of the invariant breaks:

1. **No tracked path** contains an uppercase `AGENTS/` directory
   segment.
2. **No tracked file content** references one. A reference to a path that
   does not exist is the failure this rule exists to prevent, and it can
   reappear without any directory being renamed — someone simply typing
   the old spelling into a new link.

Both halves matter. Renaming the directories fixed the state; only the
second check keeps a new `AGENTS/…` link from re-introducing the problem
on a machine where nothing looks wrong.

## 5. Scope, precisely

**Covered** — every directory of agent reference docs:

- `agents/share/` — the repository-wide shared references
- `<entity>/agents/` — the entity-level umbrella docs
- `<subproject>/agents/` — per-crate and per-front-end topic guides

**Not covered** — these keep their conventional uppercase names:

- `AGENTS.md`, `CLAUDE.md` at any level
- `README.md`, `CHANGELOG.md`, and every other conventional file

**Link labels are display text, not paths.** A table cell reading
`[AGENTS](../person/agents/index.md)` is fine: the label names the
concept, the path obeys the rule. Only the path is checked, because only
the path can break.

## See also

- [`../AGENTS.md`](../AGENTS.md) — the root agent guide, whose `@include`
  motivated §2
- [`../agents/share/index.md`](../agents/share/index.md) — the shared
  reference docs this rule renamed
- [`../scripts/ci-check.sh`](../scripts/ci-check.sh) — the `docs` stage
- [`rust-msrv-n-minus-3.md`](rust-msrv-n-minus-3.md) — the other
  repository-wide convention with a CI stage behind it
