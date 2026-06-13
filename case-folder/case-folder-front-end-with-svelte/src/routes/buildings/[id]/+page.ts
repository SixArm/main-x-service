import { api, ApiError } from '$lib/api/client';
import { cache } from '$lib/store/cache.svelte';
import { error } from '@sveltejs/kit';

export async function load({ fetch, params }) {
    try {
        // Load the building itself, plus every place so we can list
        // its child rooms and the cabinets under each room.
        const [show, places, history] = await Promise.all([
            api.places.show(params.id, { fetch }),
            api.places.list({}, { fetch }),
            api.places.history(params.id, { fetch })
        ]);
        cache.setRooms(places.rooms);
        cache.setCabinets(places.cabinets);
        return {
            building: show.place,
            rooms: places.rooms.filter((r) => r.buildingId === params.id),
            presences: history.presences
        };
    } catch (e) {
        if (e instanceof ApiError && e.status === 404) {
            error(404, 'Building not found');
        }
        error(503, (e as Error).message);
    }
}
