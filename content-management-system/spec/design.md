# Design decisions

Numbered, stable; tasks ([tasks.md](tasks.md)) trace to them.

## CMS-D1 — Consumer application; identities by URN, readers not at all

Authors, editors, reviewers, and translators are `worker:` URNs; a
site's owning body is an `organization:` URN; content *about* a
registered entity carries that entity's `EntityRef` as a typed
field. CMS owns content and editorial state only. **Readers are not
modelled**: no visitor identity, no profile store, no audience table
([scope](scope.md)).

## CMS-D2 — Hybrid persistence: JSONB where the shape is operator-defined, normalized where invariants live

Content-type schemas and entry field values are declared at runtime
by operators, so they are JSONB validated at the boundary against a
declared `schema_version`. Variants, revisions, routes, references,
assets, renditions, menus, redirects, and schedules carry real
invariants (path uniqueness per site+locale, monotonic revision
numbers, delete-refusal on a live reference, loop-free redirects)
and are therefore normalized, constraint-backed tables. A
constraint that exists only in application code over a JSON blob is
not a constraint. All-plural table names (the loco pluralization
lesson).

## CMS-D3 — Revisions are append-only; publish points at one

Every save writes a full-snapshot revision; nothing updates or
deletes one. Restore copies an old body into a **new** revision.
Publishing sets a pointer to a specific revision, so "saved" and
"live" are different facts and editing after publish changes
nothing until the next publish. The declared resolution for erasure
is to redact a body while preserving the row and its linkage — the
family's history-versus-erasure resolution ([audit](audit.md)) — but
the write path is a production gate (CMS-G3), not yet implemented;
today only the read-time `mask` ABAC obligation redacts unpublished
content in a response.

## CMS-D4 — Every lifecycle is a pure-core state machine

Editorial (draft → in_review → approved → published → archived,
with reject / unpublish / reasoned restore) and translation
(request → in_translation → translated) are transition tables in
DB-free `rules/` modules, exhaustively unit-tested; controllers only
wire them; an illegal transition is `422` naming the current state.
Direct-publish is a *policy* permission on the same transition, not
a second code path.

## CMS-D5 — Structured blocks, not stored HTML

Bodies are typed block documents with structured inline marks. A
stored-HTML CMS is a stored-XSS engine; blocks carry no markup to
smuggle, render to any channel, and are queryable (which is what
makes reference extraction and "where used" possible at all). Where
HTML is accepted at the edges (import, `embed`) it is sanitized
against an allow-list **at write time** and re-escaped at delivery.

## CMS-D6 — Headless: the service renders nothing

Delivery is structured JSON; a **Template** is a declared region
contract, not markup. The family runs backend-only services with no
template tier ([loco](../../agents/share/loco.md)), and the
omnichannel claim is only true if the payload is not already a web
page. Themes and CSS live in the channel.

## CMS-D7 — Delivery reads published revisions only, behind a narrow public allow-list

The composer cannot reach an unpublished revision, and the public
allow-list is method-, site-, and status-scoped (`GET`/`HEAD`,
`visibility = public`, published only) — the guard-all /
deny-unless-public posture ([security](../../agents/share/security.md)
SEC-G5) with one deliberate, minimal exception. Unpublished access
is authenticated, or via a **preview token scoped to exactly one
(variant, revision)**, short-lived and audited: the permanent
guessable preview URL is how embargoed content leaks.

## CMS-D8 — References are extracted at save time

The pure core walks blocks and fields on every save and writes
Reference rows in the same transaction. This makes "where used" an
index lookup, makes delete-refusal enforceable, and makes broken
references a derived finding rather than a reader's 404.

## CMS-D9 — Assets on the family `ArtifactStore`, content-addressed, allow-listed

Reuse the care-pathway `ArtifactStore` (local / s3, base-confined,
presigned GETs) rather than inventing storage or signing. SHA-256
addressing dedupes; declared MIME must match sniffed bytes; media
types are allow-listed, not deny-listed; filenames are metadata,
never paths. Renditions are *declared* in v1 — image transcoding on
attacker-supplied bytes earns its own hardening round
([roadmap](roadmap.md)).

## CMS-D10 — Routing invariants are enforced at write time

Unique current path per (site, locale); normalized paths; a slug
change auto-creates a `301`; redirect loops are refused and
over-long chains collapsed **on creation**, with a bounded hop cap
at resolution. Discovering a redirect loop at request time is a
request-time DoS with an editorial cause.

## CMS-D11 — Personalization from request context only

Audience rules read an allow-listed context (`locale`, `channel`,
`audience_tag`, `preview`) asserted by the caller — never cookies,
IPs, user agents, referrers, or behavioural history. A rule engine
that cannot see a visitor cannot become a tracking system by
accident. Matched rules are reported, and personalized responses
vary their ETag by the consulted context and declare `Vary` (a
personalized page cached on URL alone is a leak).

## CMS-D12 — Extension points are webhooks, not plugins

The "plugins" a CMS is expected to have are, here, declared
outbound webhooks driven from the event record: HTTPS only, no
redirects followed, signed, timed out, size-capped, retried with
backoff, logged. Loading third-party code into a service that
forbids `unsafe` and gates every input would forfeit exactly the
properties this family exists to demonstrate.

## CMS-D13 — Derived numbers, never stored opinions

Staleness, content-health findings, throughput, time-in-state,
locale coverage, sitemap contents, and "where used" are pure-core
derivations from recorded facts. No editable insight fields exist;
each finding carries the rule that produced it, and the honesty
rules (numerator/denominator, `null` on zero denominator,
percentile sample floor) apply everywhere.

## CMS-D14 — Jobs on `bg_pg`, idempotent by key

The schedule sweep (idempotent per variant × scheduled_at), sitemap
build, reference/link check, and webhook delivery run as loco
Postgres-backed jobs — no external broker. Every job is safe to
rerun and records what it skipped.

## CMS-D15 — Transactional integrity

Revision write + reference extraction + route change + audit +
outbox share the mutation's transaction; publish, unpublish, and
schedule execution serialize on the variant row (`FOR UPDATE`), and
revision numbers are allocated under that lock. Optimistic
concurrency on `base_revision_pid` is authoritative; advisory locks
are cooperative and never claimed to be more.

## CMS-D16 — Stub-first upstream clients

Display-name lookups behind traits with `http` + `stub`
implementations, config-selected, cached, best-effort — boots and
tests with no siblings running; an unresolvable name never blocks a
write.

## CMS-D17 — Family fixtures from day one

Loco-idiomatic layout, forbid-unsafe + clippy-pedantic, OpenAPI +
Swagger, `Accepts-version`, OTLP + `/metrics.prom`, Podman, input
caps, `404` mapping at `find_by_pid` call sites, enforcement tests
in their own binary (the OnceLock lesson), ETag-conditional
delivery and insights, 13-locale i18n in the front-end from the
start (the PPM lesson).
