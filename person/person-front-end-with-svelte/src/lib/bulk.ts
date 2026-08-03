// Client-side rules for the bulk import/export surface.
//
// Mirrors `person-service-with-loco/src/bulk/handlers.rs` (the REST layer
// of `agents/share/bulk-import-export.md` §4), so the operator learns each
// constraint from the form rather than from a 400. The server stays
// authoritative — this is a pre-flight courtesy, never a substitute for
// its answer.
//
// Pure and dependency-free (no Svelte, no fetch) so the polling
// terminal-state rule and the dry-run wire encoding are unit-testable
// exactly as the Rust side's are.

/** File formats the service recognises (`BulkFormat::parse`). */
export const BULK_FORMATS = ["jsonl", "csv", "parquet"] as const;
/** One of {@link BULK_FORMATS}. */
export type BulkFormat = (typeof BULK_FORMATS)[number];

/**
 * Formats accepted for **import**. Parquet is export-only — the service's
 * `parse_import_format` rejects it with `400 UNSUPPORTED_FORMAT` before a
 * job is ever created — so it is not offered in the import picker.
 */
export const BULK_IMPORT_FORMATS = ["jsonl", "csv"] as const;
/** One of {@link BULK_IMPORT_FORMATS}. */
export type BulkImportFormat = (typeof BULK_IMPORT_FORMATS)[number];

/** Export masking profiles (`MaskingProfile::parse`). */
export const MASKING_PROFILES = ["masked", "full"] as const;
/** One of {@link MASKING_PROFILES}. `full` requires elevated authorisation. */
export type MaskingProfile = (typeof MASKING_PROFILES)[number];

/** Lifecycle states a `bulk_jobs` row moves through. */
export const BULK_JOB_STATUSES = [
  "queued",
  "running",
  "completed",
  "completed_with_errors",
  "failed",
] as const;
/** One of {@link BULK_JOB_STATUSES}. */
export type BulkJobStatus = (typeof BULK_JOB_STATUSES)[number];

/**
 * States after which the job will never change again, so polling stops.
 *
 * `completed_with_errors` is terminal *and* successful-ish: valid rows
 * committed and the failures are listed in the error report (§7). Treating
 * it as non-terminal would poll forever.
 */
export const TERMINAL_BULK_JOB_STATUSES = [
  "completed",
  "completed_with_errors",
  "failed",
] as const;

/**
 * Whether a job has reached a state it will never leave.
 *
 * Accepts a bare `string` because `status` arrives off the wire: an
 * unrecognised value from a newer service must not be treated as terminal,
 * or the UI would stop polling a job that is still running.
 *
 * @param status - The job's `status` field as returned by the service.
 * @returns `true` when polling should stop.
 */
export function isTerminalStatus(status: string): boolean {
  return (TERMINAL_BULK_JOB_STATUSES as readonly string[]).includes(status);
}

/**
 * Encode the dry-run checkbox as the multipart field value the service
 * accepts.
 *
 * The Rust handler treats a `dry_run` field as true only for the exact
 * trimmed tokens `1` / `true` / `yes` / `on`; anything else (including a
 * missing field) is false. Emitting `"false"` for the unchecked case is
 * therefore correct *and* explicit — the server reads it as false, and the
 * request shows the operator's choice rather than an absence.
 *
 * @param dryRun - Whether the operator asked for a preview.
 * @returns The literal string to put in the `dry_run` form field.
 */
export function dryRunFormValue(dryRun: boolean): string {
  return dryRun ? "true" : "false";
}

/**
 * Percentage of rows processed, or `null` when the total is not yet known
 * (the worker counts rows only after it has read the input, so an early
 * poll legitimately has `rows_total: null`).
 *
 * @param processed - Rows processed so far.
 * @param total - Total rows, once counted.
 * @returns An integer 0–100, or `null` when indeterminate.
 */
export function progressPercent(
  processed: number,
  total: number | null,
): number | null {
  // A zero total would divide by zero; a negative one is nonsense. Both
  // mean "no meaningful proportion to show" rather than 0%.
  if (total === null || total <= 0) return null;
  const pct = Math.round((processed / total) * 100);
  // Clamp: a worker that over-counts must not render a 140%-wide bar.
  return Math.min(100, Math.max(0, pct));
}

/** How often (ms) to re-poll a non-terminal job's status. */
export const POLL_INTERVAL_MS = 1500;
