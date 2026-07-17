# AGENTS.md — working agreements

A pocket guide for human and AI collaborators working in this
subproject. Read this **before** opening a PR.

## What this project is

A **SvelteKit browser client** for the
[Loco JSON API sibling](../patient-flow-service-with-rust/): the
ward whiteboard (including a wall-mounted **touchscreen/kiosk
mode** with large tap targets), stay detail, hospital-at-a-glance,
bed-request board, patient locate, and handover audit views. The
Svelte app owns no data; every page round-trips through the API.

> ⚠️ Demo software, not a regulated medical record. See
> [regulatory](../spec/regulatory.md).

## Ground rules

1. **Spec first.** The cross-cutting spec at
   [`../spec/`](../spec/index.md) is the single source of truth —
   especially [whiteboard](../spec/whiteboard.md) (the bed-card
   field set and actions) and [auth](../spec/auth.md). Task queue:
   [`../spec/tasks.md`](../spec/tasks.md) (PF-T15/T16 + later).
2. **Family front-end conventions.** SvelteKit 2, **Svelte 5 runes
   only** (no legacy stores/`$:`), TypeScript strict. Drift between
   front-ends is accepted — copy-adapt from a sibling (the
   portfolio front-end's operational views are the closest source);
   do not factor out a shared package.
3. **BFF auth.** The SvelteKit server holds the cookie session and
   exchanges it for short-lived PASETO tokens; **no token in browser
   JS, no localStorage credentials**. Mutations go through server
   routes with CSRF protection.
4. **Whiteboard honesty.** Every board view renders its `as_of`
   timestamp; polling via ETag/`updated_since`. Masked mode (no
   patient names) must be a first-class rendering path — corridor
   screens use it.
5. **No whiteboard-only writes.** Every card action calls an
   existing API mutation; the front-end invents no state.
6. **Tests.** vitest for the bed-card state × flags matrix;
   Playwright e2e against the service in stub mode
   ([testing](../spec/testing.md)).

## Running

```bash
npm install
npm run dev        # expects the Loco sibling (stub mode) on its default port
npm test           # vitest
npx playwright test
```
