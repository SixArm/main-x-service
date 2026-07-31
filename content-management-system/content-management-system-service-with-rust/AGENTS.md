# AGENTS.md — working agreements

A pocket guide for human and AI collaborators working in this
subproject. Read this **before** opening a PR.

## What this project is

A **back-end JSON API**, written in Rust on [Loco](https://loco.rs)
(Axum + SeaORM + PostgreSQL), for headless content management:
operator-defined content types, structured block authoring with
append-only revisions, a content-addressed asset library, the
editorial workflow with scheduling and locks, locale variants with
fallback and translation staleness, published-only delivery with
routing and SEO, and derived content-health insights. There is no
built-in UI — the
[Svelte sibling](../content-management-system-front-end-with-svelte/)
is the authoring client.

**Domain ownership.** CMS **owns content and editorial state** (its
own tables) but **references identities**: authors, editors, and
translators are worker-service records, a site's owning body an
organization-service record — always as EntityRef URNs
(`worker:<uuid>`), never duplicated. Content *about* a registered
entity carries that entity's URN as a pointer, and CMS never becomes
the registry. **Readers are not modelled at all**: no visitor
identity, no profile store. See the cross-cutting spec's
[scope boundary](../spec/scope.md).

> ⚠️ Demo software, not a production CMS. See
> [regulatory](../spec/regulatory.md).

## Ground rules

1. **Spec first.** The cross-cutting spec at
   [`../spec/`](../spec/index.md) is the single source of truth;
   this subproject's `spec/` adds stack detail only. A behavioural
   change is spec edit + code + tests in one PR. The live task queue
   is [`../spec/tasks.md`](../spec/tasks.md) (CMS-T* ids, traced to
   CMS-D*/CMS-R*).
2. **Family conventions.** Loco-idiomatic layout
   (`src/controllers/`), `#![forbid(unsafe_code)]`, thiserror,
   tracing + OTLP, OpenAPI/Swagger, header API versioning
   (`Accepts-version`), Podman not Docker, PostgreSQL not SQLite,
   `bg_pg` jobs, in-memory loco cache. See
   [rust-loco-stack](../../agents/share/rust-loco-stack.md).
3. **Pure core.** Both lifecycle machines (editorial, translation),
   block validation + sanitization, schema compatibility
   classification, reference extraction, path normalization +
   redirect resolution, locale fallback + staleness, delivery
   composition, audience-rule evaluation, SEO/sitemap derivation,
   and every insight live in DB-free `src/rules/` modules with
   exhaustive unit tests; controllers only wire them
   ([design](../spec/design.md) CMS-D4, CMS-D13).
4. **Never store HTML and trust it later.** Bodies are structured
   block documents; anything HTML-shaped is sanitized against an
   allow-list at **write** time and re-escaped at delivery
   (CMS-D5). Do not add a field that round-trips raw markup.
5. **Delivery reads published revisions only.** The composer must
   not be able to reach a draft, and the public allow-list stays
   narrow: GET/HEAD, `visibility = public`, published only
   (CMS-D7). Unpublished access is authenticated or via a preview
   token scoped to exactly one (variant, revision).
6. **History is append-only.** Revisions are never updated or
   deleted; restore writes a new revision; erasure redacts a body
   while preserving the row and its linkage (CMS-D3).
7. **Uploads are hostile input.** Byte caps, MIME sniff match,
   media-type allow-list (not deny-list), checksum addressing,
   filenames as metadata never paths, `nosniff` on the way out
   (CMS-D9).
8. **Known family gotchas.** loco `create_table` pluralizes table
   names (use already-plural names / explicit SQL);
   `ModelError::EntityNotFound` is NOT mapped to 404 (return
   `Error::NotFound` at `find_by_pid` call sites); enforcement tests
   need their own test binary (OnceLock caching).

## Running (target)

```bash
cargo run -- db migrate && cargo run -- task seed && cargo run -- start
cargo test                    # DB-free unit tests
cargo test -- --ignored       # request tests (needs Postgres)
```
