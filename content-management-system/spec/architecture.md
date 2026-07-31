# Architecture

```
 author / editor / translator / admin browser
        │  (cookie session; no token in JS)
        ▼
 content-management-system-front-end-with-svelte   (SvelteKit BFF)
        │  Authorization: Bearer v4.public.…
        ▼
 content-management-system-service-with-rust       (Loco: Axum + SeaORM + PostgreSQL)
        │                        │
        │                        └── ArtifactStore (local | s3) for asset bytes
        │  EntityRef lookups (read-only, cached, stub-able)
        ▼
 worker / organization / authentication services

 anonymous reader / app / kiosk ──▶ GET /delivery/{site}/… (public sites only)
```

## Service edition

Loco-idiomatic layout (the patient-flow / WPM / CRM shape):

```
src/
├── app.rs                loco Hooks (+ bg_pg workers: schedule
│                         sweep, sitemap build, link/reference
│                         check, webhook delivery)
├── controllers/          sites (sites/templates/menus/redirects),
│                         types (content types + compatibility),
│                         entries (variants/revisions/diff/restore),
│                         workflow (submit/approve/publish/schedule),
│                         assets (upload/metadata/renditions/usage),
│                         translation, delivery (public), preview,
│                         insights, audits, webhooks, docs, metrics
├── models/               helpers + _entities/ (SeaORM)
├── clients.rs            stub-first upstream display-name lookups
├── storage.rs            ArtifactStore seam (local | s3 feature)
├── rules/                pure core: editorial + translation state
│                         machines, block-document validation +
│                         sanitization, content-type schema
│                         validation + compatibility classification,
│                         reference extraction, path normalization +
│                         redirect resolution (loop/hop bounds),
│                         locale fallback + staleness, delivery
│                         composition, audience-rule evaluation,
│                         sitemap + SEO derivation, insight
│                         derivations, revision diff
├── auth.rs               offline PASETO + ABAC + personas + preview
│                         tokens + public-delivery allow-list + mask
├── streaming.rs          envelope + memory/outbox transports
├── validation.rs         caps + URN shapes + MIME/sniff → 422
└── openapi.rs            OpenAPI 3 doc
migration/                sea-orm-migration (crate root)
```

Key decisions (numbered in [design.md](design.md)): a **hybrid
schema** — declarative content-type schemas and entry field values
as JSONB, everything structural (variants, revisions, routes,
references, assets, menus) normalized and constraint-backed;
append-only revisions with publish pointing at one of them; every
lifecycle and every derivation in the DB-free pure core; blocks not
HTML, sanitized on write; delivery reads published revisions only
behind a narrow public allow-list; assets on the family
`ArtifactStore`; `bg_pg` jobs for schedules, sitemaps, reference
checks, and webhook delivery (no external broker); ETag-conditional
delivery and insights; family fixtures
(`#![forbid(unsafe_code)]`, clippy-pedantic, OTLP,
`Accepts-version`, Podman). **All-plural table names** (the loco
`create_table` pluralization lesson); `404` mapping at
`find_by_pid` call sites; enforcement tests in their own binary.

### Why hybrid persistence

The two halves have opposite requirements, so one storage style
cannot serve both honestly:

- **Content-type schemas and entry field values are
  operator-defined** — their shape is unknown at compile time. They
  are JSONB, validated at the boundary against the declared schema
  version ([authoring](authoring.md)).
- **Workflow, routes, references, revisions, assets** are
  invariant-heavy: uniqueness of a path per site and locale,
  monotonic revision numbers, delete refusal on a live reference,
  loop-free redirects. Those are constraints, and a constraint that
  lives only in application code over a JSON blob is a constraint
  you do not have.

## Front-end edition

SvelteKit 2 + Svelte 5 runes SPA + same-origin BFF proxy,
13-locale i18n from the start. Views: the entry list and block
editor (structured blocks, not a rich-text blob), the revision
history + diff + restore, the review queue and workflow actions,
the schedule calendar, the asset library with usage and alt-text
enforcement, the translation dashboard with staleness, the site
settings (locales, fallback chains, templates, menus, redirects),
the delivery preview panel, and the content-health + throughput
insights.
