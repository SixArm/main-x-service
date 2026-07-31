# Module 2 — Digital asset management

## One library, content-addressed

Uploads go through the family **`ArtifactStore`** seam already
proven in care-pathway
([bulk-import-export](../../agents/share/bulk-import-export.md)
§12): `local` (confined to a base directory, `..` and absolute
paths refused) or `s3` (any S3-compatible endpoint, presigned
short-lived GETs). CMS adds no new storage abstraction and no
hand-rolled signing.

Every upload is **SHA-256 content-addressed**: the checksum is the
storage key, so re-uploading the same bytes deduplicates to one
stored object with a new `Asset` row's metadata (or returns the
existing asset, caller's choice via `on_duplicate`).

## Upload safety (the CMS attack surface)

An asset endpoint is an internet-facing file sink, so the rules are
non-negotiable ([security](../../agents/share/security.md)):

- **Byte cap** per upload and a per-site quota; both refused with
  `413`/`422`, never truncated.
- **Declared MIME must match sniffed content** (magic bytes); a
  mismatch is refused rather than trusted.
- **Allow-list, not deny-list**, of accepted media types; anything
  executable or script-bearing (`.html`, archives, executables) is
  refused. The v1 accepted set is PNG, JPEG, GIF, WebP, MP4, WebM,
  MP3, WAV, and PDF.
- **SVG is refused in v1** (revised 2026-07-30, with CMS-T8). The
  original position — accept it when sanitized — assumed a sanitizer
  this project does not have: SVG carries script through `<script>`,
  `on*` handlers, `<foreignObject>`, animated `href`s, and external
  entity references, and an HTML5 sanitizer is not an SVG sanitizer.
  Running SVG through the block-document sanitizer would *look* like
  protection while leaving a real attack surface — the same
  unverified-security-code trap this project refuses elsewhere. The
  refusal names the reason and points at a raster export. Accepting
  SVG needs a purpose-built sanitizer and its own round
  ([roadmap.md](roadmap.md)); when it lands, SVG is additionally
  served with `Content-Disposition` and a restrictive
  `Content-Security-Policy` regardless.
- **Filenames are metadata, never paths.** The stored key is the
  checksum; the original name is recorded for display only.
- **Delivery of an asset never executes it**: `X-Content-Type-
  Options: nosniff`, no `text/html` passthrough.

## Metadata and tags

`title`, `alt_text`, `caption`, `credit`, `licence`, `tags[]`,
plus intrinsic `width`/`height`/`duration_ms` where the format
declares them. **Alt text is required for image assets before a
referencing variant may publish** — an accessibility rule enforced
at the publish gate, not a nagging UI hint
([regulatory](regulatory.md)). Missing alt text is a content-health
finding ([insights](insights.md)).

## Renditions

A **Rendition** is a *declared* derived variant (`thumb`, `wide`,
`square`, …): dimensions, format, and a state
(`declared → produced | failed`). v1 records the declaration and
serves whichever renditions exist; **producing pixels is a
documented worker seam**, not a v1 promise
([roadmap](roadmap.md)). This is deliberate: an image pipeline
(decoders on attacker-supplied bytes) is a security-sensitive
subsystem that deserves its own round rather than being smuggled in
as a detail, and a delivery payload that names a rendition which
does not exist is worse than one that names only what does.

Delivery therefore reports, per asset reference, exactly which
renditions are available — a channel picks; it never guesses a URL
pattern.

## Usage and deletion

Every revision save extracts **Reference** rows
([authoring](authoring.md)), so each asset knows where it is used:

- `GET /assets/{pid}/usage` → the referring entries, variants, and
  whether those are published.
- **Deleting an asset referenced by any non-archived revision is
  refused** `409` with the referrer list. Force-delete requires an
  explicit flag, is audited with a reason, and marks the referrers
  as broken-reference findings.
- **Replace** (new bytes, same asset identity) writes a new
  `storage_ref` + checksum, keeps the metadata and the references,
  and emits `asset_replaced` — the correct operation for "fix the
  logo everywhere" that deletion-plus-reupload silently botches.

## Orphans

An asset referenced by nothing is an **orphan** — a derived
insight, never an automatic deletion. Storage reclamation is an
operator decision with a review list, because "unreferenced today"
and "safe to destroy" are not the same claim.
