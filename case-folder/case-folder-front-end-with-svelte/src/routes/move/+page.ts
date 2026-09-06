// Move folder (`/move`) load function.
//
// Hydrates the cache with cabinets (the move destination picker) and
// workers (the "moved by" picker) in parallel. The folder being moved is
// looked up live as the user types an NHS Number on the page. 503 on failure.

import { api } from '$lib/api/client';
import { cache } from '$lib/store/cache.svelte';
import { error } from '@sveltejs/kit';

export async function load({ fetch }) {
    try {
        const [places, workers] = await Promise.all([
            api.places.list({ kind: 'cabinet' }, { fetch }),
            api.workers.list({}, { fetch }),
        ]);
        cache.setCabinets(places.cabinets);
        cache.setWorkers(workers.items);
        return {};
    } catch (e) {
        error(503, (e as Error).message);
    }
}
