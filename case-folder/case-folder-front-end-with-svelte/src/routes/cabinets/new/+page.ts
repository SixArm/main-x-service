// New cabinet (`/cabinets/new`) load function.
//
// Hydrates buildings and rooms so the create form can offer a room
// picker (grouped by building) for the cabinet's parent place. 503 on failure.

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
