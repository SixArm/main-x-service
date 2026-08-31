// Volumes index (`/volumes`) load function.
//
// Fetches all volumes (movable bundles of a patient's folders) and
// returns them as page data for the list view. 503 on failure.

import { api } from '$lib/api/client';
import { error } from '@sveltejs/kit';

export async function load({ fetch }) {
    try {
        const list = await api.volumes.list({}, { fetch });
        // `page.data.title` convention (see `../+layout.svelte`).
        return { volumes: list.items, title: 'Volumes · Case Tracking' };
    } catch (e) {
        error(503, (e as Error).message);
    }
}
