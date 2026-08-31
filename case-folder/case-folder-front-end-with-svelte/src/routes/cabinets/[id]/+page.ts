// Cabinet detail (`/cabinets/[id]`) load function.
//
// Loads one cabinet with its current folders and its in/out presence
// timeline. Returned as page data (not cached). 404 if unknown, else 503.

import { api, ApiError } from '$lib/api/client';
import { error } from '@sveltejs/kit';

export async function load({ fetch, params }) {
    try {
        const [show, history] = await Promise.all([
            api.places.show(params.id, { fetch }),
            api.places.history(params.id, { fetch })
        ]);
        // `page.data.title` convention (see `../../+layout.svelte`).
        return {
            place: show.place,
            folders: show.folders,
            presences: history.presences,
            title: `${show.place.name} · Case Tracking`
        };
    } catch (e) {
        if (e instanceof ApiError && e.status === 404) error(404, 'Cabinet not found');
        error(503, (e as Error).message);
    }
}
