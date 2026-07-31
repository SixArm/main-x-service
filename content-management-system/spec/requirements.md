# Requirements

Numbered requirements with user stories and acceptance criteria.
IDs are stable; design decisions ([design.md](design.md)) and tasks
([tasks.md](tasks.md)) trace to them. The module map:
CMS-R1–R5 modelling & authoring, CMS-R6–R8 assets,
CMS-R9–R12 editorial workflow, CMS-R13–R15 localization,
CMS-R16–R20 delivery & SEO, CMS-R21 insights,
CMS-R22–R24 cross-cutting.

## CMS-R1 — Sites & templates

*As an admin I configure a delivery namespace before anyone writes
a word.*

- Site CRUD: `key`, name, optional `organization:` owner URN,
  `default_locale`, `locales[]`, per-locale `fallback_chain`,
  `strict_locales[]`, `visibility` (`public | restricted`),
  `base_url`, robots defaults.
- Template CRUD: named regions with allowed block kinds and
  min/max cardinality; the service renders no markup.
- Changing `visibility` takes effect on the next request; the
  change is audited.

## CMS-R2 — Content types

*As an admin I declare what an "Article" is, without a code
change.*

- ContentType CRUD with typed fields (`text`, `rich_text`,
  `number`, `boolean`, `date`, `datetime`, `choice`, `media`,
  `reference`, `entity_ref`, `url`, `geo`, `json`), per-field
  validation, `required`/`repeatable`, `routable`, `template_key`.
- Any field change bumps `schema_version`; the change is classified
  `additive | tightening | breaking`; a `breaking` change requires
  an explicit confirmation flag and a reason, and is audited.
- Existing revisions keep validating against the version they were
  written under; those a tightening would now reject are reported
  as `needs_migration` (CMS-R21), never silently invalidated.

## CMS-R3 — Entries, variants & revisions

*As an author I write content and never lose a version of it.*

- Entry CRUD (site, type, stable `key`, `source_locale`, owner
  `worker:` URN); per-locale EntryVariant rows.
- Every save writes an append-only Revision (monotonic `number`,
  author, optional note, full body); revisions are never updated or
  deleted.
- `GET` a diff between any two revisions (block-level and
  field-level); restore writes a **new** revision recording
  `restored_from_pid`.
- A save states its `base_revision_pid`; a stale base is refused
  `409` returning the competing revision.

## CMS-R4 — Structured block authoring & sanitization

*As an author I write structured content that is safe to publish
anywhere.*

- Bodies are block documents (allow-listed block kinds, structured
  inline marks); nest depth, text length, and array cardinality
  capped per the family invariants; a violation is `422` naming the
  path (e.g. `blocks[3].kind`).
- Any accepted HTML (import, `embed`) is sanitized against an
  allow-list **at write time** and re-escaped at delivery; raw HTML
  is never stored and later trusted.
- Unknown block kinds and unknown field keys are refused, not
  dropped silently.

## CMS-R5 — References & "where used"

*As an editor I know what a change would break before I make it.*

- Every save extracts Reference rows for referenced entries,
  assets, and `EntityRef` URNs; extraction commits with its
  revision.
- `GET /assets/{pid}/usage` and `GET /entries/{pid}/usage` return
  referrers with their published state.
- Deleting a referenced asset or entry is refused `409` with the
  referrer list; force-delete requires a flag + reason and is
  audited.

## CMS-R6 — Asset upload & safety

*As an author I upload media without opening a hole in the site.*

- Upload via the family `ArtifactStore` seam (`local` default,
  `s3` behind the optional feature); SHA-256 content addressing
  deduplicates identical bytes.
- Byte cap per upload and per-site quota; declared MIME must match
  sniffed content; media types allow-listed; script-bearing formats
  refused (SVG only when sanitized); filenames are metadata, never
  paths; responses carry `nosniff` and never `text/html`.

## CMS-R7 — Asset metadata & renditions

*As an editor I can find a picture and know I may use it.*

- Metadata: title, alt text, caption, credit, licence, tags,
  intrinsic dimensions/duration where known; listing filters by
  kind/tag/text.
- Renditions are declared (`key`, dimensions, format, state);
  delivery reports only renditions that exist; production is a
  documented worker seam, not a v1 promise.

## CMS-R8 — Asset lifecycle

- Replace (new bytes, same asset identity) preserves metadata and
  references and emits `asset_replaced`.
- Delete refusal per CMS-R5; orphans are reported, never
  auto-deleted.
- **Alt text is required on image assets before a referencing
  variant may publish** (accessibility gate, CMS-R11).

## CMS-R9 — Editorial lifecycle

*As an editor I know exactly where every piece of content stands.*

- `draft → in_review → approved → published → archived`, plus
  reject, unpublish, and reasoned restore — a pure-core machine;
  illegal transitions `422` naming the current state.
- Reject / unpublish / archive / restore / lock-steal require a
  reason; a missing reason is refused.
- `require_distinct_approver` (site setting, default on) refuses
  self-approval.

## CMS-R10 — Publishing names a revision

- `publish` sets `published_revision_pid` to the current or an
  explicitly named earlier revision; delivery serves that revision
  and nothing else.
- Editing after publish does not change the delivered payload
  (pinned by test); `first_published_at` survives
  unpublish/republish.
- Unpublish leaves a redirect to a declared replacement or a `410`
  marker per site policy — never a bare `404`.

## CMS-R11 — Publish gates

- A publish is refused when: a required field is empty, a
  referenced image asset lacks alt text, a routable variant has no
  valid unique path, or a reference target is missing. Each refusal
  names the failing gate; gates are pure-core and unit-tested.

## CMS-R12 — Scheduling, locks & concurrency

- `scheduled_publish_at` / `scheduled_unpublish_at` on an approved
  variant; a `bg_pg` sweep applies the same transition, idempotent
  per (variant, scheduled_at), skipping and recording a variant
  whose state has moved.
- Advisory locks (`locked_by_ref`, `locked_until`, auto-expiring,
  stealable with a reason) are cooperative; optimistic concurrency
  (CMS-R3) remains authoritative.
- Publish/unpublish races serialize on the variant row; revision
  numbers have no gaps or duplicates.

## CMS-R13 — Locale variants

- One variant per (entry, locale), each with its own status,
  revisions, publish pointer, and schedule; locales validated
  against the site's declared set.

## CMS-R14 — Fallback & locale honesty

- Delivery walks the declared fallback chain to the first
  **published** variant, and reports `locale_requested`,
  `locale_served`, `fallback_applied` in the payload and
  `Content-Language` in the headers.
- `strict_locales[]` refuse fallback and return `404` instead.

## CMS-R15 — Translation workflow & staleness

- `request → in_translation → translated` per target variant,
  recording the source revision, requester, and optional due date;
  completion writes an ordinary revision stamped
  `translation_of_revision_pid` and then follows the ordinary
  editorial path.
- Staleness is **derived** (source published revision number >
  translated-from number), reports how many revisions behind and
  which, is never stored as an editable flag, and never
  auto-unpublishes unless the content type opts in.

## CMS-R16 — Delivery API

- `GET /delivery/{site}/{locale}/{path}` returns the composed
  document: variant body, one-hop reference summaries, assets with
  existing renditions, template region assignment; ETag-conditional
  with `as_of`.
- Published revisions only; no parameter, header, or policy can
  widen a public delivery read to an unpublished revision.
- Menus, `sitemap.xml`, and `robots.txt` served per site.

## CMS-R17 — Routing & redirects

- `UNIQUE (site, locale, path)` for current routes; paths
  normalized (leading slash, no trailing slash, single
  percent-decode, `..` refused).
- A slug change auto-creates a `301` from the old path.
- Redirect chains resolve within a bounded hop count (default 5);
  loops are refused at write time; over-long chains collapse to the
  final target on creation.

## CMS-R18 — Menus

- Menu + ordered MenuItem CRUD per site and locale (nesting,
  internal variant targets or external URLs, optional audience
  rule); a resolved menu tree is a delivery read that omits
  unpublished targets.

## CMS-R19 — SEO artifacts

- Per-revision SEO block (meta title/description, canonical,
  robots, Open Graph, sitemap hints).
- `sitemap.xml` derived from published, indexable, routable
  variants with `lastmod` and `hreflang` alternates; canonical
  resolution; a feed of recently published entries of a declared
  type.

## CMS-R20 — Personalization by request context

- AudienceRule CRUD with a declarative predicate over an
  **allow-listed** context (`locale`, `channel`, `audience_tag`,
  `preview`) — no cookies, IPs, user agents, referrers, or visitor
  identifiers.
- Evaluation is pure and **reported** (which rules matched);
  personalized responses vary their ETag by the consulted context
  and declare `Vary`.

## CMS-R21 — Content insights

- Content health: missing alt text, missing SEO, broken references,
  orphan assets, stale content, stale translations, stuck in
  review, unscheduled approvals, `needs_migration`, route hazards —
  each finding carrying its rule key and observed values.
- Editorial throughput: activity by state, time-in-state (median /
  p90 with a sample floor), per-actor counts (policy-scoped),
  publishing cadence, locale coverage + gap list, backlogs by age.
- All pure-core, ETag-conditional, `as_of`-stamped, with the
  honesty rules (numerator/denominator, `null` on zero
  denominator).

## CMS-R22 — AuthN/Z, public delivery & preview

- Family stack: offline PASETO verify, blanket `CMS_REQUIRE_AUTH`
  guard (default off, guard-all / deny-unless-public), shared ABAC
  engine.
- **Public delivery allow-list**: `GET`/`HEAD` on `/delivery/…`
  for `public` sites only, published revisions only; everything
  else requires a credential when the flag is on.
- Preview tokens: short-lived, scoped to one (variant, revision),
  read-only, revocable, `no-store`, audited on issue and use.
- Record-level attrs (`resource.owner` with `$sub`,
  `resource.site`, `resource.status`, `resource.locale`,
  `resource.content_type`); the five personas of
  [auth.md](auth.md) expressible as policy; the `mask` obligation
  hides unpublished bodies, revision notes, and reviewer identity.

## CMS-R23 — Audit, events & webhooks

- Every mutation audited + evented (family envelope,
  `CMS_EVENT_TRANSPORT` memory/outbox); reasoned actions carry
  reasons; publish records the revision published; scheduled
  executions record their trigger; sensitive reads (unpublished
  revisions, preview issue/use, audit queries) audited.
- Webhooks: per-site event subscriptions delivered over HTTPS with
  **no redirects followed**, signed bodies, timeouts, response-size
  caps, bounded retries, and a delivery log — driven from the event
  record.

## CMS-R24 — Family fixtures

- OpenAPI + Swagger, `Accepts-version` negotiation,
  `/metrics.prom`, OTLP tracing, health routes, Podman build,
  `#![forbid(unsafe_code)]`, clippy-pedantic, input caps → `422`,
  unknown-pid → `404`, stub-first upstream clients.
- A **synthetic seed** whose purpose is to make the derived views
  demonstrable: one deliberate instance of every content-health rule,
  planted on an entry whose key names the rule, alongside a corpus
  that stays mostly healthy. Synthetic data only — no real copy,
  imagery, or people. A rerun is a no-op, and a rule that stops firing
  fails the test suite rather than quietly emptying the demo.
