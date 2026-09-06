// Room detail (`/rooms/[id]`) load function.
//
// Loads one room and its aggregated presence history (across the room's
// cabinets). Returned as page data. 404 if unknown, else 503.

import { api, ApiError } from '$lib/api/client';
import { error } from '@sveltejs/kit';

export async function load({ fetch, params }) {
    try {
        const [show, history] = await Promise.all([
            api.places.show(params.id, { fetch }),
            api.places.history(params.id, { fetch }),
        ]);
        // `page.data.title` convention (see `../../+layout.svelte`).
        return {
            place: show.place,
            presences: history.presences,
            title: `${show.place.name} · Case Tracking`,
        };
    } catch (e) {
        if (e instanceof ApiError && e.status === 404)
            error(404, 'Room not found');
        error(503, (e as Error).message);
    }
}
