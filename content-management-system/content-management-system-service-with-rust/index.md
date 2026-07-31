# Content Management System service — documentation index

A headless content management API: operator-defined content types,
structured block authoring with append-only revisions, a
content-addressed asset library, an editorial lifecycle with
scheduling and locks, locale variants with fallback, a published-only
public delivery surface, and derived content insights.

> ⚠️ **Demo software.** Not a production CMS; synthetic content only.
> See [regulatory](../spec/regulatory.md) and the production gates in
> [tasks](../spec/tasks.md).

## Start here

- **[README.md](README.md)** — what this is, status, surface summary.
- **[../spec/](../spec/index.md)** — the cross-cutting specification
  (**the single source of truth**: domain model, the six modules,
  auth, audit).
- **[spec/](spec/index.md)** — this edition's stack-specific spec.
- **[AGENTS.md](AGENTS.md)** — working agreements for contributors.
- **[CHANGELOG.md](CHANGELOG.md)** — Keep a Changelog format.
- **The task queue** — [../spec/tasks.md](../spec/tasks.md).

## Quick start

```bash
export DATABASE_URL=postgres://loco:loco@localhost:5432/content_management_system_service_development
cargo run -- db migrate      # create the schema
cargo run -- task seed       # a synthetic demo site (see below)
cargo run -- start           # http://localhost:5150
```

Then open `http://localhost:5150/api-docs/openapi.json` for the full
endpoint list, or follow the tutorial below.

### What the seed gives you

`task seed` builds a synthetic corpus — 29 entries, 50 variants across
`en`/`fr`/`fr-CA`, revisions in every workflow state, 25 assets, a
menu, a redirect chain, and **one deliberate instance of every
content-health rule**, each on an entry whose key names the rule
(`plant-stale-content`, `plant-no-alt-text`, …). Rerunning it is a
no-op.

Its asset rows describe files that were never uploaded, so a seeded
asset serves metadata but not content. The task says so when it runs;
that is the one place the demo diverges from a real upload.

## Tutorial: from nothing to a published page

### 1. Declare a site

A **site** is a delivery namespace: its locales, their fallback
chains, and whether the world may read it.

```bash
SITE=$(curl -sX POST localhost:5150/api/sites \
  -H 'content-type: application/json' -d '{
    "key": "handbook", "name": "Handbook",
    "default_locale": "en", "locales": ["en", "fr"],
    "fallback_chains": { "fr": ["en"] },
    "base_url": "https://handbook.example.test"
  }' | jq -r .pid)
```

A new site is `restricted` — nothing is world-readable until somebody
decides it should be.

### 2. Declare a content type

A **content type** is the field schema an entry is written against.

```bash
TYPE=$(curl -sX POST localhost:5150/api/sites/$SITE/content-types \
  -H 'content-type: application/json' -d '{
    "key": "article", "name": "Article", "routable": true,
    "fields": [
      { "key": "summary", "label": "Summary", "kind": "text",
        "validation": { "max_len": 300 } },
      { "key": "section", "label": "Section", "kind": "choice",
        "validation": { "options": ["news", "guide"] } }
    ]
  }' | jq -r .pid)
```

**Before changing a schema, ask what it would break:**

```bash
curl -sX POST localhost:5150/api/content-types/$TYPE/compatibility \
  -H 'content-type: application/json' -d '{ "fields": [ ... ] }' \
  | jq '{level, changes}'
```

The answer is `additive`, `tightening`, or `breaking`. Breaking changes
need explicit confirmation — the point is that you find out before the
content does.

### 3. Write an entry

Bodies are **blocks**, never HTML.

```bash
ENTRY=$(curl -sX POST localhost:5150/api/sites/$SITE/entries \
  -H 'content-type: application/json' -d '{
    "key": "getting-started", "content_type_key": "article",
    "title": "Getting started",
    "blocks": [
      { "kind": "heading", "level": 2, "text": "Getting started" },
      { "kind": "paragraph", "text": "Everything begins somewhere." }
    ],
    "fields": { "summary": "A first page.", "section": "guide" }
  }' | jq -r .pid)
```

Anything HTML-shaped inside a block is sanitized against an allow-list
at **write** time, and the response reports whether that happened
(`blocks_sanitized`) rather than quietly changing your text.

### 4. Edit it, safely

Every save names the revision it was based on. That is what turns a
concurrent edit into a refusal instead of a silent overwrite:

```bash
BASE=$(curl -s localhost:5150/api/entries/$ENTRY/variants/en/revisions \
  | jq -r '.[0].pid')

curl -sX POST localhost:5150/api/entries/$ENTRY/variants/en/revisions \
  -H 'content-type: application/json' -d "{
    \"base_revision_pid\": \"$BASE\",
    \"title\": \"Getting started\",
    \"blocks\": [{ \"kind\": \"paragraph\", \"text\": \"Revised.\" }]
  }" | jq '{revision_pid, number}'
```

Sending a stale `base_revision_pid` returns **`409`**. Do not retry it:
somebody else's work is in the revision that won. Compare first —

```bash
curl -s localhost:5150/api/revisions/$FROM/diff/$TO | jq .diff
```

History is append-only. **Restore writes a new revision** rather than
rewinding: `POST .../variants/en/restore` with a `revision_pid` and a
`reason`.

### 5. Give it an address, then publish

```bash
curl -sX PUT localhost:5150/api/entries/$ENTRY/variants/en/path \
  -H 'content-type: application/json' -d '{"path": "/getting-started"}'
```

Ask what publishing would refuse, before trying:

```bash
curl -s localhost:5150/api/entries/$ENTRY/variants/en/publish-check \
  | jq '{ready, blockers}'
```

Each blocker names its `rule` and a `remedy`. Then move it through the
lifecycle:

```
draft ──submit──▶ in_review ──approve──▶ approved ──publish──▶ published
  ▲                   │                                            │
  └────── reject ─────┘                └──── unpublish ────────────┘
```

```bash
curl -sX POST localhost:5150/api/entries/$ENTRY/variants/en/transition \
  -H 'content-type: application/json' -d '{"action": "publish"}'
```

`reject`, `unpublish`, `archive`, and `restore` **require a `reason`**
— the service refuses without one, because those are the transitions
somebody will later ask about.

Publishing names a revision. Editing after publish changes nothing a
reader sees until the next publish.

### 6. Read it as the world does

Delivery is published-only. Open the site first — `PUT` **replaces**
the site, so send the whole payload; a partial body is a `400`:

```bash
curl -sX PUT localhost:5150/api/sites/$SITE \
  -H 'content-type: application/json' -d '{
    "key": "handbook", "name": "Handbook",
    "default_locale": "en", "locales": ["en", "fr"],
    "fallback_chains": { "fr": ["en"] },
    "base_url": "https://handbook.example.test",
    "visibility": "public"
  }'

curl -s localhost:5150/delivery/handbook/en/getting-started \
  | jq '{locale_requested, locale_served, fallback_applied, published_at}'
```

The response names the locale it **actually served**. Note what it
does *not* do: an address exists per locale, so asking for a locale
that has no published page at that path is a **`404`**, not a silent
substitution of another language. Fallback resolution is a separate,
explicit question:

```bash
curl -s localhost:5150/api/entries/$ENTRY/resolve/fr | jq .resolution
# { "locale_requested": "fr", "locale_served": "en",
#   "fallback_applied": true, "chain_walked": ["en"] }
```

That answer reports the chain it walked, so "why am I seeing English?"
has an answer rather than a guess.

Also on the delivery surface:

```bash
curl -s localhost:5150/delivery/handbook/sitemap.xml
curl -s localhost:5150/delivery/handbook/robots.txt
curl -s localhost:5150/delivery/handbook/en/feed.xml      # Atom 1.0
curl -s localhost:5150/delivery/handbook/en/menus/primary
```

### 7. Translate it

```bash
curl -sX POST localhost:5150/api/entries/$ENTRY/variants \
  -H 'content-type: application/json' -d '{"locale": "fr"}'

curl -s localhost:5150/api/entries/$ENTRY/translations \
  | jq '.locales[] | {locale, status, staleness}'
```

Staleness is derived, and reports **how many source revisions behind**
a translation is — not a bare badge. It also distinguishes *unknown*
(no recorded source revision) from *up to date*, because telling a
translator their page is fine when nobody knows is the failure worth
avoiding.

### 8. Ask what is wrong

```bash
curl -s localhost:5150/api/sites/$SITE/insights/health \
  | jq '.by_rule[] | {rule, count, explanation}'
```

Ten rules, each finding carrying the rule that produced it. Also
`/insights/throughput?days=30` (editorial rates, with numerator and
denominator shown) and `/insights/backlog` (what is waiting).

## Endpoint map

| Area | Endpoints |
|---|---|
| Sites & templates | `/api/sites`, `/api/sites/{pid}`, `.../templates` |
| Content types | `.../content-types`, `/api/content-types/{pid}/compatibility` |
| Entries & revisions | `.../entries`, `/api/entries/{pid}`, `.../variants/{locale}/revisions`, `/api/revisions/{pid}`, `/api/revisions/{from}/diff/{to}` |
| Workflow | `.../transition`, `.../publish-check`, `.../schedule`, `.../lock`, `/api/schedules/sweep` |
| Assets | `.../assets`, `/api/assets/{pid}/content`, `.../usage`, `.../renditions`, `.../orphans`, `.../quota` |
| Localization | `.../variants`, `.../translations`, `.../resolve/{locale}`, `.../locale-coverage` |
| Routing | `.../path`, `.../redirects`, `.../menus`, `.../audience-rules`, `.../routes` |
| Delivery (public) | `/delivery/{site}/{locale}/{path}`, `.../sitemap.xml`, `.../robots.txt`, `.../{locale}/feed.xml`, `.../{locale}/menus/{key}` |
| Preview | `.../variants/{locale}/preview`, `/api/preview-tokens/{pid}`, `/delivery/{site}/preview/{token}` |
| Insights | `.../insights/health`, `.../insights/throughput`, `.../insights/backlog` |
| Webhooks | `.../webhooks`, `/api/webhooks/{pid}/deliveries`, `/api/webhooks/dispatch` |
| Audit & events | `/api/audits`, `/api/audits/recent`, `/api/events/recent` |
| Family fixtures | `/api-docs/openapi.json`, `/metrics.prom`, `/_health`, `/_ping` |

## Operating it

```bash
cargo run -- task schedule_sweep     # apply due publishes/unpublishes
cargo run -- task webhook_dispatch   # deliver due webhooks
cargo run -- task seed               # the synthetic corpus (idempotent)
```

Both sweeps are idempotent, so running them more often than necessary
is harmless.

### Configuration that changes behaviour

| Variable | Default | Effect |
|---|---|---|
| `CMS_REQUIRE_AUTH` | off | **The activation gate.** Off means no authentication and no authorization anywhere. |
| `CMS_PASETO_KEYS` / `_URL` | — | The published key set peers verify against, offline |
| `CMS_ABAC_POLICY` / `_FILE` | built-in | The policy; hot-reloaded from the file |
| `CMS_EVENT_TRANSPORT` | `memory` | `outbox` writes durable event rows — **required** for webhook dispatch |
| `CMS_ARTIFACT_BACKEND` | `local` | `s3` behind the optional cargo feature |
| `CMS_STALE_CONTENT_DAYS` | 365 | The staleness window |
| `CMS_REVIEW_SLA_DAYS` | 7 | When a review counts as stuck |

## The rules this service will not bend

Know these before changing anything here; each is argued in the
[spec](../spec/index.md).

1. **Bodies are blocks, never stored HTML.** Sanitization happens at
   write time against an allow-list, and there is no serializer in
   either direction.
2. **Delivery reads published revisions only**, behind a narrow
   allow-list: `GET`/`HEAD`, `visibility = public`, published only.
   The check is a database read on **every** request, so closing a
   site takes effect on the next request rather than the next restart.
3. **History is append-only.** Restore writes a new revision; erasure
   redacts a body while preserving the row and its linkage.
4. **Derived numbers show their working.** Ratios carry numerator and
   denominator and are `null` — never `0%` — on a zero denominator;
   percentiles are suppressed below a sample floor; every derived view
   carries `as_of`.
5. **No reader analytics.** Personalization reads request context
   only. There is no visitor identity anywhere.
6. **Refusals carry their remedy.** A publish blocker names the rule
   *and* what to do about it.
7. **Uploads are hostile input.** Byte caps, MIME sniffed from the
   bytes and matched against the declared type, an allow-list (not a
   deny-list), checksum addressing, `nosniff` on the way out. SVG is
   refused outright.

## Known gotchas

- loco's `create_table` pluralizes table names — use explicit SQL.
- `ModelError::EntityNotFound` is **not** mapped to `404`; return
  `Error::NotFound` at the `find_*` call site.
- Enforcement tests need their own test binary: the auth flag and the
  event transport are each resolved once per process.
- Delivery tests that need a durable transport likewise live in their
  own binary (`tests/webhook_delivery.rs`).
