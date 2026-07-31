# Module 5 — Delivery, presentation & SEO

## Headless by decision, not by omission

The service delivers **structured JSON** and renders no HTML. The
family runs backend-only services with no template tier
([loco](../../agents/share/loco.md)), and the omnichannel promise —
one source feeding a website, an app, and a screen — is only
truthful if the payload is not already a web page. A **Template**
is therefore a *contract*: named regions with allowed block kinds
and cardinalities ([domain-model](domain-model.md)), which a
channel uses to know what it must lay out. Themes and CSS live in
the channel.

## The delivery API

```
GET /delivery/{site_key}/{locale}/{path}      → the composed document
GET /delivery/{site_key}/{locale}/menus/{key} → a resolved menu tree
GET /delivery/{site_key}/sitemap.xml          → generated from what is published
GET /delivery/{site_key}/robots.txt           → site policy + sitemap pointer
```

- **Published revisions only.** The delivery composer reads
  `published_revision_pid` and cannot reach a draft. There is no
  parameter that widens it; preview is a different, authenticated
  path ([auth](auth.md)).
- **ETag-conditional**, weak tag over the payload excluding
  `as_of`; `304` on match. Every response carries `as_of`,
  `locale_requested`, `locale_served`, `fallback_applied`, and the
  `template_key` + region assignment.
- **Composition** resolves, in the pure core: the variant (via the
  fallback chain), its block document, referenced entries (as
  summaries — one hop, no recursive expansion), referenced assets
  (with the renditions that actually exist,
  [assets](assets.md)), and the template's region contract.
- **Reference expansion is one hop, bounded.** A page referencing a
  page referencing a page returns summaries, not a graph walk; this
  is a DoS boundary as much as a design one
  ([security](../../agents/share/security.md) invariant 3).

## Routing

- A routable variant has a **Route**: `UNIQUE (site, locale, path)
  where is_current`. Paths are normalized (leading slash, no
  trailing slash, lowercased, percent-decoded once, `..` refused).
- **Changing a slug auto-creates a `301`** from the old path. This
  is the default because the alternative — an editor silently
  breaking every inbound link and bookmark by renaming a page — is
  the single most common self-inflicted CMS injury.
- **Redirects resolve with a bounded hop count** (default 5) and
  **loops are refused at write time**, not discovered at request
  time. A chain that would exceed the cap is collapsed to its final
  target on creation.
- **Unpublish** leaves either a redirect to a declared replacement
  or a `410 Gone` marker per site policy — never a bare `404` with
  no record that the page existed.

## SEO artifacts

Derived from what is actually published, never hand-maintained:

| Artifact | Derivation |
|---|---|
| `sitemap.xml` | every published, `index`-able, routable variant per locale, with `lastmod` = published revision time, plus `hreflang` alternates from the entry's other published locales |
| canonical URL | the revision's `canonical_url`, else `site.base_url` + the current route |
| `robots.txt` | site defaults + the sitemap pointer |
| meta / Open Graph | the revision's SEO block, with the OG image resolved to a real rendition |
| feed | the N most recently published entries of a declared type |

Missing `meta_title` / `meta_description` / OG image, and a
`noindex` page that is nevertheless in a menu, are **content-health
findings** ([insights](insights.md)) — surfaced to editors, not
silently defaulted into plausible-looking rubbish.

## Personalization without surveillance

An **AudienceRule** is a declarative predicate over an
**allow-listed request context**: `locale`, `channel` (`web | app |
screen | feed`), `audience_tag` (a value the *channel* asserts,
e.g. a kiosk's declared location), and `preview`. Rules select
which menu items or which region variants appear.

What this deliberately excludes: cookies, IP addresses, user
agents, referrers, behavioural history, and any per-visitor
identifier or profile store ([design](design.md) CMS-D11). CMS
holds **no reader identity** ([scope](scope.md)). A rule engine
fed only by what the caller states cannot become a tracking system
by accident, and personalization that requires profiling is a
different product with a different privacy review — not a config
flag here.

Evaluation is pure, deterministic, and **reported**: the payload
lists which rules matched, so a puzzled editor can see why a
visitor got what they got. Personalized responses vary the ETag by
the context that was actually consulted, and declare `Vary`
accordingly — a personalized page served from a cache keyed on URL
alone is a data-leak mechanism.

## Caching posture

Origin correctness only: strong `as_of`, honest ETags, `Vary` on
the consulted context, and `Cache-Control` per site policy
(`public` for public sites, `no-store` for restricted ones and for
every preview response). CMS is not a CDN and does not pretend to
purge one; an invalidation **webhook** on publish is the seam a CDN
integrates through ([domain-model](domain-model.md)).
