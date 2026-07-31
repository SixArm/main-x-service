# Roadmap

Beyond the v1 queue ([tasks.md](tasks.md)):

- **SVG support** — a purpose-built SVG sanitizer (element/attribute
  allow-list over a real XML parser, external references stripped),
  plus attachment disposition and a restrictive CSP on delivery. v1
  refuses SVG outright rather than pretending an HTML sanitizer covers
  it ([assets.md](assets.md)).
- **Rendition production worker** — actual image transcoding behind
  the declared-rendition seam ([assets](assets.md)), with the
  decoder-on-hostile-bytes hardening that deserves its own round
  (sandboxed decode, dimension/pixel caps, format allow-list).
- **Machine-translation seam** — a provider behind the translation
  workflow, with machine output marked as such and never presented
  as a human translation ([localization](localization.md)).
- **Translation memory & glossary** — reuse and terminology
  enforcement across variants.
- **Content search** — Tantivy over published and draft content
  (the family's six-crate pattern), replacing v1 `ILIKE` listing
  filters; near-duplicate content detection on top of it.
- **Scheduled content bundles / release trains** — publish a set of
  variants atomically ("the campaign goes live together"), an
  extension of the per-variant schedule.
- **Content type migrations** — a guided migrate-entries operation
  for `tightening` and `breaking` schema changes, beyond v1's
  `needs_migration` reporting ([authoring](authoring.md)).
- **Bulk import / export** — adopt the family contract
  ([bulk-import-export](../../agents/share/bulk-import-export.md))
  for content and asset migration from an existing CMS; the stable
  key is `(site, content_type, entry key, locale)`.
- **Link checking beyond references** — external URL health, on a
  non-redirecting fetch client (the family SSRF rule).
- **CDN integration recipes** — purge-on-publish webhook payloads
  and cache-key guidance for the common edges.
- **Entity-ref promotion to link-graph edges** — if editorial
  references should become real cross-service edges, with the
  governance question answered first
  ([integrations](integrations.md)).
- **A/B or scheduled variants of a region** — deliberately deferred;
  it is where personalization starts wanting a visitor identity, and
  that trade needs a privacy decision, not a feature flag
  ([delivery](delivery.md)).
- **Cross-app bridges** — CRM (published campaign landing content),
  WPM (job postings from requisitions), course/event registries
  (generated listing pages from registry records).
