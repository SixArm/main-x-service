// Pure guard for the merge page, extracted from `routes/merge` so it can
// be unit-tested without mounting the component. Implements the pre-merge
// checks the service also enforces: both ids required, and they must
// differ (the service answers 422 on a self-merge — catching it here saves
// the round trip and states the reason in the operator's own language).
//
// It returns an i18n **key** rather than an English sentence so the page
// renders the message in the selected locale; the caller passes it to
// `translate` / `t`.

import type { StringKey } from "$lib/i18n.svelte";

/**
 * Validate a merge request's ids before issuing the POST.
 *
 * @param mainId - The surviving case's persistent id.
 * @param duplicateId - The case to fold in and soft-delete.
 * @returns An i18n key for the message to surface, or `null` when the
 *   pair is valid.
 */
export function validateMerge(
  mainId: string,
  duplicateId: string,
): StringKey | null {
  if (!mainId || !duplicateId) return "merge.bothIdsRequired";
  if (mainId === duplicateId) return "merge.mustDiffer";
  return null;
}
