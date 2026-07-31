# Domain model

Every owned record has a public UUID `pid`, timestamps, soft delete
(except append-only tables), audit + events. Upstream references are
EntityRef URNs. Text/array input caps per the family security
invariants. No money, no visitor identity.

## Site & configuration

**Site** — a delivery namespace.
`key` (slug, unique), `name`, `owner_ref` (`organization:` URN,
optional), `default_locale`, `locales[]` (ISO 639-1 per
[locales](../../agents/share/locales.md)), `fallback_chain` (per
locale, ordered), `visibility` (`public | restricted` — the
delivery allow-list gate, [auth.md](auth.md)), `base_url` (for
canonical/sitemap composition), `robots_default`.

**Template** — a declared presentation **contract**, not code.
`key`, `name`, `regions[]` (each: `key`, `label`, `allowed_block_
kinds[]`, `min`/`max`), `applies_to_type_keys[]`. The service
renders nothing; a channel uses this to know what it must lay out.

## Content model

**ContentType** — operator-defined shape.
`key` (unique per site or global), `name`, `description`,
`fields[]` — each `{ key, label, kind, required, repeatable,
validation }` where `kind` ∈ `text | rich_text | number | boolean |
date | datetime | choice | media | reference | entity_ref | url |
geo | json`; `routable` (does it get a path?), `template_key`,
`schema_version` (bumped on any field change; existing entries keep
validating against the version they were written under until
migrated).

**Entry** — one piece of content, identity-level.
`site_pid`, `content_type_key`, `type_schema_version`, `key`
(stable author-facing handle), `source_locale`, `owner_ref`
(`worker:` URN), `archived_at`.

**EntryVariant** — the per-locale row: the unit of workflow.
`entry_pid`, `locale`, `status` (`draft | in_review | approved |
published | archived`), `current_revision_pid` (latest saved),
`published_revision_pid` (nullable — what delivery serves),
`translation_of_revision_pid` (nullable — the source revision this
was translated from), `reviewer_ref`, `scheduled_publish_at`,
`scheduled_unpublish_at`, `locked_by_ref`, `locked_until`,
`published_at`, `first_published_at`.

**Revision** — **append-only**; never updated, never deleted.
`variant_pid`, `number` (monotonic per variant), `title`,
`blocks` (the block document, JSONB), `fields` (typed values keyed
by the content type's field keys, JSONB), `seo` (see below),
`author_ref` (`worker:` URN), `note` (why), `restored_from_pid`
(nullable), `created_at`. A restore writes a **new** revision whose
body is a copy — history is never rewritten
([audit.md](audit.md)).

**Block document** — an ordered list of typed blocks:
`heading | paragraph | list | quote | code | image | embed |
callout | divider | reference`. Inline marks are structured
(`strong`, `em`, `link`, `code`), **not** raw HTML
([authoring.md](authoring.md), [design.md](design.md) CMS-D5).

**Reference** — a typed edge out of a revision, extracted on save
so it is queryable: `from_revision_pid`, `kind`
(`entry | asset | entity`), `to_entry_pid` / `to_asset_pid` /
`to_entity_ref`, `field_key`. Drives "where used", delete refusal,
and broken-reference insights.

**SEO block** (embedded in a revision): `meta_title`,
`meta_description`, `canonical_url`, `robots` (`index|noindex`,
`follow|nofollow`), `og` (title/description/image asset ref),
`sitemap_priority`, `sitemap_changefreq`.

## Routing

**Route** — the published path of a routable variant.
`site_pid`, `locale`, `path` (normalized, leading slash),
`variant_pid`, `is_current`. `UNIQUE (site_pid, locale, path)
WHERE is_current`.

**Redirect** — `site_pid`, `locale`, `from_path`, `to_path`,
`status` (`301 | 302`), `reason` (`slug_change | manual |
unpublish`), `created_at`. Chains resolve to a final target with a
bounded hop count; loops are refused at write time
([delivery.md](delivery.md)).

**Menu** — navigation. `site_pid`, `locale`, `key`, ordered
**MenuItem** rows: `label`, `position`, `parent_item_pid`,
`target` (`variant_pid` | external `url`), `visibility_rule_pid`
(optional personalization rule).

## Assets

**Asset** — `site_pid` (nullable = shared), `kind`
(`image | video | audio | document | other`), `mime`,
`byte_size`, `checksum_sha256` (content address; identical upload
dedupes), `storage_ref` (the `ArtifactStore` reference),
`original_filename`, `title`, `alt_text`, `caption`, `credit`,
`licence`, `tags[]`, `width`/`height`/`duration_ms` (where known),
`uploaded_by_ref`.

**Rendition** — a declared derived variant.
`asset_pid`, `key` (`thumb | wide | square | …`), `width`,
`height`, `format`, `storage_ref` (nullable until produced),
`state` (`declared | produced | failed`). v1 declares and records;
production is a documented worker seam ([assets.md](assets.md)).

## Personalization

**AudienceRule** — declarative, request-context only.
`site_pid`, `key`, `name`, `predicate` (JSON over the allow-listed
context keys: `locale`, `channel`, `audience_tag`, `preview`),
`active`. Evaluated by the pure core at delivery; **no visitor
profile, no cookies read, no IP retained**
([design.md](design.md) CMS-D11).

## Extension points

**Webhook** — `site_pid`, `event_kinds[]`, `url` (HTTPS, no
redirects followed), `secret_ref`, `active`, delivery attempts +
last status. The only extension mechanism; no in-process plugins
([design.md](design.md) CMS-D12).

## Derived views (never stored as editable data)

Delivery document composition (variant + fallback + references +
menus + template regions); locale actually served; translation
staleness; content health (missing alt text / missing SEO / stale
content / stuck in review / orphan assets / broken references);
editorial throughput (submitted / approved / published per period,
time-in-state); sitemap contents; "where used" for an asset or
entry. All ETag-conditional; all carry `as_of`.

## Event kinds

`entry_created`, `revision_created`, `variant_submitted`,
`variant_approved`, `variant_rejected`, `variant_published`,
`variant_unpublished`, `variant_scheduled`, `variant_archived`,
`revision_restored`, `translation_requested`,
`translation_completed`, `asset_uploaded`, `asset_replaced`,
`asset_deleted`, `rendition_produced`, `route_changed`,
`redirect_created`, `menu_updated`, `content_type_changed`,
`site_configured`.
