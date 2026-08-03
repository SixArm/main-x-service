// Pure guard for the merge page, extracted from routes/merge so it can be
// unit-tested without mounting the component. Implements the pre-merge
// checks the service also enforces: both pids required, and they must
// differ (the service answers a self-merge with 422 — catching it here
// saves a round trip and gives a localized message).
//
// Unlike the sibling front-ends' copies, this returns an i18n **key**
// rather than an English sentence: this app renders every string through
// `t()` across 13 locales, so a hard-coded English string here would be
// the one untranslated message on the page.

import type { StringKey } from "$lib/i18n.svelte";

/**
 * Validate a merge request's pids before issuing the POST.
 *
 * @param mainPid - The surviving organization's pid.
 * @param duplicatePid - The organization to fold in and soft-delete.
 * @returns The i18n key of the error to surface, or `null` when the pair
 *   is valid.
 */
export function validateMerge(
  mainPid: string,
  duplicatePid: string,
): StringKey | null {
  if (!mainPid || !duplicatePid) return "merge.bothIdsRequired";
  if (mainPid === duplicatePid) return "merge.mustDiffer";
  return null;
}
