// Cabinets index (`/cabinets`) load function.
//
// Hydrates buildings, rooms, and cabinets so the page can show each
// cabinet with its resolved building/room location path. 503 on failure.

import { api } from '$lib/api/client';
import { cache } from '$lib/store/cache.svelte';
import { error } from '@sveltejs/kit';

export async function load({ fetch }) {
    try {
        const places = await api.places.list({}, { fetch });
        cache.setBuildings(places.buildings);
        cache.setRooms(places.rooms);
        cache.setCabinets(places.cabinets);
        return {};
    } catch (e) {
        error(503, (e as Error).message);
    }
}
