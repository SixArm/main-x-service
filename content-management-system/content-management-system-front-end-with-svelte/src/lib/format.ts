// Formatters. The client formats; it does not compute
// (`../spec/insights.md`).
//
// Every function here turns a value the API already decided into
// something readable. None of them derives a number: if the UI could
// calculate a rate, it could disagree with the service about it, and
// the service is the one with the data.

import type { Ratio, Staleness } from "./api/cms";

/**
 * A ratio as a percentage, or `null` when there is nothing to show.
 *
 * A zero denominator arrives as `value: null` and must **not** render
 * as `0%`: "we measured and it was zero" and "we had nothing to
 * measure" are different claims, and only one of them is true.
 */
export function percent(ratio: Ratio | null | undefined): string | null {
  if (!ratio || ratio.value === null || ratio.denominator === 0) return null;
  return `${Math.round(ratio.value * 100)}%`;
}

/** A ratio's working, for the tooltip: "13 of 15". */
export function workings(ratio: Ratio | null | undefined): string | null {
  if (!ratio) return null;
  return `${ratio.numerator} / ${ratio.denominator}`;
}

/** A duration in seconds as an approximate human span. */
export function duration(seconds: number | null | undefined): string | null {
  if (seconds === null || seconds === undefined) return null;
  const abs = Math.abs(seconds);
  if (abs < 90) return `${Math.round(seconds)}s`;
  if (abs < 5400) return `${Math.round(seconds / 60)}m`;
  if (abs < 172_800) return `${Math.round(seconds / 3600)}h`;
  return `${Math.round(seconds / 86_400)}d`;
}

/** Bytes as a readable size. */
export function bytes(value: number): string {
  const units = ["B", "kB", "MB", "GB", "TB"];
  let size = value;
  let unit = 0;
  while (size >= 1000 && unit < units.length - 1) {
    size /= 1000;
    unit += 1;
  }
  return `${size < 10 && unit > 0 ? size.toFixed(1) : Math.round(size)} ${units[unit]}`;
}

/**
 * Staleness in words.
 *
 * Three outcomes, not two. "Up to date", "N revisions behind", and
 * **"unknown"** — the service reports the last of these when a variant
 * records no source revision, and collapsing it into "up to date"
 * would tell an editor their translation is fine when nobody knows.
 */
export function staleness(value: Staleness | null | undefined): {
  tone: "ok" | "stale" | "unknown";
  text: string;
} {
  if (!value) return { tone: "unknown", text: "unknown" };
  if (value.unknown) return { tone: "unknown", text: value.unknown };
  if (!value.stale) return { tone: "ok", text: "up to date" };
  const n = value.revisions_behind;
  return {
    tone: "stale",
    text: `${n} source revision${n === 1 ? "" : "s"} behind`,
  };
}

/** A timestamp in the reader's locale, or an empty string. */
export function when(value: string | null | undefined): string {
  return value ? new Date(value).toLocaleString() : "";
}

/** A worker URN shortened for display, keeping it recognisable as a
 *  reference rather than a name we do not have. */
export function actor(ref: string | null | undefined): string {
  if (!ref) return "unattributed";
  const [type, id] = ref.split(":");
  return id ? `${type}:${id.slice(0, 8)}…` : ref;
}
