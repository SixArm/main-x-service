import { api } from '$lib/api/client';
import { cache } from '$lib/store/cache.svelte';
import { error } from '@sveltejs/kit';

export async function load({ fetch }) {
    try {
        const [places, workers] = await Promise.all([
            api.places.list({ kind: 'cabinet' }, { fetch }),
            api.workers.list({}, { fetch })
        ]);
        cache.setCabinets(places.cabinets);
        cache.setWorkers(workers.items);
        return {};
    } catch (e) {
        error(503, (e as Error).message);
    }
}
