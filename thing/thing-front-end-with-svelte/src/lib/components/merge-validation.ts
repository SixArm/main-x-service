// Pure guard for the merge page, extracted from routes/things/merge so it
// can be unit-tested without mounting the component. Implements the FR-9
// pre-merge checks: both ids required and they must differ (you cannot merge
// a record into itself).

/**
 * Validate a merge request's ids before issuing the POST (FR-9).
 *
 * @param mainId - The surviving record id.
 * @param duplicateId - The record to fold in and soft-delete.
 * @returns An error message to surface, or `null` when the pair is valid.
 */
export function validateMerge(mainId: string, duplicateId: string): string | null {
    if (!mainId || !duplicateId) return "Both IDs required";
    if (mainId === duplicateId) return "Main and duplicate must differ";
    return null;
}
