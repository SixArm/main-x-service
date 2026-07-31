# Authentication & authorization

The family stack unchanged
([authentication-sessions](../../agents/share/authentication-sessions.md),
[authorization-attributes](../../agents/share/authorization-attributes.md)):
cookie sessions + BFF for humans, offline PASETO v4.public for
services, blanket guard `CMS_REQUIRE_AUTH` (default **off** — the
family activation gate; activate before any real exposure), shared
ABAC engine.

## The one CMS-specific wrinkle: a deliberately public surface

CMS is the first consumer app whose *purpose* includes serving
anonymous readers. That is handled by a **narrow, explicit
allow-list**, not by weakening the guard
([security](../../agents/share/security.md) SEC-G5, guard-all /
deny-unless-public):

- Public without a credential: `GET` on
  `/delivery/{site}/…` **only for sites whose `visibility` is
  `public`**, plus `sitemap.xml` / `robots.txt` for those sites,
  plus the family health/docs/metrics paths.
- Everything else — every authoring, asset, workflow, insight, and
  audit path, and delivery for a `restricted` site — requires a
  credential when the flag is on.
- The allow-list is **method- and status-scoped**: `GET`/`HEAD`
  only, published revisions only. There is no query parameter, no
  header, and no policy rule that can widen a public delivery read
  to an unpublished revision.
- A site flipping `public → restricted` takes effect immediately on
  the next request (visibility is read per request, not cached at
  router construction, unlike the flag itself).

## Preview tokens

Unpublished content is visible to authorized editors, or through a
**preview token**: short-lived (default 15 min), scoped to exactly
one `(variant, revision)`, single-site, read-only, revocable, and
audited on issue and on use. It is bearer-shaped but deliberately
narrow — a preview link that is permanent, guessable, or valid for
"whatever is latest" is the classic CMS leak of embargoed content.
Preview responses are always `Cache-Control: no-store` and never
appear in a sitemap.

## Personas as policy (not code)

One API; five typical policy personas expressed over `attrs`:

| Persona | Typical rules |
|---|---|
| **author** | create entries; edit **own** drafts (`resource.owner = $sub`); submit for review; upload assets; read published |
| **editor** | review/approve/reject/publish/unpublish/schedule within their sites; edit any draft; steal locks with a reason |
| **translator** | claim and complete translation requests for their locales; edit target-locale variants only; no publish |
| **admin** | content types, sites, templates, menus, redirects, webhooks, breaking schema changes, force-delete |
| **delivery** (machine peer, `svc=true`) | read published delivery for the sites it is scoped to; nothing else |

## Record-level attributes

Handlers derive, after loading, `resource.owner` (the owning
`worker:` URN, enabling `$sub` self-scope), `resource.site`,
`resource.status`, `resource.locale`, and `resource.content_type`
for the second ABAC pass
([authorization-attributes](../../agents/share/authorization-attributes.md)
§9). This makes the useful policies expressible without new code:
"translators may write only `resource.locale ∈ their locales`",
"editors publish only within `resource.site`", "nobody but admin
touches `resource.content_type = policy-notice`".

The `mask` obligation redacts **unpublished bodies, revision notes,
reviewer identities, and editorial metadata** while leaving
structure (that an entry exists, its type, its published state)
visible — the shape a partner needs without the newsroom's
internals.

## Sensitivity map

| Data | Tier |
|---|---|
| unpublished / embargoed revisions, preview tokens | **high** — pre-publication disclosure is the CMS's signature harm; reads audited |
| review notes, rejection reasons, reviewer identity | medium — personnel-adjacent editorial judgement |
| audit trail, webhook secrets, asset store credentials | high — secrets never returned by any read ([security](../../agents/share/security.md) invariant 9) |
| published content, routes, menus, sitemap | low — public by design on a `public` site |
