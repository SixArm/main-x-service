# Contributing

Thank you for your interest. Contributions of code, documentation,
issue reports, and feedback are welcome.

## Ground rules

- **Spec-driven development.** Each subproject's `spec/` directory is
  its single source of truth. A behavioural change is a **three-part
  change** — spec edit + code edit + test edit — landed together, with
  the subproject's CHANGELOG updated. See
  [AGENTS.md](AGENTS.md) for the full working agreements.
- **Green gate.** Before submitting: `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, and `cargo test` must
  pass on every crate you touched (front-ends: `pnpm check` +
  `pnpm test`). Run any CI stage locally exactly as CI does:
  `scripts/ci-check.sh <stage> [crate]`.
- **Branch per change.** Branch from `main`, merge back with
  `--no-ff`; never commit directly to `main`.
- **Constraints that are not up for debate** (see
  [agents/share/rust-loco-stack.md](agents/share/rust-loco-stack.md)):
  Podman not Docker, Tokio not async_std, PostgreSQL not SQLite,
  PASETO cookie-session auth not JWT sessions, MSRV = stable N-3.
- **No real personal data, ever** — fixtures and examples are
  synthetic.

## How to contribute

1. Open an issue or email <joel@joelparkerhenderson.com> describing
   the change, especially before large work.
2. For a code change, find the owning subproject's `spec/` task queue
   (§13 in numbered specs, `spec/tasks.md` in topic specs) — most
   welcome work is already enumerated there or in the repo-root
   [tasks.md](tasks.md).
3. Submit the change with the three parts and the green gate evidence.

## Contributor expectations for AI tooling

See [AI_STATEMENT.md](AI_STATEMENT.md) — it binds contributors as well
as the maintainer.

## Donations

There is no donation mechanism; the useful contribution is code,
review, or a well-written issue.
