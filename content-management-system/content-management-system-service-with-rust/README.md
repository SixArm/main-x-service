# Content Management System — Loco JSON API

A back-end **JSON API** for headless content management:
operator-defined content types, structured block authoring with
append-only revisions, a content-addressed asset library, an
editorial workflow (draft → review → approved → published →
archived) with scheduling and locks, locale variants with fallback
chains and derived translation staleness, a published-only delivery
API with routing, redirects and SEO artifacts, and derived
content-health and editorial-throughput insights. Implemented in
Rust on [Loco](https://loco.rs) (Axum + SeaORM + PostgreSQL). No
built-in UI and **no HTML rendering** — delivery is structured JSON,
and the
[Svelte sibling](../content-management-system-front-end-with-svelte/)
provides the authoring client.

> ⚠️ **Demo software.** Not a production CMS; it serves no real
> public site and ships synthetic content only. See
> [spec/regulatory](../spec/regulatory.md).

**Status: CMS-T1–T24 implemented (2026-07-31); the front-end
(T25/T26) is a separate, also-complete subproject** — see
[../content-management-system-front-end-with-svelte](../content-management-system-front-end-with-svelte/).
The declaration layer (sites, templates, content types with the
compatibility classifier), the authoring core (entries, per-locale
variants, append-only revisions with conflict refusal, block documents
sanitized on write, diff/restore, reference extraction and
delete-refusal), and the asset library (content-addressed uploads typed
from their bytes, metadata, declared renditions, replace, orphans, and
the alt-text publish gate), and the editorial workflow (lifecycle
transitions with reasons, publishing that names a revision, scheduling
with an idempotent sweep, advisory locks), and localization (fallback
resolution that reports what it served, strict locales, the translation
workflow, derived staleness), and routing + the **public delivery
surface** (addresses with automatic redirects, published-only
composition with honest ETags, sitemap and robots, personalization
without visitor tracking), and content insights (health findings that
name their rule, throughput with honest ratios), the record-level ABAC
pass (five personas, the `mask` obligation), preview tokens (scoped to
one revision, hashed at rest, audited on use), and outbound webhooks
(signed, non-redirecting, retried on a backoff) are live, and
`task seed` builds a synthetic corpus that demonstrates every one of
them. 231 DB-free unit tests + 60 request tests + the enforcement
matrix + 3 delivery tests pass against Postgres 18; clippy-pedantic and
`cargo deny` clean; live smoke verified. Remaining:
[production gates](../spec/tasks.md) only (activation, public-surface
hardening, the accessibility/records/rights review).

## Quick start

```bash
export DATABASE_URL=postgres://loco:loco@localhost:5432/content_management_system_service_development
cargo run -- db migrate && cargo run -- task seed && cargo run -- start
curl localhost:5150/api/sites | jq .
# Classify a proposed schema edit before making it:
curl -X POST localhost:5150/api/content-types/$PID/compatibility \
  -H 'content-type: application/json' -d '{"fields": [...]}' | jq .
# What is wrong with the seeded content, and which rule says so:
curl localhost:5150/api/sites/$SITE/insights/health | jq '.by_rule[] | {rule, count}'
```

`task seed` builds a **synthetic** demo site: 29 entries and 50
variants across three locales, revisions in every workflow state, 25
assets, a menu, a redirect chain, and one deliberate instance of every
content-health rule — each on an entry whose key names the rule
(`plant-stale-content`, `plant-no-alt-text`, …), so a finding can be
traced back to what caused it. Rerunning it is a no-op. Its asset rows
describe files that were never uploaded, so a seeded asset serves
metadata but not content; the task says so when it runs.

## What it answers (the design goal)

The questions the finished service exists to answer; the phases that
implement each are in [../spec/tasks.md](../spec/tasks.md).

- *What is live right now, and which revision is it?* — publishing
  names a revision; editing after publish changes nothing until the
  next publish
- *What would this change break?* — references are extracted on
  save, so "where used" is a lookup and deleting a used asset is
  refused
- *Which translations have rotted?* — staleness derived from
  source-revision drift, with the count of revisions behind
- *Why did that page 404?* — slug changes auto-redirect; redirect
  loops are refused at write time
- *What is wrong with our content?* — missing alt text, missing
  SEO, broken references, orphan assets, stuck reviews — each with
  the rule that found it

## Surface

**Live now:** sites (+ templates) · content types +
the compatibility dry-run · entries / variants / revisions +
diff + restore · "where used" for entries and assets · publish-check ·
assets (upload / content / metadata / renditions / replace / orphans /
quota) · workflow (transitions / schedule / lock / sweep) ·
what-is-published · locale resolution / translations / staleness /
locale coverage · routes / redirects / menus / audience rules ·
**public delivery** (`/delivery/{site}/{locale}/{path}`, menus,
`sitemap.xml`, `robots.txt`, per-locale Atom `feed.xml`) · insights (health / throughput /
backlog) · preview shares · **webhooks** (register / list / withdraw /
delivery log / dispatch) · audits · `/events/recent` · OpenAPI +
Swagger · `/metrics.prom` · `Accepts-version` negotiation.

That is the full v1 target surface — nothing is still planned inside
the service; only the [production gates](../spec/tasks.md) (auth
activation, public-surface hardening, the accessibility/records/
rights review) stand between this and real exposure.

Auth enforcement defaults **off** (`CMS_REQUIRE_AUTH` is the family
activation gate). When it is **on**, two checks apply: the blanket
guard on every request, and a **record-level** pass in the handlers
that read or write a specific variant — the second is what makes a
policy like "authors edit only their own drafts" or "translators write
only their locales" expressible without new code, because the guard
decides before any record is loaded. An allow may carry the `mask`
obligation, which redacts unpublished bodies, editorial notes, and
reviewer identities while leaving the structure visible; an obligation
this build cannot honour is **refused**, never ignored.

The public delivery allow-list is deliberately narrow: `GET`/`HEAD` only, sites whose `visibility` is `public` only,
published revisions only — the guard defers those reads to the
delivery controller, which checks visibility on every request, and an
enforcement test pins each case. A new site defaults to
`visibility = restricted`; upstream lookups default to **stub mode**;
events default to the in-memory transport; and the asset artifact store
defaults to **local** (`CMS_ARTIFACT_DIR`), with S3-compatible storage
behind the optional `s3` cargo feature.

Webhook **dispatch** needs the durable event transport
(`CMS_EVENT_TRANSPORT=outbox`) because deliveries are driven from the
event record; under the default in-memory transport it refuses with a
`422` naming that setting, rather than delivering a subset that would
disappear on restart. Run it on a schedule with
`cargo run -- task webhook_dispatch`.
