import { api, ApiError } from '$lib/api/client';
import { error } from '@sveltejs/kit';

export async function load({ fetch, params }) {
    try {
        const move = await api.moves.show(params.id, { fetch });
        // The patient's other folders, so the user can jump across them.
        const siblings = await api.folders.list({ nhsNumber: move.nhsNumber }, { fetch });
        return { move, folders: siblings.items };
    } catch (e) {
        if (e instanceof ApiError && e.status === 404) error(404, 'Move event not found');
        error(503, (e as Error).message);
    }
}
