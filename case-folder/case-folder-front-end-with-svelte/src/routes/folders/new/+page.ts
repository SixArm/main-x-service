// New folder (`/folders/new`) load function.
//
// Hydrates only cabinets (filtered server-side via `kind`) so the create
// form can offer an initial-cabinet picker. 503 on failure.

import { api } from '$lib/api/client';
import { cache } from '$lib/store/cache.svelte';
import { error } from '@sveltejs/kit';

export async function load({ fetch }) {
    try {
        const places = await api.places.list({ kind: 'cabinet' }, { fetch });
        cache.setCabinets(places.cabinets);
        return {};
    } catch (e) {
        error(503, (e as Error).message);
    }
}
