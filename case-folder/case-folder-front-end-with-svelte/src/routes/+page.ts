import { api } from '$lib/api/client';
import { cache } from '$lib/store/cache.svelte';
import { error } from '@sveltejs/kit';

export async function load({ fetch }) {
    try {
        const [stats, places, folders, recentMoves, patients] = await Promise.all([
            api.stats({ fetch }),
            api.places.list({}, { fetch }),
            api.folders.list({}, { fetch }),
            api.moves.list({}, { fetch }),
            api.patients.list({}, { fetch })
        ]);
        cache.setStats(stats);
        cache.setBuildings(places.buildings);
        cache.setRooms(places.rooms);
        cache.setCabinets(places.cabinets);
        cache.setFolders(folders.items);
        cache.setMoves(recentMoves.items);
        cache.setPatients(patients.items);
        return {};
    } catch (e) {
        error(503, (e as Error).message);
    }
}
