# Purpose

## Problem

Content lives everywhere except where it can be managed. Copy sits
in documents, images sit in shared drives under names nobody can
search, "who approved this?" is answered from memory, the same
paragraph is published in three places and corrected in one, a
translated page silently rots when the English source is edited,
a page is deleted and every link to it breaks, and publishing
anything requires a developer.

A CMS exists so that people who are not developers can create,
manage, and publish digital content — with a record of who changed
what, an ability to undo it, and a delivery surface that can feed a
website, an app, and a screen from the same source.

## What CMS provides

### 1. Content modelling & authoring

- **Operator-defined content types** — an admin declares the fields
  an article, a page, or an event listing has (text, rich text,
  media, reference, date, choice, boolean, number), with validation
  rules, rather than a developer changing a schema.
- **Structured authoring** — content is a **block document** (typed
  blocks: heading, paragraph, list, quote, image, embed, callout),
  not a blob of HTML. It renders anywhere and can be validated.
- **References** — entries link to other entries and to assets as
  typed references, so "what would this deletion break?" is
  answerable.
- **Revisions** — every save writes an append-only revision;
  history is a diff you can read and restore from.

### 2. Digital asset management

- **One library** — images, video, audio, documents — with
  checksum-addressed storage, size and type limits, and metadata
  (title, alt text, caption, credit, tags, licence).
- **Renditions** — declared derived variants (thumbnail, wide,
  square) so a channel asks for the size it needs.
- **Usage tracking** — where each asset is used; deleting a
  referenced asset is refused, not silently broken.

### 3. Editorial workflow & governance

- **A lifecycle everyone can see** — `draft → in_review →
  approved → published → archived`, plus unpublish and a reasoned
  restore, enforced by a pure-core state machine.
- **Review and approval** — submitting for review assigns a
  reviewer; approval is recorded with the actor and time; the
  published thing is a specific revision, not "whatever is latest".
- **Scheduling** — publish or unpublish at a stated time, executed
  idempotently by a job.
- **Roles as policy** — author, editor, translator, admin, and the
  machine delivery peer are ABAC attribute profiles, not hard-coded
  role enums ([auth.md](auth.md)).

### 4. Localization & translation

- **Locale variants** — one entry, one variant per locale, each
  with its own revisions and workflow state.
- **Fallback chains** — a site declares `fr-CA → fr → en`; delivery
  states which locale actually served, never silently pretends.
- **Translation staleness** — when the source variant advances past
  the revision a translation was made from, the translation is
  flagged stale. Rot becomes visible instead of invisible.

### 5. Delivery, presentation & SEO

- **Headless delivery** — published content is served as structured
  JSON at a stable route, ETag-conditional, so a website, app, or
  screen consumes the same source ([delivery.md](delivery.md)).
- **Templates as contracts** — a template declares named regions a
  channel fills; the service renders no HTML
  ([design.md](design.md) CMS-D6; the family runs backend-only
  services with no template tier).
- **Routing that survives editing** — unique paths per site and
  locale; renaming a slug creates a redirect automatically; redirect
  chains are bounded and loop-free.
- **SEO artifacts** — per-entry metadata, canonical URLs, robots
  directives, and a generated `sitemap.xml` derived from what is
  actually published.
- **Personalization without surveillance** — declarative rules over
  **request context** (locale, channel, declared audience tag),
  never a per-visitor profile.

### 6. Content insights

- **Content health** — stale content, entries stuck in review,
  missing alt text, missing SEO metadata, orphan assets, broken
  internal references, stale translations.
- **Editorial throughput** — submissions, approvals, publishes,
  time-in-state per period and per actor — derived by arithmetic
  from recorded facts, never typed in.

## Goals

| Goal | Measure |
|---|---|
| Non-developers publish | content types, entries, media, and routes are all data, changed through the API |
| Nothing is lost | every save is an append-only revision; restore is a new revision, never a rewrite |
| Publishing is deliberate | a publish names a revision; scheduling and approval are recorded facts |
| Translations do not rot silently | staleness derived from source-revision drift, surfaced in insights |
| Links do not break | slug changes auto-redirect; referenced assets and entries cannot be deleted out from under a reference |
| The published surface is safe | structured blocks, sanitized on write, escaped at delivery; published-only reads |
| Numbers agree | every insight is one pure-core derivation with unit tests |

## Non-goals

- Not an identity registry — worker / organization services own who
  people and bodies *are*.
- Not a visitor analytics or tracking platform — no reader
  identities, no profile store, no behavioural targeting.
- Not a template/rendering engine — the family's services are
  backend-only ([loco.md](../../agents/share/loco.md)); delivery is
  structured JSON and the channel renders it.
- Not a plugin runtime — extension points are declared webhooks, not
  in-process third-party code ([design.md](design.md) CMS-D12).
- Not a web server / CDN / e-commerce platform.
