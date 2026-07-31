# Scope

## In scope (v1)

- **Sites** (a delivery namespace: locales, fallback chain,
  visibility) and operator-defined **content types** with declared
  field schemas and validation rules.
- **Entries** as locale **variants**, each an append-only chain of
  **revisions** holding a structured **block document** plus typed
  fields; references to other entries and assets; restore-as-new-
  revision.
- **Assets**: upload with byte/type caps and checksum addressing via
  the family `ArtifactStore` seam (local / S3-compatible), metadata
  + tags + alt text, declared **renditions**, usage references, and
  refusal to delete while referenced.
- **Editorial workflow**: `draft → in_review → approved →
  published → archived` (+ unpublish, reasoned restore) as a
  pure-core machine; reviewer assignment; edit locks + optimistic
  concurrency (`409` on stale revision); scheduled publish /
  unpublish via an idempotent job.
- **Localization**: per-locale variants, declared fallback chains,
  translation request/complete workflow, derived **staleness**
  against the source revision.
- **Delivery**: published-only structured JSON at
  `/delivery/{site}/{locale}/{path}`, ETag-conditional with
  `as_of`; locale negotiation reporting the locale actually served;
  routing with unique paths, auto-redirect on slug change, bounded
  loop-free redirect chains; short-lived scoped **preview tokens**
  for unpublished revisions.
- **SEO artifacts**: per-entry metadata / canonical / robots,
  generated `sitemap.xml`, feed; **personalization** by declarative
  request-context rules only.
- **Insights**: content health and editorial throughput as pure-core
  derivations, ETag-conditional, `as_of`-stamped.
- **Family fixtures**: auth (PASETO + ABAC + `CMS_REQUIRE_AUTH`),
  audit, events (memory / outbox), OpenAPI, `Accepts-version`, OTLP,
  Podman.

## Out of scope (v1)

- **Server-side HTML rendering, themes, or a template engine** —
  the family runs backend-only services
  ([loco.md](../../agents/share/loco.md)); templates here are
  declared region contracts, and the channel renders.
- **In-process plugins / third-party code execution** — extension
  points are declared outbound webhooks
  ([design.md](design.md) CMS-D12).
- **Visitor tracking, reader accounts, comments, A/B testing, and
  behavioural personalization** — no reader identity exists here.
- **Machine translation, ML content scoring, generated copy** —
  translation is a human workflow with recorded status.
- **Image transcoding in v1** — renditions are *declared* and
  recorded; the pixel-pushing worker is a documented seam
  ([roadmap.md](roadmap.md)).
- **E-commerce, forms/submissions capture, email sending** — the
  CRM sibling owns campaigns and consent.
- **A CDN, edge cache, or web server** — delivery is an origin API
  with correct cache validators.
- **Matching / deduplication of content** — near-duplicate content
  detection is roadmap, and identity dedup is the registries' job.

## Boundary with the family

| Concern | Owner |
|---|---|
| Who an author / editor / translator *is* | worker-service |
| Who the publishing body *is* | organization-service |
| Login, sessions, tokens, attributes | authentication-service |
| Content, its revisions, workflow, routes, assets | **CMS (this project)** |
| Marketing campaigns, consent, sends | contact-relationship-management |
| Courses, events, places as *registered entities* | the entity registries |

An entry that *describes* a registered entity (a course page, an
event listing, a place profile) carries that entity's `EntityRef`
URN as a typed reference field. **CMS never becomes the registry**:
it does not mint course, event, or place identities, and a
reference is a pointer, not a copy — the copy is the editorial
content about it.

**Readers are not entities.** CMS holds no `person:` URN for an
audience member and stores no visitor identifier; personalization
reads request context only ([delivery.md](delivery.md)).
