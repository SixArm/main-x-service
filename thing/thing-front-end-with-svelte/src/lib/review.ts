// Client-side rules for the duplicate review queue.
//
// Mirrors `thing-service-with-loco/src/db/review_queue.rs` (the
// decidable-status guard) and `src/matching/scoring.rs::MatchBreakdown`
// (the per-component score object + weights), so the board can explain a
// pair's score and disable an impossible decision before the service
// answers 422. The server stays authoritative — this is a pre-flight
// courtesy.
//
// KNOWN GAP (verified against `thing-service-with-loco`'s
// `src/api/rest/handlers.rs` wire `ReviewQueueItem` and
// `src/db/review_queue.rs`'s `COLS`, 2026-08-04): unlike person / worker /
// place / organization, this service's review-queue row and wire type
// carry no `provenance` column at all, and the wire `ReviewQueueItem`
// does not serialize `score_breakdown` even though the underlying DB row
// has that column (its one writer, the `deduplicate` handler, always
// writes it `None`). This module does not fabricate either field: the
// queue surfaces `detection_method` (the one "how found" field the
// service actually returns) in place of a provenance column, and
// {@link breakdownRows} / {@link breakdownFlags} simply return nothing
// for this service today — ready for the day the wire type carries the
// column, not pretending it already does. See
// `thing-service-with-loco/spec/13-tasks.md` for the tracked follow-up.
//
// Pure and dependency-free (no Svelte, no fetch) so the mapping is
// unit-testable exactly as the Rust side's is.

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
 * One weighted component of the matcher's score: its wire key, the
 * catalog key naming it, and its weight in the overall score.
 *
 * Weights are `MatchWeights::default()`
 * (`thing-service-with-loco/src/matching/scoring.rs`); they sum to 1.00.
 * Labels reuse the `results.*` keys already translated for
 * `MatchResultsList.svelte`'s own breakdown, rather than duplicating a
 * second set of component labels.
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
 * The five weighted components `MatchBreakdown` carries, in descending
 * weight order — so the rows a reader scans first are the ones that
 * actually moved the score.
 */
export const MATCH_COMPONENTS: readonly MatchComponent[] = [
  { key: "name_score", labelKey: "results.nameScore", weight: 0.4 },
  {
    key: "identifier_score",
    labelKey: "results.identifierScore",
    weight: 0.3,
  },
  {
    key: "description_score",
    labelKey: "results.descriptionScore",
    weight: 0.1,
  },
  { key: "url_score", labelKey: "results.urlScore", weight: 0.1 },
  { key: "same_as_score", labelKey: "results.sameAsScore", weight: 0.1 },
];

/** One rendered row of the score-breakdown table. */
export interface BreakdownRow extends MatchComponent {
  /** The component's score in `[0, 1]`. */
  score: number;
}

/**
 * Map a review item's `score_breakdown` to display rows.
 *
 * Defensive by construction: `value` may be an object, `null`/absent, or
 * (in principle) something else entirely. Only the five known weighted
 * components are surfaced, and only when their value is a finite number —
 * an unknown key is ignored rather than rendered as a mystery row, and a
 * missing one is omitted rather than shown as zero (which would read as
 * "we compared this and it did not match" when the truth is "this was not
 * compared"). Today the service never populates `score_breakdown` on the
 * wire (see the module doc), so this always returns `[]` for a live
 * queue — the caller renders the documented empty state, not a lie.
 *
 * @param value - The raw `score_breakdown` off the wire, if ever present.
 * @returns Rows in {@link MATCH_COMPONENTS} order; empty when there is
 *   nothing to show.
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

/** One boolean flag the matcher's breakdown carries alongside the score. */
export interface BreakdownFlag {
  /** Field name in the service's `MatchBreakdown`. */
  key: string;
  /** i18n key for the human label. */
  labelKey: StringKey;
}

/**
 * The two boolean flags `MatchBreakdown` carries alongside the five
 * weighted components — mirrors `MatchResultsList.svelte`'s own flag
 * rendering for the ad-hoc match/check-duplicates results.
 */
const BREAKDOWN_FLAGS: readonly BreakdownFlag[] = [
  { key: "phonetic_match", labelKey: "results.phoneticMatch" },
  { key: "deterministic_match", labelKey: "results.deterministicMatch" },
];

/**
 * Map a review item's `score_breakdown` to the flags that are actually
 * `true`. Same defensive parsing as {@link breakdownRows}, and the same
 * "always empty today" caveat applies.
 *
 * @param value - The raw `score_breakdown` off the wire, if ever present.
 * @returns The subset of {@link BREAKDOWN_FLAGS} that are `true` in `value`.
 */
export function breakdownFlags(value: unknown): BreakdownFlag[] {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return [];
  }
  const source = value as Record<string, unknown>;
  return BREAKDOWN_FLAGS.filter((flag) => source[flag.key] === true);
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
  return `/things/merge?${params.toString()}`;
}
