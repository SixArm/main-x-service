// Client-side rules for the duplicate review queue.
//
// Mirrors `organization-service-with-loco/src/controllers/organizations.rs`'s
// `ReviewStatus` (the decidable-status guard) and
// `organization_matcher::MatchBreakdown` (the per-component score object;
// weights from `organization-matcher-rust-crate/src/config.rs`'s
// `MatchConfig::default`), so the review board can explain a pair's score
// and disable an impossible decision before the service answers 422. The
// server stays authoritative — this is a pre-flight courtesy.
//
// One deliberate difference from the person front-end's `review.ts`: the
// organization service's wire `ReviewQueueItem`
// (`src/controllers/organizations.rs::review_row_to_item`) never carries a
// `score_breakdown` — the `review_queue` table has the column, but the
// controller's response struct omits it, so the stored scan never reaches
// the browser. `breakdownRows` below takes whatever breakdown object the
// caller hands it rather than reading one off the item; the review page
// supplies a *live* breakdown by calling `POST /api/organizations/match`
// against the loaded pair, since that endpoint's `MatchResult` does carry
// one.
//
// Pure and dependency-free (no Svelte, no fetch) so the mapping is
// unit-testable exactly as the Rust side's is.

import type { StringKey } from "$lib/i18n.svelte";
import type { ReviewQueueItem, ReviewStatus } from "$lib/api/types.js";

/**
 * The four stored dispositions, in the order the board's columns and the
 * status filter present them. These are the wire tokens verbatim — the
 * service serializes `ReviewStatus` lowercase and rejects anything else
 * with `422 unprocessable_entity`.
 */
export const REVIEW_STATUSES = [
  "pending",
  "confirmed",
  "rejected",
  "automerged",
] as const satisfies readonly ReviewStatus[];

/**
 * Page sizes offered for the list call. The endpoint takes only `limit`
 * (default 100) and caps it at 500 server-side, so 500 is the largest
 * value worth offering — and there is no `offset`, so this is the whole
 * of the pagination story.
 */
export const REVIEW_LIMITS = [25, 50, 100, 250, 500] as const;

/** Whether `value` is one of the four stored dispositions. */
export function isReviewStatus(value: string): value is ReviewStatus {
  return (REVIEW_STATUSES as readonly string[]).includes(value);
}

/**
 * Whether this item can still be decided.
 *
 * Only `pending` items are decidable: the service's update is guarded by
 * `WHERE id = $1 AND status = 'pending'` and answers `422` otherwise
 * (`AlreadyDecided`, first writer wins). Checking here lets the UI
 * disable the buttons rather than offer a guaranteed failure.
 */
export function canDecide(item: Pick<ReviewQueueItem, "status">): boolean {
  return item.status === "pending";
}

/**
 * One component of the matcher's weighted score: its wire key, the
 * catalog key naming it, and its weight in the overall score.
 *
 * Weights are the organization matcher's defaults, per
 * `organization-matcher-rust-crate/src/config.rs::MatchConfig::default`;
 * they sum to 1.00.
 */
export interface MatchComponent {
  /** Field name in the service's `MatchBreakdown`. */
  key: string;
  /** i18n key for the human label. */
  labelKey: StringKey;
  /** Contribution to the overall score, in `[0, 1]`. */
  weight: number;
}

/**
 * The six components `MatchBreakdown` carries, in descending weight
 * order — so the rows a reader scans first are the ones that actually
 * moved the score.
 */
export const MATCH_COMPONENTS: readonly MatchComponent[] = [
  { key: "name_score", labelKey: "review.component.name", weight: 0.35 },
  {
    key: "address_score",
    labelKey: "review.component.address",
    weight: 0.2,
  },
  { key: "url_score", labelKey: "review.component.url", weight: 0.15 },
  {
    key: "jurisdiction_score",
    labelKey: "review.component.jurisdiction",
    weight: 0.1,
  },
  {
    key: "founding_date_score",
    labelKey: "review.component.foundingDate",
    weight: 0.1,
  },
  {
    key: "keywords_score",
    labelKey: "review.component.keywords",
    weight: 0.1,
  },
];

/** One rendered row of the score-breakdown table. */
export interface BreakdownRow extends MatchComponent {
  /** The component's score in `[0, 1]`. */
  score: number;
}

/**
 * Map a `MatchBreakdown`-shaped value to display rows.
 *
 * The value may be a real `MatchBreakdown` (from a live
 * `POST /api/organizations/match` call), `null`/`undefined` (no pair
 * loaded, or the match call has not returned yet), or — defensively —
 * something else entirely. Only the six known components are surfaced,
 * and only when their value is a finite number: an unknown key is
 * ignored rather than rendered as a mystery row, and an absent one
 * (`None` server-side — the two records had nothing to compare on that
 * field) is omitted rather than shown as zero, which would read as "we
 * compared this and it did not match" when the truth is "this was not
 * compared".
 *
 * @param value - A `MatchBreakdown`-shaped object, or anything else.
 * @returns Rows in {@link MATCH_COMPONENTS} order; empty when there is
 *   nothing to show, which the caller renders as an explicit note.
 */
export function breakdownRows(value: unknown): BreakdownRow[] {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return [];
  }
  const source = value as Record<string, unknown>;
  const rows: BreakdownRow[] = [];
  for (const component of MATCH_COMPONENTS) {
    const raw = source[component.key];
    if (typeof raw === "number" && Number.isFinite(raw)) {
      rows.push({ ...component, score: raw });
    }
  }
  return rows;
}

/**
 * Deep link to the merge page with both ids pre-filled.
 *
 * A review item names an unordered *pair* — the service records no
 * survivor — so the caller chooses which side survives, and the merge
 * page keeps both ids editable.
 *
 * @param mainPid - The record that should survive.
 * @param duplicatePid - The record to be merged away (soft-deleted).
 */
export function mergeHref(mainPid: string, duplicatePid: string): string {
  const params = new URLSearchParams({
    main: mainPid,
    duplicate: duplicatePid,
  });
  return `/merge?${params.toString()}`;
}
