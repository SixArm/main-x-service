// Buildings index (`/buildings`) load function.
//
// Hydrates the cache with buildings and rooms so the page can list each
// building with its room count. Cabinets aren't needed here. 503 on failure.

import { api } from '$lib/api/client';
import { cache } from '$lib/store/cache.svelte';
import { error } from '@sveltejs/kit';

export async function load({ fetch }) {
    try {
        const places = await api.places.list({}, { fetch });
        cache.setBuildings(places.buildings);
        cache.setRooms(places.rooms);
        return {};
    } catch (e) {
        error(503, (e as Error).message);
    }
}
