// Patient detail (`/patients/[nhs]`) load function.
//
// Keyed by NHS Number (not an internal id). Returns the patient record
// (may be null if only known via folders), their folders, move history,
// and whether the central Patient Service matched. 503 on failure.

import { api } from '$lib/api/client';
import { error } from '@sveltejs/kit';

export async function load({ fetch, params }) {
    try {
        return await api.patients.show(params.nhs, { fetch });
    } catch (e) {
        error(503, (e as Error).message);
    }
}
