# Regulatory posture

> ⚠️ **Demo software.** Not a production CMS; it serves no real
> public site, holds no real personal data, and ships synthetic
> content only.

## Observed by design

- **No visitor surveillance** — no reader identity, no profile
  store, no cookies read for personalization, no IP retained;
  audience rules see only an allow-listed request context
  ([delivery](delivery.md)). A system that cannot profile readers
  cannot leak profiles of them.
- **Accessibility as a gate, not advice** — alt text is required on
  image assets before a referencing variant may publish, and
  missing alt text is a standing content-health finding
  ([assets](assets.md), [insights](insights.md)). This is the one
  WCAG obligation a CMS can genuinely enforce at the source; the
  rest (contrast, focus order, heading semantics) belongs to the
  rendering channel, and this spec does not pretend otherwise.
- **Language honesty** — delivery always reports the locale actually
  served and never passes a fallback off as a translation
  ([localization](localization.md)); a stale translation is
  detected and surfaced rather than quietly served as current.
- **Publication is an accountable record** — who approved, who
  published, which revision, and when; unpublish and archive carry
  reasons; history is append-only and restore never rewrites it
  ([audit](audit.md)).
- **Safe by construction on the published surface** — structured
  blocks rather than stored HTML, sanitized on write, escaped at
  delivery; uploads allow-listed and sniff-verified; published-only
  reads behind a narrow allow-list
  ([authoring](authoring.md), [assets](assets.md),
  [auth](auth.md)).
- **Access control** — ABAC personas + the `mask` obligation over
  unpublished content; the `CMS_REQUIRE_AUTH` activation gate must
  be on before real exposure ([security](../../agents/share/security.md)
  §4).
- **Erasure without falsifying history** — a redaction blanks a
  revision body while preserving the row, its number, and its
  linkage ([audit](audit.md)).

## Production would additionally require

- **Accessibility conformance** against WCAG 2.2 AA (and, for a UK
  public-sector deployment, the Public Sector Bodies Accessibility
  Regulations 2018 with a published accessibility statement) — an
  audit of the *rendered channel*, which this service does not
  render.
- **Records management and retention** — statutory publication
  archives, retention schedules per content class, and (for public
  bodies) FOI/EIR-ready retrieval of what was published when.
- **GDPR / UK DPA** review of any content that carries personal
  data (staff biographies, case studies, photographs): lawful
  basis, image consent for identifiable people, subject access and
  erasure coordinated with the redaction path.
- **Copyright and licensing controls** on the asset library —
  licence terms recorded per asset are a v1 field, but expiry
  enforcement, usage restrictions, and takedown workflow are not.
- **Content Security Policy, CORS, and rate-limit posture** for the
  public delivery surface, plus a CDN/WAF in front of it.
- **Embargo handling review** — preview-token scope and audit are
  the technical control; an editorial embargo policy is not.

Tracked as production gates in [tasks.md](tasks.md).
