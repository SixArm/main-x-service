// Client-side rules for the duplicate review queue.
//
// Mirrors `place-service-with-loco/src/db/review_queue.rs` (the
// decidable-status guard) and `agents/matching.md`'s `MatchBreakdown`
// (the per-component score object place-matcher computes), so the board
// can explain a pair's score and disable an impossible decision before
// the service answers 422. The server stays authoritative — this is a
// pre-flight courtesy.
//
// Pure and dependency-free (no Svelte, no fetch) so the mapping is
// unit-testable exactly as the Rust side's is.
//
// KNOWN GAP (documented, not a bug): unlike person-service's
// `ReviewQueueItem`, place-service's wire type carries neither a
// `provenance` field (the `review_queue` table has no such column) nor a
// serialized `score_breakdown` (the column exists — see
// `src/db/review_queue.rs` `ReviewQueueRow::score_breakdown` — but the
// batch-scan handler always inserts `None` and the wire `ReviewQueueItem`
// struct in `src/api/rest/handlers.rs` never exposes the column at all).
// `breakdownRows` below is therefore always called with `undefined`
// today and always renders the empty state; it is kept fully generic and
// unit-tested so the UI activates automatically the day the service
// starts populating and serializing that column, with no front-end
// change required. See place-service's own `spec/13-tasks.md` for the
// tracked follow-up.

import type { StringKey } from "$lib/i18n.svelte";
import type { ReviewQueueItem, ReviewStatus } from "$lib/api/types.js";

/**
 * The four stored dispositions, in the order the board's columns and the
 * status filter present them. These are the wire tokens verbatim — the
 * service serializes `ReviewStatus` lowercase and rejects anything else
 * with `422 INVALID_STATUS`.
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
 * `WHERE id = $1 AND status = 'pending'` and answers `422
 * INVALID_REVIEW_TRANSITION` otherwise (first writer wins). Checking here
 * lets the UI disable the buttons rather than offer a guaranteed failure.
 */
export function canDecide(item: Pick<ReviewQueueItem, "status">): boolean {
  return item.status === "pending";
}

/**
 * One component of the matcher's weighted score: its wire key, the
 * catalog key naming it, and its weight in the overall score.
 *
 * Weights are the place matcher's default weights, per
 * `place-service-with-loco/agents/matching.md`; they sum to 1.00.
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
 * The five components `MatchBreakdown` carries, in descending weight
 * order — so the rows a reader scans first are the ones that actually
 * moved the score.
 */
export const MATCH_COMPONENTS: readonly MatchComponent[] = [
  { key: "name_score", labelKey: "review.component.name", weight: 0.35 },
  { key: "geo_score", labelKey: "review.component.geo", weight: 0.25 },
  {
    key: "address_score",
    labelKey: "review.component.address",
    weight: 0.2,
  },
  {
    key: "place_type_score",
    labelKey: "review.component.placeType",
    weight: 0.1,
  },
  {
    key: "identifier_score",
    labelKey: "review.component.identifier",
    weight: 0.1,
  },
];

/** One rendered row of the score-breakdown table. */
export interface BreakdownRow extends MatchComponent {
  /** The component's score in `[0, 1]`. */
  score: number;
}

/**
 * Map a review item's `score_breakdown` to display rows.
 *
 * The service's stored column is `Option<serde_json::Value>` (see the
 * module doc's KNOWN GAP note: it is not currently on the wire at all,
 * so callers pass `undefined` today). Only the five known components are
 * surfaced, and only when their value is a finite number — an unknown
 * key is ignored rather than rendered as a mystery row, and a missing
 * one is omitted rather than shown as zero (which would read as "we
 * compared this and it did not match" when the truth is "this was not
 * compared").
 *
 * @param value - The raw `score_breakdown` off the wire, if ever present.
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
 * @param mainId - The record that should survive.
 * @param duplicateId - The record to be merged away (soft-deleted).
 */
export function mergeHref(mainId: string, duplicateId: string): string {
  const params = new URLSearchParams({
    main: mainId,
    duplicate: duplicateId,
  });
  return `/places/merge?${params.toString()}`;
}
