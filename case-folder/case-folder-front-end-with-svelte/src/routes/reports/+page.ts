import { api } from '$lib/api/client';
import { error } from '@sveltejs/kit';

export async function load({ fetch }) {
    try {
        const [stats, places, folders, moves, volumes] = await Promise.all([
            api.stats({ fetch }),
            api.places.list({}, { fetch }),
            api.folders.list({}, { fetch }),
            api.moves.list({}, { fetch }),
            api.volumes.list({}, { fetch })
        ]);
        return {
            stats,
            cabinets: places.cabinets,
            folders: folders.items,
            moves: moves.items,
            volumes: volumes.items
        };
    } catch (e) {
        error(503, (e as Error).message);
    }
}
