# Database, migrations, stub mode & seed

> Part of the [Axum edition specification](index.md).

## No database

The tracker owns no tables. Its `axa` dependency has no database feature,
so it opens no connection, needs no `DB_URL`, and has no migrations.
`db` commands answer "this service has no database" (`src/app.rs`).
Settings come from the environment, as for every axa service
([`agents/share/axa.md`](../../../agents/share/axa.md)); there are no
config files.

## Stub mode for offline / e2e runs

Set `USE_UPSTREAM_STUBS=1` before `cargo run -- start` to swap every
Main-X-Service client for an in-process `StubClient` and seed the stubs
with the same demo data the seed task creates. The initializer that does
this lives at
[`src/initializers/bootstrap_stubs.rs`](../src/initializers/bootstrap_stubs.rs).
The same `run_seed` function that the `Seed` task uses is invoked against
the stubs, so the data shape is identical.

This mode is used by the SvelteKit sibling project's Playwright suite
(see [Svelte testing](../../case-folder-front-end-with-svelte/spec/testing.md))
and is useful for local UI iteration without standing up five upstream
services.

## Seed task

`src/tasks/seed.rs` registers the `seed` task with axa. Running it:

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
