# Local CI/CD — run the gate on our own machines first

Goal: make the **local machine the primary place the full gate runs**, and
treat hosted CI (GitHub Actions, Codeberg Woodpecker) as a mirror and
backstop rather than the only place a check ever executes. Hosted CI is
unreliable and metered; the checks themselves are not — they are already
shell scripts.

| | |
|---|---|
| Status | Proposed (2026-11-04). Nothing below is implemented unless marked **exists**. |
| Tasks | [`tasks.md`](../../tasks.md) Phase 12 (`LCI-*`) |
| Theme | [`plan.md`](../../plan.md) Theme G |
| Builds on | [`scripts/ci-check.sh`](../../scripts/ci-check.sh), [`scripts/ci-crates.sh`](../../scripts/ci-crates.sh), [`scripts/ci-front-ends.sh`](../../scripts/ci-front-ends.sh), [`scripts/test-db.sh`](../../scripts/test-db.sh), [`.woodpecker.yml`](../../.woodpecker.yml), [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) |

## 1. Assessment — where we actually are

**What already works in our favour (all exist):**

- **One entry point.** Every CI step is `scripts/ci-check.sh <stage>
  [crate]`; GitHub and Woodpecker run byte-identical commands. The CI
  logic is not trapped in YAML — there is almost nothing to "port" to run
  it locally.
- **Change-aware selection.** `CI_CHANGED_SINCE=<ref>` already narrows the
  crate and front-end lists to what a diff affects, and fails *open*.
- **Local databases.** `scripts/test-db.sh` + a `compose.test.yaml` per
  service stand up the same Postgres 18 CI uses, under Podman.
- **Two pipelines already.** `origin` pushes to Codeberg and GitHub, so we
  already depend on neither being the only one.

**What the evidence says about hosted CI (measured this session, not
assumed):**

- A single push sat `queued` for **more than eight minutes** before any
  runner picked it up; `main` had been red for **many consecutive
  commits** before anyone looked.
- Four front-ends' Playwright smoke suites were failing on `main` for
  multiple commits. They reproduced locally in seconds and were fixed in
  one sitting once someone ran them — the failure was never a hosted-CI
  problem, it was that **nothing forced the suite to run before the push**.
  Hosted CI told us, late; a local gate would have told us before.
- One timing assertion (`care-pathway-matcher` `oversized_arrays_score_in_bounded_time`)
  failed under a full-tree run on a loaded machine and passed alone. A
  shared hosted runner is the *worst* place for wall-clock bounds.

**Cost and reliability, stated plainly:**

- GitHub announced a $0.002/min platform charge on **self-hosted** runners
  (December 2025) and then **postponed** it after backlash; reports
  disagree on whether it is shelved or merely delayed, and none found
  confirm anything after January 2026. Treat "self-hosted GitHub runners
  are free" as *currently true, not guaranteed*. Hosted-runner prices
  already include that charge. (Sources: [The Register](https://www.theregister.com/2025/12/17/github_charge_dev_own_hardware/),
  [Socket](https://socket.dev/blog/github-actions-pricing-whiplash).)
  **Checked 2026-11-04: this repository is public** (`gh repo view`).
  GitHub's standard hosted runners are free for public repositories (from
  GitHub's published terms, not re-verified here), so **today's monetary
  cost is probably nil** and the case for local CI rests on reliability,
  queue latency and earlier feedback, not on a bill. Cost becomes real
  if the repository goes private, if larger/arm runners are used, or if
  the self-hosted-runner charge returns — keep the design cost-robust,
  but do not oversell it as a saving.
- Codeberg's **Forgejo Actions** runners are open to everyone with default
  anti-abuse limits; hosted **Woodpecker** needs an approved per-account
  access request, a public free/libre repository, and frugal resource use
  (cap Rust builds with `-j`). Codeberg also allows linking a
  self-hosted Woodpecker instance. (Source:
  [Codeberg CI docs](https://docs.codeberg.org/ci/).)

**Honest limits of "local":**

- The development machine is **Apple Silicon (aarch64-darwin)**; hosted CI
  is **x86_64 Linux**. A local green is strong evidence, not proof:
  architecture-, libc- and OS-specific failures (musl builds, the
  `aws-lc-sys` C build, Chromium versions) only show on Linux. Parity
  needs a container (§3, option C) and, ideally, an amd64 run.
- A local result is **self-attested**. It proves nothing to a reviewer
  unless it is bound to a commit and published (§4).
- Local runs need discipline or a hook; otherwise they do not happen
  (see the red `main` above).

## 2. Principles

1. **The script is the contract.** Any runner — a laptop, a pre-push hook,
   a self-hosted agent, GitHub — invokes `scripts/ci-check.sh`. YAML only
   chooses *when* and *where*. New checks go into the script, never only
   into a workflow file. (Already the rule; this keeps it so.)
2. **Fail open on selection, fail closed on verdict.** Change-aware
   narrowing may widen to everything when unsure; a stage that cannot run
   (missing tool, no database) reports **skipped**, loudly — never green.
3. **Attest, don't assert.** A local pass is recorded against an exact
   tree (commit SHA + clean working tree + toolchain versions), not
   claimed in prose.
4. **Hosted CI stays, demoted.** It remains the independent second
   opinion on a different OS/arch, and the thing external contributors
   see. It stops being the *first* time a check runs.
5. **No new mandatory service.** Prefer what is already installed
   (Podman, Make, shell). Adding a long-lived server must buy something
   the shell cannot.

## 3. Options considered

| Option | What it is | Verdict |
|---|---|---|
| **A. Local gate script + git hook** | `scripts/ci-local.sh` runs a chosen stage set change-aware; `pre-push` hook runs the fast tier. Make target wraps it. | **Adopt first.** Zero infrastructure; would have caught every red e2e suite this session. |
| **B. Attested local results** | After a green full run, post a commit status (or sign a note) for that SHA so hosted branch protection / reviewers can see it. | **Adopt second.** Turns A from private habit into evidence. Needs a trust decision (§4). |
| **C. Container parity** | Run the same `ci-check.sh` inside the pinned CI image (`rust:1.97-bookworm`, `node:26-bookworm`) under Podman, optionally `--platform linux/amd64`. | **Adopt third.** Closes the Linux/arch gap. amd64 emulation on Apple Silicon is slow — use it for a nightly/full run, not every push. |
| **D. `woodpecker-cli exec`** | Run the existing `.woodpecker.yml` from a checkout. Official docs show `docker` and `local` backends; **no Podman backend is documented**. | **Evaluate** (LCI-6). Possible via Podman's Docker-compatible socket — **unverified**. Value: reproduces the real pipeline file, catching YAML drift. Secrets are passed explicitly, not pulled. |
| **E. Self-hosted Woodpecker/Forgejo runner** | An agent on an always-on box (home server/VPS) linked to Codeberg. | **Adopt when the above is stable.** Moves the metered/queued part off shared infrastructure while keeping the pipeline file. Costs a machine to run and secure. |
| **F. Self-hosted GitHub runner** | Register our own runner for `ci.yml`. | **Defer.** Free *today*, but exposed to GitHub's postponed-not-cancelled charge, and a runner on a public repo executing PR code is a security risk (§5). |
| **G. Dagger / Nix / Bazel** | A portable pipeline engine or hermetic build system. | **Reject for now.** `dagger` is installed locally but adds an engine dependency and a second pipeline language over scripts that already work; the problem here is *not running* checks, not *describing* them. Revisit if the scripts outgrow shell. |
| **H. `act` (run GitHub Actions locally)** | Executes workflow YAML in Docker. | **Reject.** Our workflows are thin wrappers over `ci-check.sh`; running the script directly is more faithful and cheaper. |

## 4. Design

### 4.1 Tiers

| Tier | When | Stages | Budget |
|---|---|---|---|
| **fast** | `pre-push` hook | `fmt`, `docs`, `clippy` + `test` on changed crates, front-end `check`/`vitest`/`lint` on changed projects | a few minutes; never blocks on Docker |
| **full** | on demand, before a release or a dependency bump | every stage in `ci-check.sh` incl. `deny`, `msrv`, `bench`, front-end `build`, enrolled Playwright suites (serial — port 4173 collides), `test-db` for enrolled crates | tens of minutes |
| **parity** | nightly or on demand | `full`, inside the CI container image, amd64 | long; unattended |
| **hosted** | every push | unchanged; second opinion | n/a |

Selection reuses `CI_CHANGED_SINCE`; the local entry point defaults it to
`origin/main`. A stage listed in the tier but unable to run **prints
`SKIPPED: <reason>` and the run is not recorded as a full pass.**

### 4.2 The record

A run writes a small JSON line per stage to an untracked directory
(`.ci-local/`, git-ignored): commit SHA, `git status --porcelain` clean
flag, stage, crate/front-end, exit code, duration, `rustc -V`, `node -V`,
`pnpm -V`, host triple. This is the unit B (attestation) publishes and the
thing that tells us how long each stage actually takes (the data we lack
for scheduling and for spotting flakes such as the timing test above).

### 4.2a The hook

`hooks/pre-push` runs the **fast** tier. It is opt-in:
`scripts/install-hooks.sh` points this clone's `core.hooksPath` at
`hooks/`, and `--uninstall` undoes it. The installer refuses to replace a
`core.hooksPath` it did not set, so a hook manager already in use is not
silently turned off.

- **Selection.** Each pushed ref is compared with the remote's current
  commit for it, so only what the push adds is checked. A new branch, or
  a remote commit this clone lacks, falls back to `origin/main`. A branch
  deletion runs nothing. Uncommitted edits are selected too, because
  `ci-local.sh` includes the worktree.
- **Budget.** `CI_LOCAL_BUDGET` seconds, default 600. The gate runs in its
  own process group, and past the budget the whole group is stopped and
  **the push is refused**: a check that did not finish proved nothing,
  and passing it would make the budget a way round the gate.
- **Escape.** `git push --no-verify`, or `CI_LOCAL_HOOK=0`, skips one
  push. Hosted CI still runs either way.

### 4.3 Attestation (option B) — the trust decision

Publishing "local CI passed for `<sha>`" lets branch protection and
reviewers rely on it, but means **the pusher vouches for themselves**. In
a single-maintainer repo that is acceptable; with more committers it is
not a control. Proposed posture:

- Post a `local-ci` commit status via the forge API **only from a clean
  tree at that exact SHA**, and name the tier (`fast`/`full`/`parity`).
- Do **not** make `local-ci` a required check in place of hosted CI.
  Require it *in addition*, so a hosted failure still blocks. The aim is
  earlier signal and fewer wasted hosted runs, not removing the
  independent check.
- Revisit when the committer set grows (GOVERNANCE.md).

### 4.4 Reduce hosted cost without weakening it

- Keep `concurrency` cancel-in-progress (exists). Add `paths-ignore` for
  docs-only changes where the `docs` stage already covers them.
- Run `bench` (compile-only), `fuzz` smoke and `evidence` on a schedule or
  `workflow_dispatch`, not every push, once they have been observed stable
  locally.
- Keep the change-aware matrix (exists, WEB-8). Verify the Woodpecker half,
  which its own header says is **unverified against a live runner**.
- Cache aggressively: a shared `sccache`/`kache` (a `kache` daemon is
  already running on the dev machine) and the pnpm store, so a 55-crate
  tree is not rebuilt cold. **Measure the hit rate before claiming a
  saving.**

## 5. Risks

- **False confidence.** Apple Silicon green ≠ Linux green. Mitigation:
  parity tier; hosted CI stays on.
- **Hook fatigue.** A slow or flaky hook gets bypassed with `--no-verify`.
  Mitigation: the fast tier has a hard time budget; the full tier is not a
  hook; flaky timing tests are fixed or moved off the gate (LCI-9).
- **Self-hosted runner security.** A runner that executes untrusted PR code
  on a machine holding credentials is a compromise path. Rules: runners on
  a dedicated machine/VM, no repository secrets for fork PRs, ephemeral
  containers per job, never on the development laptop. For a public
  repository, prefer Woodpecker/Forgejo runners that only run trusted
  branches by default.
- **Secrets in local runs.** `ci-check.sh` stages need none today; keep it
  that way. Any stage that gains a secret must say so in its header.
- **Silent skips.** The worst failure mode of local CI is a stage that
  quietly did not run. §4.1's `SKIPPED` rule and the record in §4.2 exist
  to make that visible.
- **Port and resource collisions.** Two Playwright suites in parallel share
  preview port 4173 and fail wholesale (known); the full tier runs
  front-end e2e serially. Postgres uses 5432 (`TEST_DB_PORT` to move it).

## 6. Open questions

- ~~Is the GitHub repository public?~~ Yes (2026-11-04). Still to confirm from the billing page: actual minutes used and any queue-time pattern.
- Does Podman's Docker-compatible socket satisfy `woodpecker-cli exec`'s
  docker backend on macOS (podman machine)? Unverified.
- Is a dedicated always-on machine available for option E, and who
  maintains it?
- Should attestation (§4.3) use forge commit statuses or signed git notes?
  Statuses are visible to branch protection; notes survive a forge move.
- amd64-under-emulation: is a nightly parity run on the dev machine
  tolerable, or does it need a real x86_64 host?

## 7. Non-goals

- Replacing hosted CI or removing `.github/workflows/ci.yml` /
  `.woodpecker.yml`. They stay as the independent second opinion.
- A new pipeline language or build system (see option G).
- Moving the code host. Codeberg + GitHub mirroring is unchanged.
