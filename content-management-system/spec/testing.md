# Testing

- **Pure-core unit tests** (DB-free, exhaustive): the editorial
  lifecycle's legal/illegal transition matrix (including reason
  requirements, terminal archive, reasoned restore, and
  distinct-approver); the translation lifecycle; block-document
  validation (allow-listed kinds, nest depth, caps, refusal paths
  named `blocks[i].kind`); HTML sanitization against the
  allow-list with a hostile corpus (script, event handlers,
  `javascript:` URLs, malformed nesting, SVG payloads); content-type
  schema validation and the additive / tightening / breaking
  compatibility classification; reference extraction from blocks and
  fields; path normalization and redirect resolution (hop cap, loop
  refusal, chain collapse); locale fallback chains including
  `strict_locales`; translation staleness arithmetic; delivery
  composition (one-hop expansion bound, renditions-that-exist);
  audience-rule evaluation including the context allow-list;
  sitemap/canonical derivation; every insight derivation including
  the honesty rules (null on zero denominator, percentile sample
  floor); revision diff.
- **Request tests** (Postgres, `#[ignore]`d): the authoring journey
  (declare a type → create an entry → save revisions → diff →
  restore → submit → approve → publish → delivery serves *that*
  revision, and a later save does **not** change the live payload);
  the concurrency pins (stale `base_revision_pid` ⇒ `409`;
  concurrent publish ⇒ one winner, no revision-number gap); the
  routing journey (slug change auto-redirects, old path `301`s, a
  loop is refused, unpublish leaves a redirect or `410`); the
  localization journey (fallback serves and *says* it fell back;
  `strict_locales` `404`s; a source publish makes the translation
  stale with the correct count); the asset journey (dedupe on
  identical bytes, MIME/sniff mismatch refused, delete refused while
  referenced, replace keeps references, alt-text gate blocks
  publish); the schedule sweep (idempotent rerun, skip on moved
  state); the delivery ETag `304`; unknown-pid `404`s (family
  lesson, pinned from day one).
- **Enforcement binary** (own process — the OnceLock lesson): the
  persona matrix (author `$sub` ownership vs another author's draft,
  editor publish scope by `resource.site`, translator locale scope
  and no-publish, admin-only type/site changes, `svc=true` delivery
  peer); the **public-delivery allow-list pins** — anonymous `GET`
  of a public site's published path succeeds; anonymous access to an
  unpublished revision, a restricted site, any non-`GET`, or any
  authoring path is refused with the flag on; preview tokens expire,
  are scoped to one revision, and are refused for a different one;
  the `mask` obligation hides unpublished bodies and reviewer
  identity.
- **Front-end**: vitest for the API client path map, the block-editor
  model transforms, the diff renderer, and the staleness/countdown
  formatters; Playwright over a `page.route`-stubbed API
  (contract-mirroring, unstubbed = 404-loud).
- Seed task: a synthetic site (~3 content types, ~40 entries across
  3 locales with a fallback chain, revisions in every workflow
  state, ~25 assets with renditions and one deliberate orphan, a
  menu, redirects including one chain, two stale translations, and
  a handful of planted content-health findings) — synthetic data
  only, no real copy, no real imagery.
