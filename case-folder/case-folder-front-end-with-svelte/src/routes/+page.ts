// Dashboard (`/`) load function.
//
// Fetches every projection the dashboard renders in one parallel batch
// and hydrates the rune-reactive cache so the page (and any sibling
// pages that read the same cache) render without further round-trips.
// Any failure is surfaced as a 503 so `+error.svelte` can show it,
// rather than leaving the dashboard half-populated.

import { api } from '$lib/api/client';
import { cache } from '$lib/store/cache.svelte';
import { error } from '@sveltejs/kit';

export async function load({ fetch }) {
    try {
        // Fire all five reads concurrently — they are independent, so the
        // page is gated only on the slowest one rather than their sum.
        const [stats, places, folders, recentMoves, patients] =
            await Promise.all([
                api.stats({ fetch }),
                api.places.list({}, { fetch }),
                api.folders.list({}, { fetch }),
                api.moves.list({}, { fetch }),
                api.patients.list({}, { fetch }),
            ]);
        // Hydrate the cache; subsequent renders read these reactively.
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
