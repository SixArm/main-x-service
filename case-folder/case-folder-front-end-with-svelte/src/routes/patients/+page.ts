// Patients index (`/patients`) load function.
//
// Hydrates the cache with the patient roster for the list view. 503 on failure.

import { api } from '$lib/api/client';
import { cache } from '$lib/store/cache.svelte';
import { error } from '@sveltejs/kit';

export async function load({ fetch }) {
    try {
        const list = await api.patients.list({}, { fetch });
        cache.setPatients(list.items);
        // `page.data.title` convention (see `../+layout.svelte`).
        return { title: 'Patients · Case Tracking' };
    } catch (e) {
        error(503, (e as Error).message);
    }
}
