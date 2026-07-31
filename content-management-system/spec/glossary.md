# Glossary

| Term | Meaning |
|---|---|
| **Asset** | A stored file (image, video, audio, document) with metadata, content-addressed by SHA-256 |
| **Block document** | The ordered list of typed blocks that forms a content body — structured, never raw HTML |
| **Content type** | An operator-declared field schema (what an "Article" has), versioned by `schema_version` |
| **Delivery** | The public, published-only structured-JSON read surface consumed by channels |
| **Entry** | One piece of content at identity level; its per-locale rows are variants |
| **Fallback chain** | The ordered locale list delivery walks when a variant is unpublished (`fr-CA → fr → en`) |
| **Headless** | The CMS stores and delivers structured content; the channel renders it (no server-side HTML here) |
| **Orphan asset** | An asset referenced by no non-archived revision — reported, never auto-deleted |
| **Preview token** | A short-lived token scoped to exactly one (variant, revision) for viewing unpublished content |
| **Reference** | A typed edge extracted from a revision to an entry, asset, or `EntityRef` — drives "where used" and delete refusal |
| **Rendition** | A declared derived variant of an asset (`thumb`, `wide`, …); v1 declares and records, production is a seam |
| **Revision** | An append-only snapshot of a variant's body; publishing points at one; restore writes a new one |
| **Route** | The unique published path of a routable variant, per site and locale |
| **Site** | A delivery namespace: locales, fallback chains, visibility, base URL |
| **Slug / path** | The human-readable URL segment / full normalized path; renaming auto-creates a redirect |
| **Stale translation** | A translated variant whose source has published revisions newer than the one it was translated from |
| **Template** | A declared region contract a channel lays out — not server-side markup |
| **Variant** | An entry in one locale; the unit of workflow, revisions, and publishing |
| **Where used** | The reverse reference index for an asset or entry |
