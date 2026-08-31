// Patient detail (`/patients/[nhs]`) load function.
//
// Keyed by NHS Number (not an internal id). Returns the patient record
// (may be null if only known via folders), their folders, move history,
// and whether the central Patient Service matched. 503 on failure.

import { api } from '$lib/api/client';
import { formatNhsNumber } from '$lib/store/nhs';
import { error } from '@sveltejs/kit';

export async function load({ fetch, params }) {
    try {
        const result = await api.patients.show(params.nhs, { fetch });
        // `page.data.title` convention (see `../../+layout.svelte`). Falls
        // back to the NHS Number when the patient has no name on file yet
        // (a folder-only record with no central Patient Service match).
        const title = result.patient
            ? `${result.patient.name} · Case Tracking`
            : `${formatNhsNumber(params.nhs)} · Case Tracking`;
        return { ...result, title };
    } catch (e) {
        error(503, (e as Error).message);
    }
}
