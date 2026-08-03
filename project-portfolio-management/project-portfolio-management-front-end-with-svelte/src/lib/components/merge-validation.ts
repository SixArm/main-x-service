// Pure guard for the merge page, extracted from `routes/plans/merge` so it
// can be unit-tested without mounting the component. Two pre-merge checks:
// both pids are required, and they must differ (the service itself answers
// `422` on a self-merge — catching it here saves the round trip and states
// the reason in the operator's own language).
//
// Returns an i18n KEY rather than a sentence, so the page renders the
// message through `t()` in the selected locale.

import type { StringKey } from "$lib/i18n.svelte";

/**
 * Validate a merge request's pids before issuing the POST.
 *
 * @param mainId - The surviving plan's pid.
 * @param duplicateId - The plan to fold in and soft-delete.
 * @returns An i18n key naming the problem, or `null` when the pair is valid.
 */
export function validateMerge(
  mainId: string,
  duplicateId: string,
): StringKey | null {
  if (!mainId || !duplicateId) return "merge.bothIdsRequired";
  if (mainId === duplicateId) return "merge.mustDiffer";
  return null;
}
