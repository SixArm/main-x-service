// New volume (`/volumes/new`) load function.
//
// Hydrates cabinets so the create form can pick the volume's initial
// location. 503 on failure.

import { api } from '$lib/api/client';
import { cache } from '$lib/store/cache.svelte';
import { error } from '@sveltejs/kit';

export async function load({ fetch }) {
    try {
        const places = await api.places.list({ kind: 'cabinet' }, { fetch });
        cache.setCabinets(places.cabinets);
        // `page.data.title` convention (see `../../+layout.svelte`).
        return { title: 'New volume · Case Tracking' };
    } catch (e) {
        error(503, (e as Error).message);
    }
}
