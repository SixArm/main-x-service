// Typed CMS endpoints, in one place.
//
// Every path the UI calls is built by a function here rather than
// interpolated at the call site, so the endpoint contract is one file a
// reader can check against the service's OpenAPI document — and a unit
// test can pin it without a running service.
//
// Those paths were checked against the live document on 2026-07-31
// (`GET /api-docs/openapi.json`), **method included**. Both halves
// matter: the first pass compared paths only and passed a `GET` on
// `/api/entries/{pid}/variants`, which the service serves for `POST`
// alone — a 405 waiting to happen that looked verified. Re-run the
// comparison, with verbs, whenever the service's routes change.
//
// The types are **the shape the UI relies on**, not a mirror of every
// field the service returns. Claiming completeness we do not verify
// would be worse than an honest subset: TypeScript would assert facts
// about a payload nobody checked.

import { api, apiConditional, type Conditional } from "./client";
import { PREVIEW_BASE_URL } from "$lib/config";

/** A delivery namespace: locales, fallback chains, visibility. */
export interface Site {
  pid: string;
  key: string;
  name: string;
  default_locale: string;
  locales: string[];
  fallback_chains: Record<string, string[]>;
  visibility: string;
  base_url: string | null;
}

/** An operator-defined field schema. */
export interface ContentType {
  pid: string;
  key: string;
  name: string;
  fields: FieldSpec[];
  routable: boolean;
  schema_version: number;
  template_key: string | null;
}

/** One declared field on a content type. */
export interface FieldSpec {
  key: string;
  label: string;
  kind: string;
  required?: boolean;
  repeatable?: boolean;
}

/** A piece of content, independent of locale. */
export interface Entry {
  pid: string;
  key: string;
  content_type_key: string;
  source_locale: string;
  owner_ref: string | null;
  archived_at: string | null;
}

/** One locale's copy of an entry, and where it sits in the workflow. */
export interface Variant {
  pid: string;
  locale: string;
  status: string;
  current_revision_pid: string | null;
  published_revision_pid: string | null;
  translation_status: string | null;
  scheduled_publish_at: string | null;
}

/** One immutable point in a variant's history. */
export interface Revision {
  pid: string;
  number: number;
  title: string;
  blocks: Block[];
  fields: Record<string, unknown>;
  seo: Record<string, unknown>;
  author_ref: string | null;
  created_at: string;
}

/** A structured block. Bodies are blocks, never stored HTML. */
export interface Block {
  kind: string;
  [key: string]: unknown;
}

/** A finding from the content-health view, carrying the rule that
 *  produced it — the UI renders the rule, it does not invent one. */
export interface Finding {
  rule: string;
  subject: string;
  locale: string | null;
  detail: string;
  owner: string | null;
}

/** Health findings grouped by rule, each group naming its explanation. */
export interface HealthGroup {
  rule: string;
  explanation: string;
  count: number;
  findings: Finding[];
}

/** The content-health view. */
export interface Health {
  as_of: string;
  site: string;
  entries: number;
  published_variants: number;
  findings_total: number;
  orphan_bytes: number;
  by_rule: HealthGroup[];
}

/** A ratio that shows its working. `value` is `null` when the
 *  denominator is zero — the UI must render a no-data state, never
 *  `0%` (`../../spec/insights.md`). */
export interface Ratio {
  numerator: number;
  denominator: number;
  value: number | null;
}

/** The editorial-throughput view. */
export interface Throughput {
  as_of: string;
  period_days: number;
  activity: Record<string, number>;
  rates: Record<string, Ratio>;
  time_in_state: Record<string, unknown>;
}

/** What is waiting, bucketed by age. */
export interface Backlog {
  as_of: string;
  pending_review: unknown[];
  pending_schedule: unknown[];
  open_translations: unknown[];
}

/** One locale's row in an entry's translation matrix. */
export interface LocaleRow {
  locale: string;
  is_source: boolean;
  status: string;
  published: boolean;
  translation_status: string | null;
  translator_ref: string | null;
  due_on: string | null;
  staleness: Staleness;
}

/** How far a translation has fallen behind its source. */
export interface Staleness {
  stale: boolean;
  revisions_behind: number;
  newer_revision_numbers?: number[];
  /** Present when the answer is unknown rather than "not stale" — the
   *  service says which, and the UI must not flatten the two. */
  unknown?: string;
}

/** A revision as the history list shows it (no body). */
export interface RevisionSummary {
  pid: string;
  number: number;
  title: string;
  author_ref: string | null;
  note: string | null;
  created_at: string;
  is_current: boolean;
  is_published: boolean;
  restored_from_pid: string | null;
}

/** What publishing would refuse, and how to fix it. */
export interface Blocker {
  rule: string;
  subject: string;
  remedy: string;
}

/** The publish gate's answer. */
export interface PublishCheck {
  ready: boolean;
  status: string;
  blockers: Blocker[];
  revision_pid: string | null;
}

/** A positional comparison of two revisions. */
export interface Diff {
  from: { pid: string; number: number };
  to: { pid: string; number: number };
  diff: {
    identical: boolean;
    title_changed: boolean;
    seo_changed: boolean;
    block_comparison: string;
    blocks: unknown[];
    fields: unknown[];
  };
}

/** An asset as the library lists it. */
export interface Asset {
  pid: string;
  kind: string;
  mime: string;
  byte_size: number;
  title: string | null;
  alt_text: string | null;
  original_filename: string | null;
  width: number | null;
  height: number | null;
}

/** A rendered preview of one revision, fetched through this app's
 *  server so the token stays there (`../../spec/auth.md`). */
export interface Preview {
  preview: boolean;
  locale: string;
  status: string;
  revision: Revision;
  is_published_revision: boolean;
}

type Fetch = typeof fetch;

/** Options every call accepts: SvelteKit's `fetch` inside a `load`. */
interface Options {
  fetch?: Fetch;
}

/** Every site. */
export const listSites = (o?: Options): Promise<Site[]> => api("/api/sites", o);

/** One site. */
export const getSite = (pid: string, o?: Options): Promise<Site> =>
  api(`/api/sites/${pid}`, o);

/** A site's content types. */
export const listContentTypes = (
  sitePid: string,
  o?: Options,
): Promise<ContentType[]> => api(`/api/sites/${sitePid}/content-types`, o);

/** A site's entries. */
export const listEntries = (sitePid: string, o?: Options): Promise<Entry[]> =>
  api(`/api/sites/${sitePid}/entries`, o);

/** An entry with its locale variants — one request, because the
 *  service returns them together and there is no variants listing to
 *  call separately. */
export const getEntry = (
  pid: string,
  o?: Options,
): Promise<{ entry: Entry; variants: Variant[] }> =>
  api(`/api/entries/${pid}`, o);

/** The locale matrix for one entry: status, staleness, translator. */
export const entryTranslations = (
  pid: string,
  o?: Options,
): Promise<{ entry_key: string; locales: LocaleRow[] }> =>
  api(`/api/entries/${pid}/translations`, o);

/** A variant's revision history, newest first.
 *
 * Summaries, not bodies: the listing carries `is_current` /
 * `is_published` and no blocks, so a history view costs one request
 * rather than one per revision. Fetch a body with `getRevision`. */
export const listRevisions = (
  pid: string,
  locale: string,
  o?: Options,
): Promise<RevisionSummary[]> =>
  api(`/api/entries/${pid}/variants/${locale}/revisions`, o);

/** Write a new revision.
 *
 * `base_revision_pid` is what makes a concurrent edit a `409` rather
 * than a silent overwrite; the caller must render that conflict as a
 * comparison, never a retry (`../../spec/authoring.md`). */
export const createRevision = (
  pid: string,
  locale: string,
  body: {
    base_revision_pid: string | null;
    title: string;
    blocks: Block[];
    fields?: Record<string, unknown>;
    seo?: Record<string, unknown>;
  },
  o?: Options,
): Promise<Revision> =>
  api(`/api/entries/${pid}/variants/${locale}/revisions`, {
    method: "POST",
    body,
    ...o,
  });

/** Move a variant through the editorial lifecycle. Actions that change
 *  what readers see (`reject`, `unpublish`, `archive`, `restore`)
 *  require a reason, and the service refuses without one. */
export const transition = (
  pid: string,
  locale: string,
  action: string,
  reason?: string,
  o?: Options,
): Promise<Variant> =>
  api(`/api/entries/${pid}/variants/${locale}/transition`, {
    method: "POST",
    body: reason ? { action, reason } : { action },
    ...o,
  });

/** What publishing this variant would refuse, before trying it. */
export const publishCheck = (
  pid: string,
  locale: string,
  o?: Options,
): Promise<PublishCheck> =>
  api(`/api/entries/${pid}/variants/${locale}/publish-check`, o);

/** A site's asset library. */
export const listAssets = (
  sitePid: string,
  o?: Options,
): Promise<
  { pid: string; kind: string; title: string | null; alt_text: string | null }[]
> => api(`/api/sites/${sitePid}/assets`, o);

/** Where an asset is used — the lookup that makes deletion refusable. */
export const assetUsage = (pid: string, o?: Options): Promise<unknown> =>
  api(`/api/assets/${pid}/usage`, o);

/** One revision, with its body. */
export const getRevision = (pid: string, o?: Options): Promise<Revision> =>
  api(`/api/revisions/${pid}`, o);

/** Compare two revisions. Positional, and the response says so —
 *  the UI must not present it as a content-aligned diff. */
export const diff = (from: string, to: string, o?: Options): Promise<Diff> =>
  api(`/api/revisions/${from}/diff/${to}`, o);

/** Restore an earlier revision. This writes a **new** revision rather
 *  than rewinding history, and the UI says so before doing it. */
export const restore = (
  pid: string,
  locale: string,
  revision_pid: string,
  reason: string,
  o?: Options,
): Promise<Revision> =>
  api(`/api/entries/${pid}/variants/${locale}/restore`, {
    method: "POST",
    body: { revision_pid, reason },
    ...o,
  });

/** Queue a publish or unpublish for later. */
export const schedule = (
  pid: string,
  locale: string,
  body: { publish_at?: string | null; unpublish_at?: string | null },
  o?: Options,
): Promise<unknown> =>
  api(`/api/entries/${pid}/variants/${locale}/schedule`, {
    method: "POST",
    body,
    ...o,
  });

/** Take an advisory edit lock. */
export const lock = (
  pid: string,
  locale: string,
  o?: Options,
): Promise<unknown> =>
  api(`/api/entries/${pid}/variants/${locale}/lock`, { method: "POST", ...o });

/** Release an advisory edit lock. */
export const unlock = (
  pid: string,
  locale: string,
  o?: Options,
): Promise<unknown> =>
  api(`/api/entries/${pid}/variants/${locale}/lock`, {
    method: "DELETE",
    ...o,
  });

/** What is queued to go live or come down, site-wide. */
export const schedules = (
  sitePid: string,
  o?: Options,
): Promise<{ as_of: string; queued: ScheduledItem[] }> =>
  api(`/api/sites/${sitePid}/schedules`, o);

/** A queued publish or unpublish. */
export interface ScheduledItem {
  entry_pid: string;
  entry_key: string;
  locale: string;
  status: string;
  publish_at: string | null;
  unpublish_at: string | null;
}

/** The site's translation queue and staleness, with the rule applied. */
export const translations = (
  sitePid: string,
  o?: Options,
): Promise<{
  as_of: string;
  rule: string;
  auto_unpublished: boolean;
  queue: TranslationQueueItem[];
  stale?: unknown[];
}> => api(`/api/sites/${sitePid}/translations`, o);

/** An open translation request. */
export interface TranslationQueueItem {
  entry_pid: string;
  entry_key: string;
  locale: string;
  translation_status: string;
  translator_ref: string | null;
  requested_at: string | null;
  due_on: string | null;
}

/** Assets nothing currently references — reported, never deleted. */
export const orphanAssets = (
  sitePid: string,
  o?: Options,
): Promise<{
  as_of: string;
  auto_deleted: boolean;
  bytes_reclaimable: number;
  orphans: { pid: string; mime: string; byte_size: number; title?: string }[];
}> => api(`/api/sites/${sitePid}/assets/orphans`, o);

/** Storage use and the upload rules. */
export const assetQuota = (
  sitePid: string,
  o?: Options,
): Promise<{
  used_bytes: number;
  quota_bytes: number;
  max_upload_bytes: number;
  accepted_types: string[];
}> => api(`/api/sites/${sitePid}/assets/quota`, o);

/** A site's templates. */
export const listTemplates = (
  sitePid: string,
  o?: Options,
): Promise<unknown[]> => api(`/api/sites/${sitePid}/templates`, o);

/** A site's menus. */
export const listMenus = (sitePid: string, o?: Options): Promise<unknown[]> =>
  api(`/api/sites/${sitePid}/menus`, o);

/** A site's redirects. */
export const listRedirects = (
  sitePid: string,
  o?: Options,
): Promise<unknown[]> => api(`/api/sites/${sitePid}/redirects`, o);

/** A site's current addresses. */
export const listRoutes = (sitePid: string, o?: Options): Promise<unknown[]> =>
  api(`/api/sites/${sitePid}/routes`, o);

/** What is live right now, and which revision it is. */
export const published = (sitePid: string, o?: Options): Promise<unknown> =>
  api(`/api/sites/${sitePid}/published`, o);

/** A site's outbound webhook subscriptions (never their secrets). */
export const listWebhooks = (
  sitePid: string,
  o?: Options,
): Promise<{ webhooks: unknown[]; note: string }> =>
  api(`/api/sites/${sitePid}/webhooks`, o);

/** Content health. Conditional: the view is derived on read and
 *  ETag-stamped, so a repeat poll is a `304`. */
export const health = (
  sitePid: string,
  etag: string | null,
  o?: Options,
): Promise<Conditional<Health>> =>
  apiConditional(`/api/sites/${sitePid}/insights/health`, etag, o);

/** Editorial throughput over a window. */
export const throughput = (
  sitePid: string,
  days: number,
  o?: Options,
): Promise<Throughput> =>
  api(`/api/sites/${sitePid}/insights/throughput?days=${days}`, o);

/** What is waiting. */
export const backlog = (sitePid: string, o?: Options): Promise<Backlog> =>
  api(`/api/sites/${sitePid}/insights/backlog`, o);

/** Locale coverage across the site. */
export const localeCoverage = (
  sitePid: string,
  o?: Options,
): Promise<{ as_of: string; coverage: unknown[] }> =>
  api(`/api/sites/${sitePid}/locale-coverage`, o);

/**
 * Render one revision as a preview.
 *
 * Deliberately **not** a proxy call: this goes to this app's own
 * server route, which mints the token, spends it, and revokes it. No
 * preview token ever reaches the browser (`../../spec/auth.md`).
 */
export const preview = (
  pid: string,
  locale: string,
  site: string,
  revision?: string,
  o?: Options,
): Promise<Preview> => {
  const query = new URLSearchParams({ site });
  if (revision) query.set("revision", revision);
  const doFetch = o?.fetch ?? fetch;
  return doFetch(
    `${PREVIEW_BASE_URL}/${pid}/${locale}?${query.toString()}`,
  ).then(async (response) => {
    if (!response.ok) throw new Error("preview failed");
    return (await response.json()) as Preview;
  });
};
