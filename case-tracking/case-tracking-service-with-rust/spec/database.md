# Database, migrations, stub mode & seed

> Part of the [Loco edition specification](index.md).

## Connection string

`config/development.yaml`:

```yaml
database:
  uri: postgres://postgres:postgres@localhost:5432/case_tracking_development
  auto_migrate: true
```

Override at runtime with `DATABASE_URL`. In production set
`auto_migrate: false` and run `cargo run --release -- db migrate` from a
controlled release step (currently a no-op because there are no
migrations, but the wiring stays).

## Migrations

No migration files. `migration/src/lib.rs` declares
`Migrator::migrations() -> vec![]`. The DB connection is opened at boot
so Loco can populate `AppContext::db` but no schema is created or read.

## Stub mode for offline / e2e runs

Set `USE_UPSTREAM_STUBS=1` before `cargo run -- start` to swap every
Main-X-Service client for an in-process `StubClient` and seed the stubs
with the same demo data the seed task creates. The initializer that does
this lives at
[`src/initializers/bootstrap_stubs.rs`](../src/initializers/bootstrap_stubs.rs).
The same `run_seed` function that the `Seed` task uses is invoked against
the stubs, so the data shape is identical.

This mode is used by the SvelteKit sibling project's Playwright suite
(see [Svelte testing](../../case-tracker-front-end-with-svelte/spec/testing.md))
and is useful for local UI iteration without standing up five upstream
services.

## Seed task

`src/tasks/seed.rs` registers the `seed` task with Loco. Running it:

- Registers **3 buildings, 4 rooms, and 5 cabinets** with the Main Place
  Service (`Cabinet A1`, `A2`, `B1`, `C1`, `Archive Cabinet 1`).
- Registers **6 patients** with the Main Patient Service (Alice Johnson,
  Bob Smith, Carol Williams, David Brown, Eleanor Patel, Frank O'Connor)
  keyed by NHS Numbers that all pass Modulus 11.
- Creates **9 folders** in the Main Thing Service with patient + cabinet
  snapshots packed into `keywords` + `identifiers`.
- Records **9 initial move events** in the Main Event Service (one
  synthetic "Folder created" per folder).

If any of the four external services is unreachable the task warns and
falls back to placeholder UUIDs (or skips that step) so the demo still
produces partial but useful state.

The task looks up by name / NHS Number / folder before inserting, so it
is safe to re-run.
