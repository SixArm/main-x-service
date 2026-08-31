// Workers index (`/workers`) load function.
//
// Fetches the worker roster (mirrored from the central Worker Service)
// and returns it as page data for the list view. 503 on failure.

import { api } from '$lib/api/client';
import { error } from '@sveltejs/kit';

export async function load({ fetch }) {
    try {
        const list = await api.workers.list({}, { fetch });
        // `page.data.title` convention (see `../+layout.svelte`).
        return { workers: list.items, title: 'Workers · Case Tracking' };
    } catch (e) {
        error(503, (e as Error).message);
    }
}
