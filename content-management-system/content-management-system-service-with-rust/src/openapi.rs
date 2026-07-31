//! Hand-written `OpenAPI` 3 description of the CMS REST API.
//!
//! Summary-level by design: every path and verb is present with its
//! request/response essentials; the full field-by-field shapes live in
//! the spec (`../../spec/domain-model.md`).
//!
//! Described so far: the declaration surface (sites, templates,
//! content types), the authoring surface (entries, per-locale
//! variants, the append-only revision chain, diff/restore, usage,
//! publish-check), and the asset library (upload, metadata,
//! renditions, replace, orphans), the editorial workflow
//! (transitions, publishing, scheduling, locks), and localization
//! (fallback resolution, the translation workflow, staleness),
//! routing (addresses, redirects, menus, audience rules), and the
//! public delivery surface (pages, menus, `sitemap.xml`,
//! `robots.txt`), content insights, and preview tokens, plus the
//! audit/event reads. Webhooks join with CMS-T23
//! (`../../spec/tasks.md`).

use serde_json::{Value, json};

/// The full `OpenAPI` document, served at `/api-docs/openapi.json`.
#[must_use]
#[allow(clippy::too_many_lines)] // one literal document
pub fn spec() -> Value {
    let ok = |desc: &str| json!({ "200": { "description": desc } });
    let created = json!({
        "200": { "description": "Created: {pid}" },
        "409": { "description": "Key already in use" },
        "422": { "description": "Validation failure" }
    });
    json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Content Management System Service API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Headless CMS. Serves the declaration layer — sites (delivery namespaces with locales, fallback chains, and the visibility that gates anonymous delivery reads), templates (declared region contracts; this service renders no HTML), and operator-defined content types whose edits are classified additive | tightening | breaking before they are applied — and the authoring layer: entries with one variant per locale, an append-only revision chain (a save states the revision it edited; a stale base is 409 with the competing revision), structured block documents (never stored HTML; an embed's markup is sanitized on write), diff and restore-as-new-revision, and references extracted on save so 'where used' is a lookup and a delete that would break something is refused. The editorial layer adds a lifecycle machine (draft → in_review → approved → published → archived, with reasons on the destructive transitions), publishing that names a *specific* revision so saving after publishing changes nothing live, gates that refuse a publish which is not ready (required fields, image alt text, missing reference targets — each with a remedy), scheduling applied by an idempotent sweep that skips anything a person has since overtaken, and advisory locks that never replace the authoritative stale-edit check. Localization adds fallback resolution that always reports the locale it actually served (and the hops it walked), strict locales that refuse fallback rather than quietly answering in another language, and a translation workflow whose requests pin the source revision — which is what makes staleness derivable, reported with the count and the revision numbers rather than a bare badge. Routing normalizes every address to one form, leaves a 301 when a page is renamed, and refuses redirect loops at write time; delivery serves published revisions only, behind a narrow public allow-list keyed on per-site visibility, with honest ETags and personalization that varies its own cache key. Insights are derived on read and never stored: findings carry the rule that produced them, ratios carry their numerator and denominator (null rather than a flattering percentage on a zero denominator), and percentiles are suppressed below a sample floor in favour of the raw observations. They are editorial insights about content — this service records no visits and holds no visitor identity. Unpublished content leaves the service by exactly one route: a preview token scoped to a single revision, short-lived, revocable, stored as a hash, and audited on both issue and use. Validation failures return 422 naming the offending path; a key clash, a stale edit, or a delete that something still references returns 409. API version is negotiated with the Accepts-version header (1.0)."
        },
        "paths": {
            "/api/sites": {
                "post": { "tags": ["sites"], "summary": "Declare a delivery namespace (visibility defaults to restricted)", "responses": created },
                "get": { "tags": ["sites"], "summary": "List live sites", "responses": ok("Sites") }
            },
            "/api/sites/{pid}": {
                "get": { "tags": ["sites"], "summary": "Site + its templates + its content types", "responses": ok("SiteDetail") },
                "put": { "tags": ["sites"], "summary": "Replace the configuration (a visibility change is audited as such)", "responses": json!({
                    "200": { "description": "The updated site" },
                    "409": { "description": "Key already in use" },
                    "422": { "description": "Validation failure (including an unwalkable fallback chain)" }
                }) },
                "delete": { "tags": ["sites"], "summary": "Soft-delete; refused while templates or content types remain", "responses": json!({
                    "200": { "description": "Deleted" },
                    "409": { "description": "Site still holds templates or content types" }
                }) }
            },
            "/api/sites/{pid}/templates": {
                "post": { "tags": ["templates"], "summary": "Declare a region contract (allowed block kinds, min/max)", "responses": created },
                "get": { "tags": ["templates"], "summary": "The site's live templates", "responses": ok("Templates") }
            },
            "/api/templates/{pid}": {
                "get": { "tags": ["templates"], "summary": "One template", "responses": ok("Template") },
                "put": { "tags": ["templates"], "summary": "Replace the region contract", "responses": ok("Template") },
                "delete": { "tags": ["templates"], "summary": "Soft-delete; refused while a content type names it", "responses": json!({
                    "200": { "description": "Deleted" },
                    "409": { "description": "Template still used by content types" }
                }) }
            },
            "/api/sites/{pid}/content-types": {
                "post": { "tags": ["content-types"], "summary": "Declare a content type (typed field schema, schema_version 1)", "responses": created },
                "get": { "tags": ["content-types"], "summary": "The site's live content types", "responses": ok("ContentTypes") }
            },
            "/api/content-types/{pid}": {
                "get": { "tags": ["content-types"], "summary": "One content type", "responses": ok("ContentType") },
                "put": { "tags": ["content-types"], "summary": "Edit the declaration; a breaking edit needs confirm_breaking + reason", "responses": json!({
                    "200": { "description": "Applied, with the change classification" },
                    "409": { "description": "Key already in use" },
                    "422": { "description": "Validation failure, or an unconfirmed breaking edit (names the breaking changes)" }
                }) },
                "delete": { "tags": ["content-types"], "summary": "Soft-delete", "responses": ok("Deleted") }
            },
            "/api/content-types/{pid}/compatibility": {
                "post": { "tags": ["content-types"], "summary": "Classify a proposed field set without writing (additive | tightening | breaking, with per-field detail)", "responses": ok("Classification") }
            },
            "/api/sites/{pid}/entries": {
                "post": { "tags": ["entries"], "summary": "Create an entry with its source-locale variant and revision 1", "responses": created },
                "get": { "tags": ["entries"], "summary": "The site's live entries (?content_type=)", "responses": ok("Entries") }
            },
            "/api/entries/{pid}": {
                "get": { "tags": ["entries"], "summary": "Entry + every locale variant (the locale matrix in one read)", "responses": ok("EntryDetail") },
                "delete": { "tags": ["entries"], "summary": "Soft-delete; refused while a live revision references it, or while a variant is published", "responses": json!({
                    "200": { "description": "Deleted" },
                    "409": { "description": "Still referenced, or still published" }
                }) }
            },
            "/api/entries/{pid}/usage": { "get": { "tags": ["entries"], "summary": "Where this entry is referenced (current revisions only)", "responses": ok("Usage") } },
            "/api/assets/{pid}/usage": { "get": { "tags": ["entries"], "summary": "Where this asset is referenced (current revisions only)", "responses": ok("Usage") } },
            "/api/entries/{pid}/variants": {
                "post": { "tags": ["entries"], "summary": "Start this entry in another declared locale", "responses": created }
            },
            "/api/entries/{pid}/variants/{locale}": {
                "get": { "tags": ["entries"], "summary": "Variant + its current revision (and which revision is published — a different question)", "responses": ok("VariantDetail") }
            },
            "/api/entries/{pid}/variants/{locale}/revisions": {
                "post": { "tags": ["entries"], "summary": "Save a revision, stating base_revision_pid", "responses": json!({
                    "200": { "description": "Saved: revision pid, number, blocks_sanitized, references" },
                    "409": { "description": "Stale base revision — names the competing revision" },
                    "422": { "description": "Validation failure naming the path (blocks[3].kind, fields.x)" }
                }) },
                "get": { "tags": ["entries"], "summary": "Revision history, newest first (summaries)", "responses": ok("Revisions") }
            },
            "/api/entries/{pid}/variants/{locale}/restore": {
                "post": { "tags": ["entries"], "summary": "Restore an earlier revision by writing a NEW one that copies it", "responses": ok("Saved") }
            },
            "/api/revisions/{pid}": { "get": { "tags": ["entries"], "summary": "One revision in full", "responses": ok("Revision") } },
            "/api/revisions/{from}/diff/{to}": { "get": { "tags": ["entries"], "summary": "What changed between two revisions of one variant (positional block comparison, disclosed in the payload)", "responses": ok("Diff") } },
            "/api/sites/{pid}/assets": {
                "post": { "tags": ["assets"], "summary": "Upload (multipart: file + optional title/alt_text/caption/credit/licence/tags/on_duplicate); typed from the bytes, content-addressed, deduplicated", "responses": json!({
                    "200": { "description": "The asset (deduplicated=true when identical bytes were already stored)" },
                    "413": { "description": "Over the per-upload cap or the site quota" },
                    "422": { "description": "Refused: unrecognised format, a declared type that disagrees with the bytes, or a format that can carry script (SVG, HTML, archives, executables)" }
                }) },
                "get": { "tags": ["assets"], "summary": "The library (?kind=&tag=&q=)", "responses": ok("Assets") }
            },
            "/api/sites/{pid}/assets/orphans": { "get": { "tags": ["assets"], "summary": "Assets nothing references — reported, never auto-deleted", "responses": ok("Orphans") } },
            "/api/sites/{pid}/assets/quota": { "get": { "tags": ["assets"], "summary": "Bytes used, the quota, the per-upload cap, and the accepted types", "responses": ok("Quota") } },
            "/api/assets/{pid}": {
                "get": { "tags": ["assets"], "summary": "The asset + its renditions + which of them a channel can actually fetch", "responses": ok("AssetDetail") },
                "put": { "tags": ["assets"], "summary": "Metadata only (the bytes are immutable — use replace)", "responses": ok("Asset") },
                "delete": { "tags": ["assets"], "summary": "Soft-delete; refused while used, overridable with ?force=true&reason= (records what it broke)", "responses": json!({
                    "200": { "description": "Deleted" },
                    "409": { "description": "Still used by a live revision" },
                    "422": { "description": "Forced without a reason" }
                }) }
            },
            "/api/assets/{pid}/content": { "get": { "tags": ["assets"], "summary": "The bytes, with nosniff and a kind-appropriate disposition (documents download rather than render)", "responses": ok("bytes") } },
            "/api/assets/{pid}/replace": { "post": { "tags": ["assets"], "summary": "New bytes, same asset identity: references are preserved and produced renditions reset to declared", "responses": ok("Asset") } },
            "/api/assets/{pid}/renditions": { "post": { "tags": ["assets"], "summary": "Declare a derived variant (images only; production is a documented seam)", "responses": ok("Rendition") } },
            "/api/renditions/{pid}": { "put": { "tags": ["assets"], "summary": "Record the outcome; `produced` requires a storage_ref", "responses": ok("Rendition") } },
            "/api/entries/{pid}/variants/{locale}/publish-check": { "get": { "tags": ["entries"], "summary": "What stands between this variant and publication — required fields, image alt text, missing reference targets — each with a remedy", "responses": ok("PublishCheck") } },
            "/api/entries/{pid}/variants/{locale}/transition": {
                "post": { "tags": ["workflow"], "summary": "submit | approve | reject | publish | unpublish | archive | restore; reject/unpublish/archive/restore require a reason", "responses": json!({
                    "200": { "description": "The applied transition (from, to, published revision, first_published_at)" },
                    "422": { "description": "Illegal from the current state (names it and the legal actions), a missing reason, a distinct-approver refusal, or publish gates not met" }
                }) }
            },
            "/api/entries/{pid}/variants/{locale}/schedule": {
                "post": { "tags": ["workflow"], "summary": "Queue a publish and/or unpublish; refused unless the transition would be legal now and the time is in the future", "responses": ok("Variant") }
            },
            "/api/entries/{pid}/variants/{locale}/lock": {
                "post": { "tags": ["workflow"], "summary": "Take or extend the advisory lock (stealing one needs a reason)", "responses": json!({
                    "200": { "description": "Lock holder and expiry" },
                    "409": { "description": "Held by someone else and no reason was given" }
                }) },
                "delete": { "tags": ["workflow"], "summary": "Release the lock", "responses": ok("Released") }
            },
            "/api/schedules/sweep": { "post": { "tags": ["workflow"], "summary": "Apply due schedules now (idempotent; skips and records anything whose state has moved). Also available as the `schedule_sweep` CLI task", "responses": ok("SweepOutcomes") } },
            "/api/sites/{pid}/schedules": { "get": { "tags": ["workflow"], "summary": "What is queued, before it fires", "responses": ok("Schedules") } },
            "/api/sites/{pid}/published": { "get": { "tags": ["workflow"], "summary": "What is live, and where newer work is waiting behind it", "responses": ok("Published") } },
            "/api/entries/{pid}/resolve/{locale}": { "get": { "tags": ["localization"], "summary": "Which locale would serve this entry — with locale_requested, locale_served, fallback_applied, and the hops actually walked", "responses": ok("Resolution") } },
            "/api/entries/{pid}/translations": { "get": { "tags": ["localization"], "summary": "The locale matrix: status, published, translation state, and derived staleness per locale, plus the locales never started", "responses": ok("Matrix") } },
            "/api/entries/{pid}/variants/{locale}/translation": {
                "post": { "tags": ["localization"], "summary": "request | claim | complete | cancel; `request` pins the source revision being translated", "responses": json!({
                    "200": { "description": "The translation state" },
                    "404": { "description": "No variant in that locale" },
                    "422": { "description": "Out of order, or the source locale, or nothing to translate from" }
                }) }
            },
            "/api/sites/{pid}/translations": { "get": { "tags": ["localization"], "summary": "The translator queue and the stale list; stale content is reported, never auto-unpublished (unless a content type opted in, in which case it is listed under would_unpublish)", "responses": ok("SiteTranslations") } },
            "/api/sites/{pid}/locale-coverage": { "get": { "tags": ["localization"], "summary": "Per locale: entries started, published, and the keys still missing", "responses": ok("Coverage") } },
            "/api/entries/{pid}/variants/{locale}/path": { "put": { "tags": ["routing"], "summary": "Set or change a page's address; a change leaves a 301 from the old one automatically", "responses": json!({
                "200": { "description": "The normalized path, and whether a redirect was created" },
                "409": { "description": "Another live page already answers at that address" },
                "422": { "description": "Malformed path, a non-routable type, or a change that would create a loop" }
            }) } },
            "/api/sites/{pid}/routes": { "get": { "tags": ["routing"], "summary": "The live address book", "responses": ok("Routes") } },
            "/api/sites/{pid}/redirects": {
                "post": { "tags": ["routing"], "summary": "Declare a redirect (301/302) or a 410 marker; loops are refused and chains collapse to their final target", "responses": ok("Redirect") },
                "get": { "tags": ["routing"], "summary": "The redirect table", "responses": ok("Redirects") }
            },
            "/api/redirects/{pid}": { "delete": { "tags": ["routing"], "summary": "Remove a redirect", "responses": ok("Deleted") } },
            "/api/sites/{pid}/menus": {
                "post": { "tags": ["routing"], "summary": "Declare navigation", "responses": ok("Menu") },
                "get": { "tags": ["routing"], "summary": "Declared menus", "responses": ok("Menus") }
            },
            "/api/sites/{pid}/audience-rules": {
                "post": { "tags": ["routing"], "summary": "Declare a personalization rule over the allow-listed request context (locale, channel, audience_tag, preview)", "responses": json!({
                    "200": { "description": "The rule" },
                    "422": { "description": "A predicate reading anything outside the allow-list — no cookies, IPs, user agents, or referrers" }
                }) },
                "get": { "tags": ["routing"], "summary": "Declared rules, and the context keys personalization may read", "responses": ok("AudienceRules") }
            },
            "/delivery/{site}/{locale}/{path}": { "get": { "tags": ["delivery"], "summary": "The composed page: published revisions only, with locale honesty fields, one-hop reference summaries, existing renditions, the template's region contract, canonical, and which audience rules matched. Weak ETag (excluding as_of) with 304 support; personalized responses vary their tag and declare Vary. Public sites answer anonymously; restricted sites need a credential", "responses": json!({
                "200": { "description": "The document" },
                "301": { "description": "The address moved (Location names the new one)" },
                "304": { "description": "Unchanged since the given ETag" },
                "401": { "description": "A restricted site, with no credential" },
                "404": { "description": "No such page, or nothing published there" },
                "410": { "description": "The page was unpublished and left a Gone marker" },
                "508": { "description": "The redirect chain could not be resolved" }
            }) } },
            "/delivery/{site}/{locale}/menus/{key}": { "get": { "tags": ["delivery"], "summary": "A resolved menu; items whose target is not published are omitted", "responses": ok("Menu") } },
            "/delivery/{site}/sitemap.xml": { "get": { "tags": ["delivery"], "summary": "Derived from published, indexable, routable variants, with lastmod and reciprocal hreflang alternates", "responses": ok("XML") } },
            "/delivery/{site}/{locale}/feed.xml": { "get": { "tags": ["delivery"], "summary": "Atom 1.0 feed of recently published pages in one locale, newest first, capped at 50. Published-only and noindex-excluded (a page the site asked crawlers to ignore has not asked to be syndicated); summaries are declared plain text, never markup; the entry id is the entry's pid so a rename does not resurface it as a new item; an empty feed still carries an `updated`", "responses": ok("text") } },
            "/delivery/{site}/robots.txt": { "get": { "tags": ["delivery"], "summary": "Site policy plus the sitemap pointer; a restricted site disallows everything", "responses": ok("text") } },
            "/api/sites/{pid}/insights/health": { "get": { "tags": ["insights"], "summary": "Content health: findings grouped by rule, each shipping the sentence the code applied — missing alt text, missing SEO, broken references, orphan assets, stale content and translations, stuck reviews, unpublished approvals, needs-migration, route hazards. Derived on read, ETag-conditional, and it acts on nothing", "responses": ok("Health") } },
            "/api/sites/{pid}/insights/throughput": { "get": { "tags": ["insights"], "summary": "Editorial throughput over ?days= (default 30): activity by transition, rates that show numerator and denominator (null on a zero denominator), time-in-state measured from transition audit rows with percentiles suppressed below a sample floor, and per-actor counts", "responses": ok("Throughput") } },
            "/api/sites/{pid}/insights/backlog": { "get": { "tags": ["insights"], "summary": "What is waiting — pending reviews, pending schedules, open translations — bucketed by age", "responses": ok("Backlog") } },
            "/api/entries/{pid}/variants/{locale}/preview": {
                "post": { "tags": ["preview"], "summary": "Mint a preview share: scoped to ONE (variant, revision), short-lived (15 min default, 1 day max), revocable. The token is returned once and stored only as a hash; issue and use are audited", "responses": ok("IssuedShare") },
                "get": { "tags": ["preview"], "summary": "Outstanding shares for this variant (never the tokens)", "responses": ok("Shares") }
            },
            "/api/preview-tokens/{pid}": { "delete": { "tags": ["preview"], "summary": "Withdraw a share immediately", "responses": ok("Revoked") } },
            "/delivery/{site}/preview/{token}": { "get": { "tags": ["preview"], "summary": "Render the shared revision. no-store and noindex; every refusal (unknown, expired, revoked, wrong revision) returns the same 404 so the endpoint cannot be used to probe for valid tokens", "responses": json!({
                "200": { "description": "The previewed revision" },
                "404": { "description": "Not a valid share (uniform answer for every refusal)" }
            }) } },
            "/api/sites/{pid}/webhooks": {
                "post": { "tags": ["webhooks"], "summary": "Register an outbound subscription. The signing secret is returned ONCE and by no read afterwards; the URL must be https (loopback excepted) and carry no credentials", "responses": ok("RegisteredWebhook") },
                "get": { "tags": ["webhooks"], "summary": "Subscriptions for this site, without their secrets", "responses": ok("Webhooks") }
            },
            "/api/webhooks/{pid}": { "delete": { "tags": ["webhooks"], "summary": "Withdraw a subscription", "responses": ok("Deleted") } },
            "/api/webhooks/{pid}/deliveries": { "get": { "tags": ["webhooks"], "summary": "The attempt log: state, status, error, and attempt number per event", "responses": ok("Deliveries") } },
            "/api/webhooks/dispatch": { "post": { "tags": ["webhooks"], "summary": "Deliver what is due, signed HMAC-SHA256 over `{timestamp}.{body}`. Reads the durable event outbox, so it requires CMS_EVENT_TRANSPORT=outbox and returns 422 otherwise rather than delivering a subset that vanishes on restart. Reruns are safe: delivered and abandoned events are never re-sent and failed ones wait out their backoff", "responses": json!({
                "200": { "description": "What was attempted, delivered, failed, and abandoned" },
                "422": { "description": "The in-memory transport has no durable record to deliver from" }
            }) } },
            "/api/audits/recent": { "get": { "tags": ["audit"], "summary": "Recent audit entries", "responses": ok("Audits") } },
            "/api/audits": { "get": { "tags": ["audit"], "summary": "Owner-scoped audit (?owner=&since=)", "responses": ok("Audits") } },
            "/api/audits/{entity_pid}": { "get": { "tags": ["audit"], "summary": "One record's trail", "responses": ok("Audits") } },
            "/api/events/recent": { "get": { "tags": ["audit"], "summary": "Recent events (memory ring, or the outbox under CMS_EVENT_TRANSPORT=outbox)", "responses": ok("Events") } }
        },
        "tags": [
            { "name": "sites", "description": "Delivery namespaces: locales, fallback chains, visibility" },
            { "name": "templates", "description": "Declared region contracts a channel lays out" },
            { "name": "content-types", "description": "Operator-defined field schemas and the compatibility gate" },
            { "name": "entries", "description": "Entries, locale variants, the append-only revision chain, and references" },
            { "name": "assets", "description": "The asset library: uploads, metadata, renditions, orphans" },
            { "name": "workflow", "description": "Editorial transitions, publishing, scheduling, locks" },
            { "name": "localization", "description": "Locale resolution, translation workflow, staleness" },
            { "name": "routing", "description": "Addresses, redirects, menus, and audience rules" },
            { "name": "delivery", "description": "The public, published-only read surface" },
            { "name": "insights", "description": "Content health and editorial throughput — derived, never stored, and never about readers" },
            { "name": "preview", "description": "The one credential that shows unpublished content" },
            { "name": "webhooks", "description": "Signed outbound subscriptions — this service's only extension mechanism" },
            { "name": "audit", "description": "Audit trail and event stream" }
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The document parses, is version-stamped, and describes the
    /// routes the app actually mounts.
    #[test]
    fn spec_covers_the_mounted_paths() {
        let spec = spec();
        assert_eq!(spec["openapi"], "3.0.3");
        assert_eq!(spec["info"]["version"], env!("CARGO_PKG_VERSION"));
        let paths = spec["paths"].as_object().expect("paths is an object");
        for path in [
            "/api/sites",
            "/api/sites/{pid}",
            "/api/sites/{pid}/templates",
            "/api/templates/{pid}",
            "/api/sites/{pid}/content-types",
            "/api/content-types/{pid}",
            "/api/content-types/{pid}/compatibility",
            "/api/audits/recent",
            "/api/events/recent",
            "/api/sites/{pid}/entries",
            "/api/entries/{pid}",
            "/api/entries/{pid}/usage",
            "/api/entries/{pid}/variants",
            "/api/entries/{pid}/variants/{locale}",
            "/api/entries/{pid}/variants/{locale}/revisions",
            "/api/entries/{pid}/variants/{locale}/restore",
            "/api/revisions/{pid}",
            "/api/revisions/{from}/diff/{to}",
            "/api/assets/{pid}/usage",
            "/api/sites/{pid}/assets",
            "/api/sites/{pid}/assets/orphans",
            "/api/assets/{pid}",
            "/api/assets/{pid}/content",
            "/api/assets/{pid}/replace",
            "/api/assets/{pid}/renditions",
            "/api/renditions/{pid}",
            "/api/entries/{pid}/variants/{locale}/publish-check",
            "/api/entries/{pid}/variants/{locale}/transition",
            "/api/entries/{pid}/variants/{locale}/schedule",
            "/api/entries/{pid}/variants/{locale}/lock",
            "/api/schedules/sweep",
            "/api/sites/{pid}/schedules",
            "/api/sites/{pid}/published",
            "/api/entries/{pid}/resolve/{locale}",
            "/api/entries/{pid}/translations",
            "/api/entries/{pid}/variants/{locale}/translation",
            "/api/sites/{pid}/translations",
            "/api/sites/{pid}/locale-coverage",
            "/api/entries/{pid}/variants/{locale}/path",
            "/api/sites/{pid}/routes",
            "/api/sites/{pid}/redirects",
            "/api/sites/{pid}/menus",
            "/api/sites/{pid}/audience-rules",
            "/delivery/{site}/{locale}/{path}",
            "/delivery/{site}/sitemap.xml",
            "/delivery/{site}/robots.txt",
            "/delivery/{site}/{locale}/feed.xml",
            "/api/sites/{pid}/insights/health",
            "/api/sites/{pid}/insights/throughput",
            "/api/sites/{pid}/insights/backlog",
            "/api/entries/{pid}/variants/{locale}/preview",
            "/api/preview-tokens/{pid}",
            "/delivery/{site}/preview/{token}",
            "/api/sites/{pid}/webhooks",
            "/api/webhooks/{pid}",
            "/api/webhooks/{pid}/deliveries",
            "/api/webhooks/dispatch",
        ] {
            assert!(paths.contains_key(path), "{path} is missing from the spec");
        }
    }
}
