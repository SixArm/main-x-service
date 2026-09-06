// Formatters. The client formats; it does not compute.
//
// Every function here turns a value the API already decided into
// something readable — mirroring the content-management-system
// front-end's `$lib/format.ts` (WPM-T39). The recurring rule across all
// of them: a value that is genuinely `0` and a value that is *absent*
// are different claims, and only one of them is true — "no data" must
// never render as "0%" or "0.0", and a real zero must never collapse
// into "no data" either.

import type { Ratio } from "./api/types";

/**
 * A ratio as a rounded percentage, or `null` when there is nothing to
 * show.
 *
 * A zero-denominator ratio arrives as `value: null` and must **not**
 * render as `0%`: "we measured and it was zero" and "we had nothing to
 * measure" are different claims.
 */
export function percent(ratio: Ratio | null | undefined): string | null {
  if (!ratio || ratio.value === null) return null;
  return `${Math.round(ratio.value * 100)}%`;
}

/** A ratio's working, for display alongside the percentage: `"2/3"`. */
export function workings(ratio: Ratio | null | undefined): string | null {
  if (!ratio) return null;
  return `${ratio.numerator}/${ratio.denominator}`;
}

/**
 * A ratio rendered as `"67% (2/3)"`, or `fallback` (default `"—"`) when
 * there is nothing to show. This is the combined form every route
 * actually renders; `percent`/`workings` stay available separately for
 * a caller that lays the two out as distinct table columns.
 */
export function percentWithWorkings(
  ratio: Ratio | null | undefined,
  fallback = "—",
): string {
  const pct = percent(ratio);
  if (pct === null) return fallback;
  const w = workings(ratio);
  return w ? `${pct} (${w})` : pct;
}

/**
 * A percentage computed from a raw `done`/`total` pair, for a caller
 * that has counts rather than a service-supplied {@link Ratio} (e.g.
 * learning-path step progress). Same null-not-zero rule: `total === 0`
 * renders `fallback`, never `0%`.
 */
export function percentOf(done: number, total: number, fallback = "—"): string {
  if (total === 0) return fallback;
  return `${Math.round((done / total) * 100)}%`;
}

/**
 * A mean to one decimal place, or `null` when absent (`null`/
 * `undefined`) — never let a missing sample silently render as `"0.0"`
 * or, worse, the literal string `"undefined"`.
 */
export function mean(value: number | null | undefined): string | null {
  if (value === null || value === undefined) return null;
  return value.toFixed(1);
}
